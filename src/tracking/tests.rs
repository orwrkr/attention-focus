use super::*;
use migrate::SCHEMA_VERSION;
use rusqlite::Connection;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use worker::command_moment;

/// How long a test waits for the worker. A passing test returns as soon as the event arrives; the
/// limit only stops a hang, and a busy CI runner once needed more than five seconds to open the store.
const WORKER_WAIT: Duration = Duration::from_secs(60);

mod history_correction {
    use super::*;
    use rusqlite::params;
    use serde_json::Value;

    fn edit(id: Option<i64>, start: i64, end: i64) -> IntervalEdit {
        IntervalEdit {
            id,
            recorded: None,
            start_ms: start,
            end_ms: end,
            activity: Activity::default(),
            timezone: "UTC".into(),
        }
    }
    fn audit(store: &Store) -> Vec<(String, i64, Option<Value>, Option<Value>)> {
        store
            .connection
            .prepare("SELECT action,edited_ms,before,after FROM focus_session_edits ORDER BY id")
            .unwrap()
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap()
            .map(|r| {
                let (a, ms, before, after) = r.unwrap();
                (
                    a,
                    ms,
                    before.map(|s| serde_json::from_str(&s).unwrap()),
                    after.map(|s| serde_json::from_str(&s).unwrap()),
                )
            })
            .collect()
    }
    fn history(store: &Store) -> Vec<String> {
        stored_text(
            &store.connection,
            "SELECT json_object('id',id,'start',started_ms,'end',ended_ms,'checkpoint',checkpoint_ms,'elapsed',elapsed_ms,'state',state,'activity',activity,'reason',end_reason) FROM focus_sessions ORDER BY id",
        )
    }
    fn insert(store: &Store, start: i64, end: Option<i64>, state: &str) -> i64 {
        store.connection.execute("INSERT INTO focus_sessions(started_ms,ended_ms,checkpoint_ms,elapsed_ms,state,activity,end_reason) VALUES(?1,?2,COALESCE(?2,?1),COALESCE(?2-?1,0),?3,'{\"project\":null,\"note\":\"\"}','stop')", params![start, end, state]).unwrap();
        store.connection.last_insert_rowid()
    }
    fn catalog_activity(store: &mut Store) -> Activity {
        store.add_projects(&["Example".into()]).unwrap();
        let id = store.catalog().unwrap()[0].id;
        store.add_tasks(id, &["Task".into()]).unwrap();
        Activity {
            project: Some(Project {
                id,
                title: "Example".into(),
                task: Some("Task".into()),
            }),
            note: "Note".into(),
        }
    }

    #[test]
    fn schema_four_upgrade_backs_up_every_row_once_and_is_atomic_on_failure() {
        for fail in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("focus.sqlite3");
            let store = Store::open(&path).unwrap();
            insert(&store, 100, Some(200), "stopped");
            insert(&store, 300, Some(400), "interrupted");
            let before = history(&store);
            store
                .connection
                .execute_batch("DROP TABLE focus_session_edits; PRAGMA user_version=4;")
                .unwrap();
            if fail {
                store
                    .connection
                    .execute_batch("CREATE TABLE focus_session_edits(conflict TEXT);")
                    .unwrap();
            }
            drop(store);
            if fail {
                assert!(Store::open(&path).is_err());
                let connection = Connection::open(&path).unwrap();
                assert_eq!(
                    connection
                        .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                        .unwrap(),
                    4
                );
                assert_eq!(history(&Store::wrap(connection)), before);
            } else {
                let store = Store::open(&path).unwrap();
                assert_eq!(history(&store), before);
                assert_eq!(
                    store
                        .connection
                        .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                        .unwrap(),
                    5
                );
                assert!(audit(&store).is_empty());
                drop(store);
                Store::open(&path).unwrap();
            }
            let backups = backup_files(dir.path(), "focus-before-v5-");
            assert_eq!(backups.len(), 1);
            let backup = Store::wrap(Connection::open(&backups[0]).unwrap());
            assert_eq!(history(&backup), before);
            assert_eq!(
                backup
                    .connection
                    .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                4
            );
        }
    }
    #[test]
    fn every_older_numbered_schema_reaches_five_and_fresh_history_needs_no_backup() {
        for version in 1..=3 {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("focus.sqlite3");
            legacy_database(
                &path,
                version,
                &[(
                    100,
                    Some(200),
                    100,
                    "stopped",
                    legacy_activity(1, "example", "Example", None),
                )],
            );
            let store = Store::open(&path).unwrap();
            assert_eq!(
                store
                    .connection
                    .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                5
            );
            assert_eq!(store.snapshot().unwrap().total_ms, 100);
            assert!(audit(&store).is_empty());
            assert_eq!(backup_files(dir.path(), "focus-before-").len(), 1);
        }
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        assert!(audit(&store).is_empty());
        assert!(backup_files(dir.path(), "focus-before-").is_empty());
    }
    #[test]
    fn manual_add_rejects_bad_order_future_and_every_positive_overlap_but_allows_touching() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        for (start, end, key) in [
            (200, 100, "errors.interval_order"),
            (100, 100, "errors.interval_order"),
            (100, 1001, "errors.interval_future"),
        ] {
            assert_eq!(
                store
                    .save_interval(&edit(None, start, end), 1000)
                    .unwrap_err()
                    .to_string(),
                crate::i18n::message(key, &[])
            );
        }
        let saved = insert(&store, 200, Some(300), "stopped");
        insert(&store, 400, Some(500), "interrupted");
        insert(&store, 600, None, "running");
        insert(&store, 150, Some(150), "stopped");
        let original = history(&store);
        for (start, end) in [(250, 350), (450, 550), (650, 750)] {
            assert!(
                store
                    .save_interval(&edit(None, start, end), 1000)
                    .unwrap_err()
                    .to_string()
                    .contains("overlaps")
            );
            assert_eq!(history(&store), original);
            assert!(audit(&store).is_empty());
        }
        let conflict = store
            .save_interval(&edit(None, 100, 900), 1000)
            .unwrap_err()
            .to_string();
        assert!(
            conflict.contains("00:00:00"),
            "seconds appear in the conflict"
        );
        store.save_interval(&edit(None, 100, 200), 1000).unwrap();
        store.save_interval(&edit(None, 300, 400), 1000).unwrap();
        assert_eq!(audit(&store).len(), 2);
        assert_eq!(saved, 1);
    }
    #[test]
    fn overlap_names_the_earliest_conflict_with_seconds_in_the_requested_zone() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        let start = instant("2026-10-27T12:00:11Z");
        insert(&store, start + 60000, Some(start + 90000), "stopped");
        insert(&store, start, Some(start + 30000), "stopped");
        let mut addition = edit(None, start - 1000, start + 100000);
        addition.timezone = "America/New_York".into();
        let error = store
            .save_interval(&addition, start + 200000)
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            crate::i18n::message(
                "errors.interval_overlap",
                &[
                    ("start", "2026-10-27 08:00:11 -04:00"),
                    ("end", "2026-10-27 08:00:41 -04:00")
                ]
            )
        );
        addition.timezone = "invalid-zone".into();
        assert!(
            store
                .save_interval(&addition, start + 200000)
                .unwrap_err()
                .to_string()
                .contains("overlaps"),
            "an unknown display zone cannot bypass overlap validation"
        );
        assert!(audit(&store).is_empty());
    }
    #[test]
    fn manual_add_refreshes_both_daily_totals_and_count_without_selection_or_recent_use() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        let chosen = catalog_activity(&mut engine.store);
        engine
            .apply(Some(Command::Start(chosen.clone())), at(0))
            .unwrap();
        engine.apply(Some(Command::Stop), at(100)).unwrap();
        let before = engine.snapshot(at(1000)).unwrap();
        let recent = stored_text(
            &engine.store.connection,
            "SELECT json_object('key',activity_key,'order',used_order) FROM focus_recent ORDER BY used_order",
        );
        let mut addition = edit(None, at(200).wall_ms, at(500).wall_ms);
        addition.activity = chosen;
        engine
            .apply(Some(Command::SaveInterval(addition)), at(1000))
            .unwrap();
        let after = engine.snapshot(at(1000)).unwrap();
        assert_eq!(
            (
                after.total_ms,
                after.completed,
                after.daily_project_ms,
                after.daily_all_ms
            ),
            (400, 2, Some(400), Some(400))
        );
        assert_eq!(after.activity, before.activity);
        assert_eq!(
            stored_text(
                &engine.store.connection,
                "SELECT json_object('key',activity_key,'order',used_order) FROM focus_recent ORDER BY used_order"
            ),
            recent
        );
        let edits = audit(&engine.store);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            (&edits[0].0, edits[0].1, edits[0].2.as_ref()),
            (&"create".into(), at(1000).wall_ms, None)
        );
        assert_eq!(edits[0].3.as_ref().unwrap()["end_reason"], "manual");
        drop(engine);
        let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        assert_eq!(store.snapshot().unwrap().total_ms, 400);
        assert_eq!(audit(&store).len(), 1);
        let mut engine = Engine::new(store, at(1000));
        engine
            .apply(Some(Command::Start(Activity::default())), at(1000))
            .unwrap();
        assert!(engine.snapshot(at(1000)).unwrap().running);
    }
    #[test]
    fn activity_only_edit_keeps_legacy_overlap_elapsed_and_exact_boundaries_and_audits_both_sides()
    {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        let id = insert(&store, 101, Some(299), "stopped");
        insert(&store, 150, Some(350), "stopped");
        store
            .connection
            .execute(
                "UPDATE focus_sessions SET elapsed_ms=170,checkpoint_ms=280 WHERE id=?1",
                [id],
            )
            .unwrap();
        let before = history(&store);
        let mut correction = edit(Some(id), 101, 299);
        correction.activity.note = "Corrected note".into();
        store.save_interval(&correction, 1000).unwrap();
        let row: Value = serde_json::from_str(&history(&store)[0]).unwrap();
        assert_eq!(
            (
                row["start"].as_i64(),
                row["end"].as_i64(),
                row["elapsed"].as_i64(),
                row["checkpoint"].as_i64()
            ),
            (Some(101), Some(299), Some(170), Some(280))
        );
        let edits = audit(&store);
        assert_eq!(edits[0].0, "update");
        assert_eq!(edits[0].2.as_ref().unwrap()["activity"]["note"], "");
        assert_eq!(
            edits[0].3.as_ref().unwrap()["activity"]["note"],
            "Corrected note"
        );
        store.save_interval(&correction, 1000).unwrap();
        assert_eq!(audit(&store).len(), 1, "a no-op has no audit");
        assert_ne!(history(&store), before);
        correction.end_ms = 300;
        assert!(
            store.save_interval(&correction, 1000).is_err(),
            "the worker compares boundaries with the stored row"
        );
    }
    #[test]
    fn boundary_edit_uses_wall_length_keeps_reason_and_preserves_unknown_snapshot_json_exactly() {
        for catalog_change in ["rename", "archive", "delete"] {
            let dir = tempfile::tempdir().unwrap();
            let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
            let activity = catalog_activity(&mut store);
            let project = activity.project.as_ref().unwrap().id;
            let id = insert(&store, 100, Some(200), "stopped");
            let text = format!("  {}  ", serde_json::to_string(&activity).unwrap());
            store.connection.execute("UPDATE focus_sessions SET activity=?1,elapsed_ms=90,end_reason='switch' WHERE id=?2",params![text,id]).unwrap();
            match catalog_change {
                "rename" => store.rename_project(project, "New name").unwrap(),
                "archive" => store.set_project_archived(project, true).unwrap(),
                _ => {
                    store
                        .connection
                        .execute("DELETE FROM focus_projects WHERE id=?1", [project])
                        .unwrap();
                }
            }
            let mut correction = edit(Some(id), 101, 202);
            correction.activity = activity;
            store.save_interval(&correction, 1000).unwrap();
            assert_eq!(
                stored_text(&store.connection, "SELECT activity FROM focus_sessions"),
                vec![text]
            );
            let row: Value = serde_json::from_str(&history(&store)[0]).unwrap();
            assert_eq!(row["elapsed"], 101);
            assert_eq!(row["checkpoint"], 202);
            assert_eq!(row["reason"], "switch");
        }
    }
    #[test]
    fn changed_activity_uses_catalog_title_and_accepts_archived_finished_and_unicode() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        let mut activity = catalog_activity(&mut store);
        let id = activity.project.as_ref().unwrap().id;
        // Synthetic Unicode project and note exercise user data without prose fixtures.
        let title = "Example \u{4e2d}\u{1f331}";
        store.rename_project(id, title).unwrap();
        store.set_project_archived(id, true).unwrap();
        store.set_task_done(id, "Task", true).unwrap();
        activity.note = "Note \u{6458}\u{8981} \u{1f331}".into();
        let mut addition = edit(None, 100, 200);
        addition.activity = activity.clone();
        store.save_interval(&addition, 1000).unwrap();
        let row = history(&store);
        let snapshot: Value = serde_json::from_str(&row[0]).unwrap();
        let saved: Activity = serde_json::from_str(snapshot["activity"].as_str().unwrap()).unwrap();
        assert_eq!(saved.project.unwrap().title, title);
        assert_eq!(saved.note, activity.note);
        assert_eq!(snapshot["reason"], "manual");
        assert_eq!(
            audit(&store)[0].3.as_ref().unwrap()["activity"]["note"],
            activity.note
        );
        for unknown_task in [false, true] {
            let mut bad = edit(Some(1), 100, 200);
            bad.activity = activity.clone();
            if unknown_task {
                bad.activity.project.as_mut().unwrap().task = Some("Missing".into());
            } else {
                bad.activity.project.as_mut().unwrap().id += 1;
            }
            assert_eq!(
                store.save_interval(&bad, 1000).unwrap_err().to_string(),
                crate::i18n::message("errors.interval_activity", &[])
            );
            assert_eq!(history(&store), row);
            assert_eq!(audit(&store).len(), 1);
        }
    }
    #[test]
    fn locked_or_missing_rows_refuse_save_and_delete_without_any_write() {
        for state in ["running", "interrupted", "missing"] {
            let dir = tempfile::tempdir().unwrap();
            let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
            let id = if state == "missing" {
                99
            } else {
                insert(&store, 100, Some(200), state)
            };
            let before = history(&store);
            let expected = crate::i18n::message(
                if state == "missing" {
                    "errors.interval_missing"
                } else {
                    "errors.interval_locked"
                },
                &[],
            );
            assert_eq!(
                store
                    .save_interval(&edit(Some(id), 100, 200), 1000)
                    .unwrap_err()
                    .to_string(),
                expected
            );
            assert_eq!(
                store
                    .delete_interval(id, (0, 0), 1000)
                    .unwrap_err()
                    .to_string(),
                expected
            );
            assert_eq!(history(&store), before);
            assert!(audit(&store).is_empty());
        }
    }
    #[test]
    fn deletion_refreshes_totals_and_releases_the_catalog_while_audit_keeps_the_old_row() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        let activity = catalog_activity(&mut engine.store);
        let project = activity.project.as_ref().unwrap().id;
        let mut addition = edit(None, at(100).wall_ms, at(500).wall_ms);
        addition.activity = activity;
        engine
            .apply(Some(Command::SaveInterval(addition)), at(1000))
            .unwrap();
        assert!(engine.store.delete_project(project).is_err());
        assert_eq!(engine.snapshot(at(1000)).unwrap().daily_all_ms, Some(400));
        engine
            .apply(
                Some(Command::DeleteInterval {
                    id: 1,
                    recorded: (at(100).wall_ms, at(500).wall_ms),
                }),
                at(1000),
            )
            .unwrap();
        let state = engine.snapshot(at(1000)).unwrap();
        assert_eq!(
            (
                state.total_ms,
                state.completed,
                state.daily_project_ms,
                state.daily_all_ms
            ),
            (0, 0, Some(0), Some(0))
        );
        assert!(history(&engine.store).is_empty());
        let edits = audit(&engine.store);
        assert_eq!(edits[1].0, "delete");
        assert_eq!(edits[1].2, edits[0].3);
        assert!(edits[1].3.is_none());
        engine.store.delete_project(project).unwrap();
    }
    #[test]
    fn a_reused_id_does_not_let_a_stale_view_change_another_interval() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        insert(&store, 100, Some(200), "stopped");
        store.delete_interval(1, (100, 200), 1000).unwrap();
        store.save_interval(&edit(None, 300, 400), 1000).unwrap();
        let before = history(&store);
        let edits = audit(&store).len();
        let mut stale = edit(Some(1), 110, 190);
        stale.recorded = Some((100, 200));
        for result in [
            store.save_interval(&stale, 1000),
            store.delete_interval(1, (100, 200), 1000),
        ] {
            assert_eq!(
                result.unwrap_err().to_string(),
                crate::i18n::message("errors.interval_missing", &[])
            );
        }
        assert_eq!(history(&store), before);
        assert_eq!(audit(&store).len(), edits);
    }
    #[test]
    fn a_note_change_keeps_the_recorded_title_and_needs_no_valid_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        let recorded = Activity {
            project: Some(crate::tracking::Project {
                id: 9,
                title: "Recorded title".into(),
                task: Some("Task that left the list".into()),
            }),
            note: String::new(),
        };
        store
            .connection
            .execute(
                "INSERT INTO focus_sessions(started_ms,checkpoint_ms,ended_ms,elapsed_ms,state,activity) VALUES(500,5000,5000,0,'stopped',?1)",
                [serde_json::to_string(&recorded).unwrap()],
            )
            .unwrap();
        let mut changed = edit(Some(1), 500, 5000);
        changed.activity = Activity {
            note: "Corrected note".into(),
            ..recorded.clone()
        };
        // The stored end lies after this clock reading, which must not block a note.
        store.save_interval(&changed, 1000).unwrap();
        let stored: Activity = serde_json::from_str(
            &stored_text(&store.connection, "SELECT activity FROM focus_sessions")[0],
        )
        .unwrap();
        assert_eq!(stored, changed.activity);
        // A different task must still come from the list.
        changed.activity.project.as_mut().unwrap().task = Some("Other".into());
        assert!(store.save_interval(&changed, 1000).is_err());
    }
    #[test]
    fn audit_failure_rolls_back_add_edit_and_delete() {
        for action in ["add", "edit", "delete"] {
            let dir = tempfile::tempdir().unwrap();
            let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
            insert(&store, 100, Some(200), "stopped");
            let before = history(&store);
            store
                .connection
                .execute_batch("ALTER TABLE focus_session_edits RENAME TO unavailable_audit;")
                .unwrap();
            let result = match action {
                "add" => store.save_interval(&edit(None, 300, 400), 1000),
                "edit" => store.save_interval(&edit(Some(1), 101, 201), 1000),
                _ => store.delete_interval(1, (100, 200), 1000),
            };
            assert!(result.is_err(), "{action}");
            assert_eq!(history(&store), before, "{action}");
            assert!(
                stored_text(&store.connection, "SELECT action FROM unavailable_audit").is_empty()
            );
        }
    }
    #[test]
    fn manual_midnight_is_one_row_and_two_chart_segments_and_keeps_running_state() {
        let dir = tempfile::tempdir().unwrap();
        let midnight = instant("2026-10-27T00:00:00Z");
        let clock = Clock {
            anchor_ms: midnight,
        };
        let mut engine = clocked_engine(dir.path(), clock, "UTC");
        engine
            .apply(
                Some(Command::Start(Activity::default())),
                clock.moment(midnight + 1000),
            )
            .unwrap();
        let started = engine.started.unwrap();
        engine
            .apply(
                Some(Command::SaveInterval(edit(
                    None,
                    midnight - 10000,
                    midnight,
                ))),
                clock.moment(midnight + 2000),
            )
            .unwrap();
        assert_eq!(engine.started.unwrap().wall_ms, started.wall_ms);
        assert_eq!(engine.started.unwrap().mono_ms, started.mono_ms);
        engine
            .apply(Some(Command::Stop), clock.moment(midnight + 2000))
            .unwrap();
        engine
            .apply(
                Some(Command::SaveInterval(edit(
                    Some(2),
                    midnight - 10000,
                    midnight + 500,
                ))),
                clock.moment(midnight + 3000),
            )
            .unwrap();
        let mut request = whole_day("2026-10-26", "UTC");
        request.through = "2026-10-27".into();
        let report = engine
            .store
            .report(&request, clock.moment(midnight + 3000), 0)
            .unwrap();
        assert_eq!(report.range.days[0].saved_ms, 10000);
        assert_eq!(report.range.days[1].saved_ms, 1500);
        let chart = report.chart.unwrap();
        let segments: Vec<_> = chart.segments.iter().filter(|s| s.session == 2).collect();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].elapsed_ms + segments[1].elapsed_ms, 10500);
        assert_eq!(chart.sessions[&2].start_ms, midnight - 10000);
    }
}
#[test]
fn all_project_today_combines_projects_and_unassigned_without_double_counting_live_time() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("Alpha"))), at(0))
        .unwrap();
    engine
        .apply(Some(Command::Switch(activity("Beta"))), at(10_000))
        .unwrap();
    engine
        .apply(Some(Command::Switch(Activity::default())), at(20_000))
        .unwrap();
    for _ in 0..2 {
        let state = engine
            .snapshot_in_zone(at(25_000), jiff::tz::TimeZone::UTC)
            .unwrap();
        assert_eq!(state.elapsed_ms, 5_000);
        assert_eq!(state.daily_project_ms, Some(5_000));
        assert_eq!(state.daily_all_ms, Some(25_000));
    }
    engine.apply(Some(Command::Stop), at(25_000)).unwrap();
    engine
        .apply(Some(Command::Select(activity("Alpha"))), at(25_000))
        .unwrap();
    let state = engine
        .snapshot_in_zone(at(25_000), jiff::tz::TimeZone::UTC)
        .unwrap();
    assert_eq!(state.daily_project_ms, Some(10_000));
    assert_eq!(state.daily_all_ms, Some(25_000));
    assert_eq!(state.elapsed_ms, 0);
    assert_eq!(
        engine
            .snapshot_in_zone(at(86_425_000), jiff::tz::TimeZone::UTC)
            .unwrap()
            .daily_all_ms,
        Some(0)
    );
}
#[test]
fn daily_totals_clip_midnight_combine_tasks_isolate_sources_and_exclude_review() {
    let dir = tempfile::tempdir().unwrap();
    let midnight = "2026-10-27T00:00:00Z"
        .parse::<jiff::Timestamp>()
        .unwrap()
        .as_millisecond();
    let moment = |ms: u64| Moment {
        wall_ms: midnight - 10_000 + ms as i64,
        mono_ms: ms,
    };
    let mut engine = Engine::new(
        Store::open(&dir.path().join("focus.sqlite3")).unwrap(),
        moment(0),
    );
    engine.zone = Some(jiff::tz::TimeZone::UTC);
    let first = activity("Example");
    engine
        .apply(Some(Command::Start(first.clone())), moment(0))
        .unwrap();
    assert_eq!(
        engine
            .snapshot_in_zone(moment(20_000), jiff::tz::TimeZone::UTC)
            .unwrap()
            .daily_project_ms,
        Some(10_000)
    );
    let mut task = first.clone();
    task.project.as_mut().unwrap().task = Some("Review".into());
    engine
        .apply(Some(Command::Switch(task)), moment(20_000))
        .unwrap();
    assert_eq!(
        engine
            .snapshot_in_zone(moment(25_000), jiff::tz::TimeZone::UTC)
            .unwrap()
            .daily_project_ms,
        Some(15_000)
    );
    engine.apply(Some(Command::Stop), moment(25_000)).unwrap();
    let saved = engine
        .snapshot_in_zone(moment(25_000), jiff::tz::TimeZone::UTC)
        .unwrap();
    assert_eq!(saved.daily_project_ms, Some(15_000));
    assert_eq!(saved.daily_all_ms, Some(15_000));
    assert_eq!(saved.total_ms, 25_000);
    let mut other = first.clone();
    other.project.as_mut().unwrap().id += 1;
    engine
        .apply(Some(Command::Select(other)), moment(26_000))
        .unwrap();
    assert_eq!(
        engine
            .snapshot_in_zone(moment(26_000), jiff::tz::TimeZone::UTC)
            .unwrap()
            .daily_project_ms,
        Some(0)
    );
    engine
        .apply(Some(Command::Select(first.clone())), moment(27_000))
        .unwrap();
    engine
        .apply(Some(Command::Start(first)), moment(27_000))
        .unwrap();
    engine.apply(None, moment(37_000)).unwrap();
    engine.apply(None, moment(80_000)).unwrap();
    let review = engine
        .snapshot_in_zone(moment(80_000), jiff::tz::TimeZone::UTC)
        .unwrap();
    assert!(review.needs_review);
    assert_eq!(review.daily_project_ms, Some(15_000));
    assert_eq!(review.daily_all_ms, Some(15_000));
    engine
        .apply(Some(Command::AcceptRecovery), moment(81_000))
        .unwrap();
    assert_eq!(
        engine
            .snapshot_in_zone(moment(81_000), jiff::tz::TimeZone::UTC)
            .unwrap()
            .daily_project_ms,
        Some(25_000)
    );
    assert_eq!(
        engine
            .snapshot_in_zone(moment(86_420_000), jiff::tz::TimeZone::UTC)
            .unwrap()
            .daily_project_ms,
        Some(0)
    );
    engine
        .store
        .connection
        .execute("UPDATE focus_sessions SET activity='invalid'", [])
        .unwrap();
    engine.daily_cache.borrow_mut().take();
    assert_eq!(
        engine
            .snapshot_in_zone(moment(81_000), jiff::tz::TimeZone::UTC)
            .unwrap()
            .daily_project_ms,
        None
    );
}
fn at(ms: u64) -> Moment {
    Moment {
        wall_ms: 1_000_000 + ms as i64,
        mono_ms: ms,
    }
}
fn activity(title: &str) -> Activity {
    Activity {
        project: Some(Project {
            // Stable per title, so tests can compare activities without a project list.
            id: title.bytes().fold(7_i64, |id, byte| {
                (id * 31 + i64::from(byte)) % 1_000_000_007
            }) + 1,
            title: title.into(),
            task: Some("Draft \u{1f331}".into()),
        }),
        note: "Example note".into(),
    }
}
fn engine(dir: &Path) -> Engine {
    let mut engine = Engine::new(Store::open(&dir.join("focus.sqlite3")).unwrap(), at(0));
    engine.zone = Some(jiff::tz::TimeZone::UTC);
    engine
}

/// Wall clock anchored to one instant, with monotonic time advancing identically.
#[derive(Clone, Copy)]
struct Clock {
    anchor_ms: i64,
}
impl Clock {
    fn moment(self, wall_ms: i64) -> Moment {
        Moment {
            wall_ms,
            mono_ms: (wall_ms - self.anchor_ms) as u64,
        }
    }
}
fn instant(text: &str) -> i64 {
    text.parse::<jiff::Timestamp>().unwrap().as_millisecond()
}
fn clocked_engine(dir: &Path, clock: Clock, zone: &str) -> Engine {
    let mut engine = Engine::new(
        Store::open(&dir.join("focus.sqlite3")).unwrap(),
        clock.moment(clock.anchor_ms),
    );
    engine.zone = Some(jiff::tz::TimeZone::get(zone).unwrap());
    engine
}
/// Skip the heartbeat check to jump a long simulated session forward in one call.
fn jump(engine: &mut Engine, now: Moment) -> Result<bool> {
    engine.observed = now;
    engine.apply(None, now)
}
#[derive(Debug, PartialEq)]
struct Row {
    start: i64,
    end: Option<i64>,
    checkpoint: i64,
    elapsed: i64,
    state: String,
    reason: Option<String>,
}
fn rows(engine: &Engine) -> Vec<Row> {
    engine
        .store
        .connection
        .prepare("SELECT started_ms,ended_ms,checkpoint_ms,elapsed_ms,state,end_reason FROM focus_sessions ORDER BY id")
        .unwrap()
        .query_map([], |r| {
            Ok(Row {
                start: r.get(0)?,
                end: r.get(1)?,
                checkpoint: r.get(2)?,
                elapsed: r.get(3)?,
                state: r.get(4)?,
                reason: r.get(5)?,
            })
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}
fn whole_day(date: &str, timezone: &str) -> reports::Request {
    reports::Request {
        from: date.into(),
        through: date.into(),
        timezone: timezone.into(),
        from_time: "00:00".into(),
        through_time: "24:00".into(),
    }
}

#[test]
fn midnight_closes_the_interval_and_continues_the_same_activity_at_one_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 90_000;
    let clock = Clock { anchor_ms: start };
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    let chosen = activity("Example");
    engine
        .apply(Some(Command::Start(chosen.clone())), clock.moment(start))
        .unwrap();
    let recent = engine.snapshot(clock.moment(start)).unwrap().recent;
    let rank = engine
        .snapshot(clock.moment(start))
        .unwrap()
        .project_rank(&chosen);
    for second in 1..=90 {
        let now = clock.moment(start + second * 1000);
        engine.apply(None, now).unwrap();
        assert_eq!(rows(&engine).len(), 1, "no split until the day has started");
    }
    // The tick that lands exactly on midnight is still inside the half-open interval.
    assert_eq!(
        engine.snapshot(clock.moment(midnight)).unwrap().completed,
        0
    );
    let first_tick = midnight + 700;
    engine.apply(None, clock.moment(first_tick)).unwrap();
    let split = rows(&engine);
    assert_eq!(
        split,
        vec![
            Row {
                start,
                end: Some(midnight),
                checkpoint: midnight,
                elapsed: 90_000,
                state: "stopped".into(),
                reason: Some("day_boundary".into()),
            },
            Row {
                start: midnight,
                end: None,
                checkpoint: first_tick,
                elapsed: 700,
                state: "running".into(),
                reason: None,
            },
        ]
    );
    let state = engine.snapshot(clock.moment(first_tick)).unwrap();
    assert!(state.running);
    assert_eq!(state.elapsed_ms, 700, "the current interval restarts");
    assert_eq!(state.active_start_ms, Some(midnight));
    assert_eq!((state.total_ms, state.completed), (90_000, 1));
    assert_eq!(state.daily_all_ms, Some(700));
    assert_eq!(state.daily_project_ms, Some(700));
    assert_eq!(state.activity, chosen, "the selection is not a new choice");
    assert_eq!(state.recent.len(), recent.len());
    assert_eq!(
        state.project_rank(&chosen),
        rank,
        "a split is not a new use"
    );
    let activities: Vec<String> = engine
        .store
        .connection
        .prepare("SELECT activity FROM focus_sessions")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(activities[0], activities[1]);
    assert_eq!(
        serde_json::from_str::<Activity>(&activities[1]).unwrap(),
        chosen
    );
    engine
        .apply(Some(Command::Stop), clock.moment(midnight + 5_000))
        .unwrap();
    let state = engine.snapshot(clock.moment(midnight + 5_000)).unwrap();
    assert_eq!(state.total_ms, 95_000, "parts add up to the whole duration");
    assert_eq!(state.completed, 2);
    let overlaps: i64 = engine.store.connection.query_row("SELECT COUNT(*) FROM focus_sessions a JOIN focus_sessions b ON a.id<b.id AND a.started_ms<b.ended_ms AND b.started_ms<a.ended_ms", [], |r| r.get(0)).unwrap();
    assert_eq!(overlaps, 0);
}

#[test]
fn explicit_commands_after_midnight_split_first_and_stopping_at_midnight_does_not() {
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 10_000;
    let clock = Clock { anchor_ms: start };
    // Stopping exactly at midnight leaves one interval and no empty continuation.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    engine
        .apply(Some(Command::Stop), clock.moment(midnight))
        .unwrap();
    let stored = rows(&engine);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].end, Some(midnight));
    assert_eq!(stored[0].reason.as_deref(), Some("stop"));
    // A switch just after midnight attributes the old activity to both days.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    engine
        .apply(
            Some(Command::Switch(activity("B"))),
            clock.moment(midnight + 2_000),
        )
        .unwrap();
    let stored = rows(&engine);
    assert_eq!(stored.len(), 3);
    assert_eq!(
        (stored[0].start, stored[0].end, stored[0].elapsed),
        (start, Some(midnight), 10_000)
    );
    assert_eq!(
        (stored[1].start, stored[1].end, stored[1].elapsed),
        (midnight, Some(midnight + 2_000), 2_000)
    );
    assert_eq!(stored[1].reason.as_deref(), Some("switch"));
    assert_eq!(stored[2].start, midnight + 2_000);
    // Editing the current interval after a split leaves the finished day untouched.
    engine
        .apply(
            Some(Command::SetChangeMode(ChangeMode::EditCurrent)),
            clock.moment(midnight + 3_000),
        )
        .unwrap();
    engine
        .apply(
            Some(Command::ChangeActivity(activity("Corrected"))),
            clock.moment(midnight + 4_000),
        )
        .unwrap();
    let labels: Vec<String> = engine
        .store
        .connection
        .prepare("SELECT activity FROM focus_sessions ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| {
            serde_json::from_str::<Activity>(&r.unwrap())
                .unwrap()
                .label()
                .to_owned()
        })
        .collect();
    assert_eq!(labels, ["A", "A", "Corrected"]);
}

#[test]
fn late_ticks_divide_monotonic_time_and_interrupted_sessions_never_split() {
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 10_000;
    let clock = Clock { anchor_ms: start };
    let dir = tempfile::tempdir().unwrap();
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    // Delayed inside the heartbeat allowance, with the wall clock two seconds ahead.
    let late = Moment {
        wall_ms: midnight + 12_000,
        mono_ms: 20_000,
    };
    engine.apply(None, late).unwrap();
    let stored = rows(&engine);
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].elapsed + stored[1].elapsed, 20_000);
    assert_eq!(stored[1].elapsed, 12_000);
    assert_eq!(stored[1].checkpoint, midnight + 12_000);
    // A gap beyond the allowance is reviewed from its checkpoint, before the next day.
    let start = midnight - 15_000;
    let clock = Clock { anchor_ms: start };
    let dir = tempfile::tempdir().unwrap();
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    engine.apply(None, clock.moment(midnight - 1_000)).unwrap();
    engine.apply(None, clock.moment(midnight + 60_000)).unwrap();
    let stored = rows(&engine);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].state, "interrupted");
    assert_eq!(stored[0].end, Some(midnight - 1_000));
    assert!(
        !engine
            .snapshot(clock.moment(midnight + 60_000))
            .unwrap()
            .running
    );
}

#[test]
fn crash_after_split_keeps_the_finished_day_and_reviews_only_the_continuation() {
    let dir = tempfile::tempdir().unwrap();
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 20_000;
    let clock = Clock { anchor_ms: start };
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    engine.apply(None, clock.moment(midnight + 500)).unwrap();
    drop(engine);
    let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
    let state = store.snapshot().unwrap();
    assert!(state.needs_review && !state.running);
    assert_eq!((state.total_ms, state.completed), (20_000, 1));
    let review: (i64, i64) = store
        .connection
        .query_row(
            "SELECT started_ms,elapsed_ms FROM focus_sessions WHERE state='interrupted'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(review, (midnight, 500));
}

#[test]
fn disabling_the_split_keeps_one_interval_and_the_same_daily_totals() {
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 7_000;
    let stop = midnight + 3_000;
    let clock = Clock { anchor_ms: start };
    let mut totals = Vec::new();
    for split in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = clocked_engine(dir.path(), clock, "UTC");
        engine
            .apply(Some(Command::SetSplitOnDay(split)), clock.moment(start))
            .unwrap();
        assert_eq!(
            engine.snapshot(clock.moment(start)).unwrap().split_on_day,
            split
        );
        engine
            .apply(Some(Command::Start(activity("A"))), clock.moment(start))
            .unwrap();
        for ms in (start + 1_000..=stop).step_by(1_000) {
            engine.apply(None, clock.moment(ms)).unwrap();
        }
        assert_eq!(rows(&engine).len(), if split { 2 } else { 1 });
        // Today includes only the new day's part while still running, in both modes.
        assert_eq!(
            engine.snapshot(clock.moment(stop)).unwrap().daily_all_ms,
            Some(3_000)
        );
        let running = engine
            .store
            .report(&whole_day("2026-10-27", "UTC"), clock.moment(stop), {
                engine.snapshot(clock.moment(stop)).unwrap().elapsed_ms
            })
            .unwrap();
        assert_eq!(running.running_ms, 3_000);
        engine
            .apply(Some(Command::Stop), clock.moment(stop))
            .unwrap();
        let state = engine.snapshot(clock.moment(stop)).unwrap();
        assert_eq!(state.daily_all_ms, Some(3_000));
        let report = |date: &str| {
            engine
                .store
                .report(&whole_day(date, "UTC"), clock.moment(stop), 0)
                .unwrap()
                .saved_ms
        };
        totals.push((state.total_ms, report("2026-10-26"), report("2026-10-27")));
        drop(engine);
        let reopened = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        assert_eq!(reopened.snapshot().unwrap().split_on_day, split);
    }
    assert_eq!(totals[0], (10_000, 7_000, 3_000));
    assert_eq!(totals[0], totals[1], "the option must not change any total");
}

#[test]
fn enabling_the_split_during_a_long_interval_divides_every_passed_day() {
    let dir = tempfile::tempdir().unwrap();
    let start = instant("2026-10-25T23:00:00Z");
    let clock = Clock { anchor_ms: start };
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::SetSplitOnDay(false)), clock.moment(start))
        .unwrap();
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    let now = instant("2026-10-27T01:00:00Z");
    jump(&mut engine, clock.moment(now)).unwrap();
    assert_eq!(rows(&engine).len(), 1);
    engine
        .apply(Some(Command::SetSplitOnDay(true)), clock.moment(now))
        .unwrap();
    let later = now + 1_000;
    engine.apply(None, clock.moment(later)).unwrap();
    let stored = rows(&engine);
    assert_eq!(stored.len(), 3);
    let first_day = instant("2026-10-26T00:00:00Z");
    let second_day = instant("2026-10-27T00:00:00Z");
    assert_eq!(
        (stored[0].start, stored[0].end, stored[0].elapsed),
        (start, Some(first_day), 3_600_000)
    );
    assert_eq!(
        (stored[1].start, stored[1].end, stored[1].elapsed),
        (first_day, Some(second_day), 86_400_000)
    );
    assert_eq!((stored[2].start, stored[2].end), (second_day, None));
    assert_eq!(stored[2].elapsed, 3_601_000);
    assert_eq!(stored[2].checkpoint, later);
    assert_eq!(
        engine.snapshot(clock.moment(later)).unwrap().elapsed_ms,
        3_601_000
    );
}

#[test]
fn day_starts_follow_local_calendar_through_dst_gaps_and_skipped_dates() {
    // (zone, start, later, expected finished interval lengths in hours)
    for (zone, start, later, hours) in [
        // Spring forward and fall back make 23- and 25-hour local days.
        (
            "America/New_York",
            "2026-03-08T04:00:00Z",
            "2026-03-09T05:00:00Z",
            vec![1, 23],
        ),
        (
            "America/New_York",
            "2026-11-01T03:00:00Z",
            "2026-11-02T06:00:00Z",
            vec![1, 25],
        ),
        // Midnight itself does not exist, so the day starts at 01:00 local time.
        (
            "America/Sao_Paulo",
            "2015-10-18T02:00:00Z",
            "2015-10-19T03:00:00Z",
            vec![1, 23],
        ),
        // The date 2011-12-30 was skipped, without an empty interval for it.
        (
            "Pacific/Apia",
            "2011-12-30T09:00:00Z",
            "2011-12-31T11:00:00Z",
            vec![1, 24],
        ),
        // Non-hour offset: local midnight falls at 18:15 UTC.
        (
            "Asia/Kathmandu",
            "2026-05-01T17:15:00Z",
            "2026-05-02T19:15:00Z",
            vec![1, 24],
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let begin = instant(start);
        let clock = Clock { anchor_ms: begin };
        let mut engine = clocked_engine(dir.path(), clock, zone);
        engine
            .apply(Some(Command::Start(activity("A"))), clock.moment(begin))
            .unwrap();
        let end = instant(later);
        jump(&mut engine, clock.moment(end)).unwrap();
        let stored = rows(&engine);
        let finished: Vec<i64> = stored
            .iter()
            .filter(|r| r.state == "stopped")
            .map(|r| r.elapsed / 3_600_000)
            .collect();
        let finished_hours: Vec<i64> = hours.iter().map(|h| i64::from(*h)).collect();
        assert_eq!(finished, finished_hours, "{zone} {start}");
        assert_eq!(
            stored.iter().map(|r| r.elapsed).sum::<i64>(),
            end - begin,
            "{zone} {start}"
        );
        for pair in stored.windows(2) {
            assert_eq!(pair[0].end, Some(pair[1].start), "{zone} {start}");
        }
        let tz = jiff::tz::TimeZone::get(zone).unwrap();
        for row in &stored {
            let first = jiff::Timestamp::from_millisecond(row.start)
                .unwrap()
                .to_zoned(tz.clone())
                .date();
            if let Some(end) = row.end {
                let last = jiff::Timestamp::from_millisecond(end - 1)
                    .unwrap()
                    .to_zoned(tz.clone())
                    .date();
                assert_eq!(first, last, "an interval spans a local date: {zone}");
            }
        }
    }
}

#[test]
fn failed_split_keeps_the_interval_and_retries_while_commands_still_work() {
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 5_000;
    let clock = Clock { anchor_ms: start };
    let dir = tempfile::tempdir().unwrap();
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    engine.store.connection.execute_batch("CREATE TRIGGER fail_split BEFORE INSERT ON focus_sessions BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
    assert!(engine.apply(None, clock.moment(midnight + 1_000)).is_err());
    let stored = rows(&engine);
    assert_eq!(
        stored.len(),
        1,
        "the failed split must roll back completely"
    );
    assert_eq!(stored[0].state, "running");
    assert_eq!(engine.started.unwrap().wall_ms, start);
    // Checkpoints continue while the split keeps failing.
    assert!(engine.apply(None, clock.moment(midnight + 5_000)).is_err());
    assert_eq!(rows(&engine)[0].elapsed, 10_000);
    assert_eq!(
        engine
            .snapshot(clock.moment(midnight + 5_000))
            .unwrap()
            .elapsed_ms,
        10_000
    );
    // Report requests are not blocked, and clip the unsplit interval correctly.
    let now = clock.moment(midnight + 5_500);
    let request = whole_day("2026-10-27", "UTC");
    assert!(
        !engine
            .apply(
                Some(Command::Report {
                    id: 1,
                    request: request.clone(),
                }),
                now,
            )
            .unwrap()
    );
    let elapsed = engine.snapshot(now).unwrap().elapsed_ms;
    assert_eq!(
        engine
            .store
            .report(&request, now, elapsed)
            .unwrap()
            .running_ms,
        5_500
    );
    // Stop is not blocked either.
    let stopped_dir = tempfile::tempdir().unwrap();
    let mut stopped = clocked_engine(stopped_dir.path(), clock, "UTC");
    stopped
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    stopped.store.connection.execute_batch("CREATE TRIGGER fail_split BEFORE INSERT ON focus_sessions BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
    stopped
        .apply(Some(Command::Stop), clock.moment(midnight + 1_000))
        .unwrap();
    assert_eq!(rows(&stopped).len(), 1);
    assert!(
        !stopped
            .snapshot(clock.moment(midnight + 1_000))
            .unwrap()
            .running
    );
    // Recovery after the failure splits at the original boundary.
    engine
        .store
        .connection
        .execute_batch("DROP TRIGGER fail_split;")
        .unwrap();
    engine.apply(None, clock.moment(midnight + 6_000)).unwrap();
    let stored = rows(&engine);
    assert_eq!(stored.len(), 2);
    assert_eq!((stored[0].end, stored[0].elapsed), (Some(midnight), 5_000));
    assert_eq!((stored[1].start, stored[1].elapsed), (midnight, 6_000));
}

#[test]
fn queued_lifecycle_boundary_is_a_clean_stop_not_a_clock_change() {
    // A tick processed after the boundary was captured must not look like a rewind.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("A"))), at(0))
        .unwrap();
    engine.apply(None, at(4_000)).unwrap();
    engine.apply(None, at(5_000)).unwrap();
    engine
        .apply(
            Some(Command::Interrupt {
                reason: "lock",
                boundary: None,
                done: None,
            }),
            at(4_500),
        )
        .unwrap();
    let state = engine.snapshot(at(5_500)).unwrap();
    assert!(!state.running && !state.needs_review);
    assert_eq!(state.total_ms, 4_500);
    // A boundary captured just before a queued start still stops that interval.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = self::engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("A"))), at(1_000))
        .unwrap();
    engine
        .apply(
            Some(Command::Interrupt {
                reason: "lock",
                boundary: None,
                done: None,
            }),
            at(900),
        )
        .unwrap();
    let state = engine.snapshot(at(1_100)).unwrap();
    assert!(!state.running && !state.needs_review);
    assert_eq!(state.total_ms, 0);
}

#[test]
fn small_backward_clock_corrections_keep_tracking_and_the_timeline_monotonic() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("A"))), at(0))
        .unwrap();
    engine.apply(None, at(10_000)).unwrap();
    // Windows synchronizes and steps the wall clock back three seconds.
    let corrected = |mono_ms: u64| Moment {
        wall_ms: 1_000_000 + mono_ms as i64 - 3_000,
        mono_ms,
    };
    engine.apply(None, corrected(11_000)).unwrap();
    let state = engine.snapshot(corrected(11_000)).unwrap();
    assert!(state.running && !state.needs_review);
    assert_eq!(
        state.elapsed_ms, 11_000,
        "monotonic time stays authoritative"
    );
    // Saved times continue by monotonic time, so the next interval cannot overlap this one.
    engine
        .apply(Some(Command::Switch(activity("B"))), corrected(12_000))
        .unwrap();
    let stored = rows(&engine);
    assert_eq!(stored[0].end, Some(at(12_000).wall_ms));
    assert_eq!(stored[0].elapsed, 12_000);
    assert_eq!(stored[1].start, at(12_000).wall_ms);
    engine.apply(None, corrected(14_000)).unwrap();
    engine
        .apply(Some(Command::Stop), corrected(15_000))
        .unwrap();
    let stored = rows(&engine);
    assert_eq!(stored[1].end, Some(at(15_000).wall_ms));
    assert_eq!(stored[1].elapsed, 3_000);
    let overlaps: i64 = engine.store.connection.query_row("SELECT COUNT(*) FROM focus_sessions a JOIN focus_sessions b ON a.id<b.id AND a.started_ms<b.ended_ms AND b.started_ms<a.ended_ms", [], |r| r.get(0)).unwrap();
    assert_eq!(overlaps, 0);
    // Beyond the five-second allowance, it is still a clock change that needs review.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = self::engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("A"))), at(0))
        .unwrap();
    engine.apply(None, at(10_000)).unwrap();
    let changed = Moment {
        wall_ms: at(11_000).wall_ms - 6_000,
        mono_ms: 11_000,
    };
    engine.apply(None, changed).unwrap();
    assert!(engine.snapshot(changed).unwrap().needs_review);
}

#[test]
fn interval_inside_a_clock_correction_still_counts_everywhere() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    let corrected = |mono_ms: u64| Moment {
        wall_ms: 1_000_000 + mono_ms as i64 - 2_000,
        mono_ms,
    };
    let request = whole_day("1970-01-01", "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), at(0))
        .unwrap();
    engine.apply(None, corrected(1_000)).unwrap();
    // While running, a report and Today agree with the timer despite the earlier clock.
    let state = engine.snapshot(corrected(1_500)).unwrap();
    assert_eq!(state.elapsed_ms, 1_500);
    assert_eq!(state.daily_all_ms, Some(1_500));
    let report = engine
        .store
        .report(
            &request,
            engine.on_timeline(corrected(1_500)),
            state.elapsed_ms,
        )
        .unwrap();
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.running_ms, 1_500);
    // The whole interval lies within the correction, yet it has a real wall span.
    engine.apply(Some(Command::Stop), corrected(2_000)).unwrap();
    let stored = rows(&engine);
    assert_eq!(
        (stored[0].start, stored[0].end, stored[0].elapsed),
        (at(0).wall_ms, Some(at(2_000).wall_ms), 2_000)
    );
    let report = engine.store.report(&request, corrected(2_000), 0).unwrap();
    assert!(report.diagnostics.is_empty(), "the session is not excluded");
    assert_eq!(report.saved_ms, 2_000);
    let state = engine.snapshot(corrected(2_000)).unwrap();
    assert_eq!(state.total_ms, 2_000);
    assert_eq!(state.daily_all_ms, Some(2_000), "all totals agree");
}

#[test]
fn repeated_snapshots_reuse_stored_state_until_something_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("A"))), at(0))
        .unwrap();
    assert_eq!(engine.snapshot(at(100)).unwrap().elapsed_ms, 100);
    // A direct change is not noticed, which proves that the cached state is in use.
    engine
        .store
        .connection
        .execute("UPDATE focus_preferences SET split_on_change=0", [])
        .unwrap();
    let state = engine.snapshot(at(200)).unwrap();
    assert_eq!(state.change_mode, ChangeMode::NewInterval);
    assert_eq!(state.elapsed_ms, 200, "live time is still current");
    engine
        .apply(
            Some(Command::Report {
                id: 1,
                request: whole_day("1970-01-01", "UTC"),
            }),
            at(300),
        )
        .unwrap();
    assert_eq!(
        engine.snapshot(at(300)).unwrap().change_mode,
        ChangeMode::NewInterval,
        "reports do not change stored state"
    );
    // Every command that can change stored state refreshes it.
    engine.apply(Some(Command::Stop), at(400)).unwrap();
    let state = engine.snapshot(at(400)).unwrap();
    assert_eq!(state.change_mode, ChangeMode::EditCurrent);
    assert_eq!(
        (state.running, state.total_ms, state.completed),
        (false, 400, 1)
    );
    // So does a split, which happens on a tick without any command.
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 5_000;
    let clock = Clock { anchor_ms: start };
    let dir = tempfile::tempdir().unwrap();
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    engine.snapshot(clock.moment(start)).unwrap();
    engine.apply(None, clock.moment(midnight + 1_000)).unwrap();
    let state = engine.snapshot(clock.moment(midnight + 1_000)).unwrap();
    assert_eq!((state.total_ms, state.completed), (5_000, 1));
    assert_eq!(state.active_start_ms, Some(midnight));
    // And an interruption after a long gap.
    engine.snapshot(clock.moment(midnight + 1_000)).unwrap();
    engine.apply(None, clock.moment(midnight + 60_000)).unwrap();
    let state = engine.snapshot(clock.moment(midnight + 60_000)).unwrap();
    assert!(state.needs_review && !state.running);
}

#[test]
fn today_counters_match_reports_in_the_same_zone_across_midnight() {
    let midnight = instant("2026-10-27T00:00:00Z");
    let start = midnight - 40_000;
    let clock = Clock { anchor_ms: start };
    let dir = tempfile::tempdir().unwrap();
    let mut engine = clocked_engine(dir.path(), clock, "UTC");
    engine
        .apply(Some(Command::Start(activity("A"))), clock.moment(start))
        .unwrap();
    for (ms, choice) in [
        (start + 10_000, Some(activity("B"))),
        (start + 20_000, None),
        (start + 30_000, Some(Activity::default())),
        (midnight, None),
        (midnight + 10_000, None),
        (midnight + 20_000, Some(activity("A"))),
        (midnight + 25_000, None),
    ] {
        let now = clock.moment(ms);
        match choice {
            Some(next) => engine.apply(Some(Command::Switch(next)), now).unwrap(),
            None => engine.apply(None, now).unwrap(),
        };
        let today = if ms < midnight {
            "2026-10-26"
        } else {
            "2026-10-27"
        };
        let state = engine.snapshot(now).unwrap();
        let report = engine
            .store
            .report(&whole_day(today, "UTC"), now, state.elapsed_ms)
            .unwrap();
        assert_eq!(
            state.daily_all_ms,
            Some(report.saved_ms + report.running_ms),
            "Today must equal the report at {ms}"
        );
    }
    let now = clock.moment(midnight + 25_000);
    engine.apply(Some(Command::Stop), now).unwrap();
    let state = engine.snapshot(now).unwrap();
    let report = engine
        .store
        .report(&whole_day("2026-10-27", "UTC"), now, 0)
        .unwrap();
    assert_eq!(state.daily_all_ms, Some(report.saved_ms));
    assert_eq!(report.saved_ms, 25_000);
    let earlier = engine
        .store
        .report(&whole_day("2026-10-26", "UTC"), now, 0)
        .unwrap();
    assert_eq!(earlier.saved_ms, 40_000);
    assert_eq!(state.total_ms, 65_000);
    let alpha = report
        .projects
        .iter()
        .find(|p| p.project == "A")
        .map(|p| p.saved_ms);
    assert_eq!(alpha, Some(5_000));
}

/// One stored session: start, end, elapsed time, state and raw activity text.
type LegacySession = (i64, Option<i64>, i64, &'static str, String);

/// Activity text exactly as versions before schema four stored it.
fn legacy_activity(source: i64, slug: &str, title: &str, task: Option<&str>) -> String {
    serde_json::json!({
        "project": {
            "source_id": source,
            "source_path": format!("synthetic-source-{source}"),
            "slug": slug,
            "title": title,
            "task": task,
        },
        "note": "Example note",
    })
    .to_string()
}
fn unassigned_activity() -> String {
    r#"{"project":null,"note":""}"#.to_owned()
}
/// A database exactly as an older version created it, not derived from today's schema.
fn legacy_database(path: &Path, version: i64, sessions: &[LegacySession]) {
    let connection = Connection::open(path).unwrap();
    connection.execute_batch("CREATE TABLE focus_sessions (id INTEGER PRIMARY KEY, started_ms INTEGER NOT NULL, checkpoint_ms INTEGER NOT NULL, ended_ms INTEGER, elapsed_ms INTEGER NOT NULL DEFAULT 0 CHECK(elapsed_ms>=0), state TEXT NOT NULL CHECK(state IN ('running','stopped','interrupted')), activity TEXT NOT NULL, end_reason TEXT);
        CREATE UNIQUE INDEX one_running ON focus_sessions((1)) WHERE state='running';
        CREATE TABLE focus_selection(singleton INTEGER PRIMARY KEY CHECK(singleton=1), activity TEXT NOT NULL);
        INSERT INTO focus_selection VALUES(1, '{\"project\":null,\"note\":\"\"}');").unwrap();
    if version >= 2 {
        connection.execute_batch("CREATE TABLE focus_preferences(singleton INTEGER PRIMARY KEY CHECK(singleton=1), split_on_change INTEGER NOT NULL CHECK(split_on_change IN (0,1)));
            INSERT INTO focus_preferences VALUES(1,1);
            CREATE TABLE focus_recent(activity_key TEXT PRIMARY KEY, used_order INTEGER NOT NULL);
            CREATE INDEX focus_recent_order ON focus_recent(used_order);").unwrap();
    }
    if version >= 3 {
        connection.execute_batch("ALTER TABLE focus_preferences ADD COLUMN split_on_day INTEGER NOT NULL DEFAULT 1 CHECK(split_on_day IN (0,1));").unwrap();
    }
    for (start, end, elapsed, state, activity) in sessions {
        connection.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,ended_ms,elapsed_ms,state,activity,end_reason) VALUES(?1,?2,?3,?4,?5,?6,'stop')", rusqlite::params![start, end.unwrap_or(start + elapsed), end, elapsed, state, activity]).unwrap();
    }
    connection
        .execute_batch(&format!("PRAGMA user_version={version};"))
        .unwrap();
}
fn schema_two_database(path: &Path) {
    legacy_database(
        path,
        2,
        &[(
            1_000_000,
            Some(1_001_000),
            1_000,
            "stopped",
            legacy_activity(1, "saved", "Saved", None),
        )],
    );
    Connection::open(path)
        .unwrap()
        .execute("UPDATE focus_preferences SET split_on_change=0", [])
        .unwrap();
}
fn backup_files(dir: &Path, prefix: &str) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix))
        .collect()
}

#[test]
fn schema_two_upgrade_backs_up_and_enables_the_split_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    schema_two_database(&path);
    let store = Store::open(&path).unwrap();
    let state = store.snapshot().unwrap();
    assert!(
        state.split_on_day,
        "splitting is the default after upgrading"
    );
    assert_eq!(
        state.change_mode,
        ChangeMode::EditCurrent,
        "the existing preference survives"
    );
    assert_eq!((state.total_ms, state.completed), (1_000, 1));
    assert_eq!(
        store
            .connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
    drop(store);
    Store::open(&path).unwrap();
    let found = backup_files(dir.path(), "focus-before-v3-");
    assert_eq!(found.len(), 1, "reopening must not back up again");
    assert!(
        backup_files(dir.path(), "focus-before-v4-").is_empty(),
        "the first backup already holds the original"
    );
    let backup = Connection::open(&found[0]).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        backup
            .query_row("SELECT SUM(elapsed_ms) FROM focus_sessions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1_000
    );
    assert!(
        backup
            .prepare("SELECT split_on_day FROM focus_preferences")
            .is_err(),
        "the backup keeps the old schema"
    );
}

#[test]
fn failed_schema_three_upgrade_keeps_schema_two_and_newer_history_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    schema_two_database(&path);
    // A conflicting column forces the upgrade statement itself to fail.
    Connection::open(&path)
        .unwrap()
        .execute_batch("ALTER TABLE focus_preferences ADD COLUMN split_on_day TEXT;")
        .unwrap();
    assert!(Store::open(&path).is_err());
    let original = Connection::open(&path).unwrap();
    assert_eq!(
        original
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        original
            .query_row("SELECT SUM(elapsed_ms) FROM focus_sessions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1_000
    );
    drop(original);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    drop(Store::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute_batch(&format!("PRAGMA user_version={};", SCHEMA_VERSION + 1))
        .unwrap();
    assert!(Store::open(&path).is_err());
    assert!(export_snapshot(&path, &whole_day("2026-01-01", "UTC")).is_err());
}

#[test]
fn timer_ticks_write_only_ten_second_checkpoints_and_idle_ticks_do_not_write() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    let initial = engine.store.connection.total_changes();
    for second in 1..=60 {
        engine.apply(None, at(second * 1000)).unwrap();
        engine.snapshot(at(second * 1000)).unwrap();
    }
    assert_eq!(engine.store.connection.total_changes(), initial);
    engine
        .apply(Some(Command::Start(activity("Example"))), at(60_000))
        .unwrap();
    let started = engine.store.connection.total_changes();
    for second in 1..=60 {
        let now = at(60_000 + second * 1000);
        engine.apply(None, now).unwrap();
        engine.snapshot(now).unwrap();
        assert_eq!(
            engine.store.connection.total_changes() - started,
            second / 10
        );
    }
    engine.apply(Some(Command::Stop), at(120_500)).unwrap();
    let stopped = engine.store.connection.total_changes();
    for second in 1..=60 {
        engine.apply(None, at(120_500 + second * 1000)).unwrap();
    }
    assert_eq!(engine.store.connection.total_changes(), stopped);
    assert_eq!(engine.snapshot(at(180_500)).unwrap().total_ms, 60_500);
}

#[test]
fn picker_changes_split_by_default_at_one_boundary_and_never_start_while_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::ChangeActivity(activity("First"))), at(0))
        .unwrap();
    assert!(!engine.snapshot(at(0)).unwrap().running);
    assert!(engine.snapshot(at(0)).unwrap().recent.is_empty());
    assert_eq!(
        engine.snapshot(at(0)).unwrap().change_mode,
        ChangeMode::NewInterval
    );
    engine.apply(Some(Command::Toggle), at(10)).unwrap();
    engine
        .apply(Some(Command::ChangeActivity(activity("Second"))), at(510))
        .unwrap();
    let mut task = activity("Second");
    task.project.as_mut().unwrap().task = Some("Another task".into());
    engine
        .apply(Some(Command::ChangeActivity(task.clone())), at(810))
        .unwrap();
    engine
        .apply(Some(Command::ChangeActivity(task)), at(910))
        .unwrap();
    engine.apply(Some(Command::Stop), at(1010)).unwrap();
    let state = engine.snapshot(at(1010)).unwrap();
    assert_eq!((state.total_ms, state.completed), (1000, 3));
    let intervals: Vec<(i64, i64)> = engine
        .store
        .connection
        .prepare("SELECT started_ms,ended_ms FROM focus_sessions ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(
        intervals,
        vec![
            (at(10).wall_ms, at(510).wall_ms),
            (at(510).wall_ms, at(810).wall_ms),
            (at(810).wall_ms, at(1010).wall_ms)
        ]
    );
}

#[test]
fn editing_current_interval_preserves_its_start_elapsed_and_finalized_history() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("Saved"))), at(0))
        .unwrap();
    engine
        .apply(Some(Command::Switch(activity("Original"))), at(100))
        .unwrap();
    engine
        .apply(
            Some(Command::SetChangeMode(ChangeMode::EditCurrent)),
            at(150),
        )
        .unwrap();
    engine
        .apply(
            Some(Command::ChangeActivity(activity("Corrected"))),
            at(300),
        )
        .unwrap();
    let state = engine.snapshot(at(500)).unwrap();
    assert_eq!(
        (state.total_ms, state.completed, state.elapsed_ms),
        (100, 1, 400)
    );
    assert_eq!(state.activity, activity("Corrected"));
    assert_eq!(engine.started.unwrap().mono_ms, 100);
    assert_eq!(state.active_start_ms, Some(at(100).wall_ms));
    let rows: Vec<String> = engine
        .store
        .connection
        .prepare("SELECT activity FROM focus_sessions ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        serde_json::from_str::<Activity>(&rows[0]).unwrap(),
        activity("Saved")
    );
    assert_eq!(
        serde_json::from_str::<Activity>(&rows[1]).unwrap(),
        activity("Corrected")
    );
    // Reassignment durably checkpoints the corrected interval without restarting it.
    drop(engine);
    let restored = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
    let state = restored.snapshot().unwrap();
    assert_eq!(state.change_mode, ChangeMode::EditCurrent);
    assert!(state.needs_review);
    let checkpoint: i64 = restored
        .connection
        .query_row(
            "SELECT elapsed_ms FROM focus_sessions WHERE state='interrupted'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(checkpoint, 200);
}

#[test]
fn recency_is_durable_per_project_and_drives_project_and_task_ordering() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    let mut first = activity("Alpha");
    first.project.as_mut().unwrap().task = Some("First task".into());
    let mut second = first.clone();
    second.project.as_mut().unwrap().task = Some("Second task".into());
    for (ms, choice) in [
        (0, activity("Zulu")),
        (10, first.clone()),
        (20, second.clone()),
        (30, Activity::default()),
    ] {
        engine
            .apply(
                Some(if ms == 0 {
                    Command::Start(choice)
                } else {
                    Command::ChangeActivity(choice)
                }),
                at(ms),
            )
            .unwrap();
    }
    engine.apply(Some(Command::Stop), at(40)).unwrap();
    engine
        .apply(Some(Command::ChangeActivity(activity("Zulu"))), at(50))
        .unwrap();
    let state = engine.snapshot(at(50)).unwrap();
    let mut projects = vec![
        activity("Zulu"),
        activity("Unused"),
        first.clone(),
        Activity::default(),
    ];
    state.sort_projects(&mut projects);
    assert_eq!(
        projects.iter().map(|p| p.label()).collect::<Vec<_>>(),
        vec!["Unassigned focus", "Alpha", "Zulu", "Unused"]
    );
    let mut no_task = first.clone();
    no_task.project.as_mut().unwrap().task = None;
    let mut tasks = vec![no_task, first.clone(), second.clone()];
    state.sort_tasks(&mut tasks);
    assert_eq!(tasks[0], second);
    assert_eq!(tasks[1], first);
    let mut other_project = first.clone();
    other_project.project.as_mut().unwrap().id += 1;
    assert_eq!(state.project_rank(&other_project), 0);
    let mut renamed = first.clone();
    renamed.project.as_mut().unwrap().title = "New display title".into();
    renamed.note = "Different note".into();
    assert_eq!(state.project_rank(&renamed), state.project_rank(&first));
    let mut unused = vec![activity("Unused Z"), activity("Unused A")];
    state.sort_projects(&mut unused);
    assert_eq!(unused[0].label(), "Unused A");
    drop(engine);
    let restored = Store::open(&dir.path().join("focus.sqlite3"))
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(restored.project_rank(&first), state.project_rank(&first));
    assert_eq!(restored.task_rank(&second), state.task_rank(&second));
}

#[test]
fn failed_activity_updates_keep_history_selection_recency_and_timer_unchanged() {
    for mode in [ChangeMode::NewInterval, ChangeMode::EditCurrent] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        engine
            .apply(Some(Command::Start(activity("Original"))), at(0))
            .unwrap();
        engine
            .apply(Some(Command::SetChangeMode(mode)), at(10))
            .unwrap();
        let recent = engine.snapshot(at(10)).unwrap().recent;
        engine.store.connection.execute_batch("CREATE TRIGGER fail_recent BEFORE INSERT ON focus_recent BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
        assert!(
            engine
                .apply(Some(Command::ChangeActivity(activity("Rejected"))), at(500))
                .is_err()
        );
        let state = engine.snapshot(at(600)).unwrap();
        assert_eq!(state.activity, activity("Original"));
        assert_eq!(
            (state.running, state.elapsed_ms, state.completed),
            (true, 600, 0)
        );
        assert_eq!(state.recent.len(), recent.len());
        assert_eq!(state.project_rank(&activity("Rejected")), 0);
        assert_eq!(
            engine
                .store
                .connection
                .query_row("SELECT COUNT(*) FROM focus_sessions", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        engine
            .store
            .connection
            .execute_batch("DROP TRIGGER fail_recent; PRAGMA query_only=ON;")
            .unwrap();
        assert!(
            engine
                .apply(
                    Some(Command::SetChangeMode(if mode == ChangeMode::NewInterval {
                        ChangeMode::EditCurrent
                    } else {
                        ChangeMode::NewInterval
                    })),
                    at(700)
                )
                .is_err()
        );
        assert_eq!(engine.snapshot(at(700)).unwrap().change_mode, mode);
    }
}

#[test]
fn schema_one_upgrade_backs_up_rows_and_seeds_recent_use_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    legacy_database(
        &path,
        1,
        &[
            (
                1_000_000,
                Some(1_000_100),
                100,
                "stopped",
                legacy_activity(1, "older", "Older", None),
            ),
            (
                1_000_100,
                Some(1_000_200),
                100,
                "stopped",
                legacy_activity(1, "newest", "Newest", None),
            ),
        ],
    );
    let store = Store::open(&path).unwrap();
    let state = store.snapshot().unwrap();
    assert_eq!((state.total_ms, state.completed), (200, 2));
    assert_eq!(state.change_mode, ChangeMode::NewInterval);
    let pick = |title: &str| Activity {
        project: state
            .projects
            .iter()
            .find(|p| p.title == title)
            .map(|p| Project {
                id: p.id,
                title: p.title.clone(),
                task: None,
            }),
        note: String::new(),
    };
    assert!(state.project_rank(&pick("Newest")) > state.project_rank(&pick("Older")));
    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("focus-before-v2-")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = Connection::open(&backups[0]).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        backup
            .query_row("SELECT SUM(elapsed_ms) FROM focus_sessions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        200
    );
    drop(store);
    Store::open(&path).unwrap();
    assert_eq!(
        std::fs::read_dir(dir.path())
            .unwrap()
            .filter(|e| e
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("focus-before-v2-"))
            .count(),
        1
    );
}

#[test]
fn activity_change_after_interruption_cannot_resume_or_reassign_checkpoint() {
    for mode in [ChangeMode::NewInterval, ChangeMode::EditCurrent] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        engine
            .apply(Some(Command::SetChangeMode(mode)), at(0))
            .unwrap();
        engine
            .apply(Some(Command::Start(activity("Original"))), at(0))
            .unwrap();
        engine.apply(None, at(10_000)).unwrap();
        assert!(
            engine
                .apply(
                    Some(Command::ChangeActivity(activity("Rejected"))),
                    at(50_000)
                )
                .is_err()
        );
        let state = engine.snapshot(at(50_000)).unwrap();
        assert!(state.needs_review);
        assert!(!state.running);
        assert_eq!(state.activity, activity("Original"));
        assert_eq!(state.project_rank(&activity("Rejected")), 0);
    }
}

#[test]
fn failed_recency_backfill_keeps_schema_one_and_original_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    legacy_database(
        &path,
        1,
        &[(
            1_000_000,
            Some(1_000_100),
            100,
            "stopped",
            "invalid synthetic JSON".to_owned(),
        )],
    );
    assert!(Store::open(&path).is_err());
    let original = Connection::open(&path).unwrap();
    assert_eq!(
        original
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(original.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('focus_recent','focus_preferences')",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    assert_eq!(
        original
            .query_row("SELECT activity FROM focus_sessions", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "invalid synthetic JSON"
    );
    assert_eq!(
        original
            .query_row("SELECT elapsed_ms FROM focus_sessions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        100
    );
}

#[test]
fn switching_is_atomic_and_snapshots_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("First"))), at(0))
        .unwrap();
    assert!(
        engine
            .apply(Some(Command::Start(activity("Duplicate"))), at(1))
            .is_err()
    );
    for i in 1..=20 {
        engine
            .apply(
                Some(Command::Switch(activity(&format!("Project {i}")))),
                at(i * 10),
            )
            .unwrap();
    }
    engine.apply(Some(Command::Stop), at(1_000)).unwrap();
    let state = engine.snapshot(at(1_000)).unwrap();
    assert_eq!(state.total_ms, 1_000);
    assert_eq!(state.completed, 21);
    let overlaps: i64 = engine.store.connection.query_row("SELECT COUNT(*) FROM focus_sessions a JOIN focus_sessions b ON a.id<b.id AND a.started_ms<b.ended_ms AND b.started_ms<a.ended_ms", [], |r| r.get(0)).unwrap();
    assert_eq!(overlaps, 0);
    drop(engine);
    let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
    assert_eq!(store.snapshot().unwrap().activity, activity("Project 20"));
    let first: String = store
        .connection
        .query_row(
            "SELECT activity FROM focus_sessions ORDER BY id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Activity>(&first).unwrap(),
        activity("First")
    );
}

#[test]
fn failed_switch_rolls_back_closed_row_and_selection() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("First"))), at(0))
        .unwrap();
    engine.store.connection.execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON focus_sessions BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
    assert!(
        engine
            .apply(Some(Command::Switch(activity("Next"))), at(500))
            .is_err()
    );
    assert!(engine.snapshot(at(600)).unwrap().running);
    assert_eq!(engine.snapshot(at(600)).unwrap().elapsed_ms, 600);
    assert_eq!(
        engine.snapshot(at(600)).unwrap().activity,
        activity("First")
    );
    assert_eq!(engine.snapshot(at(600)).unwrap().completed, 0);
    engine
        .store
        .connection
        .execute_batch("DROP TRIGGER fail_insert; PRAGMA query_only=ON;")
        .unwrap();
    assert!(engine.apply(Some(Command::Stop), at(700)).is_err());
    assert!(engine.snapshot(at(800)).unwrap().running);
}

#[test]
fn confirmed_windows_shutdown_is_durable_and_does_not_resume_on_reopen() {
    for reason in ["logoff", "shutdown", "suspend"] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        engine
            .apply(Some(Command::Start(activity("Synthetic work"))), at(100))
            .unwrap();
        engine.apply(None, at(10_100)).unwrap();
        engine
            .apply(
                Some(Command::Interrupt {
                    reason,
                    boundary: None,
                    done: None,
                }),
                at(12_400),
            )
            .unwrap();
        drop(engine);
        let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        let snapshot = store.snapshot().unwrap();
        assert!(!snapshot.running);
        assert!(!snapshot.needs_review);
        assert_eq!(snapshot.total_ms, 12_300);
        assert_eq!(snapshot.completed, 1);
        let record: (i64, String) = store
            .connection
            .query_row(
                "SELECT elapsed_ms,end_reason FROM focus_sessions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(record, (12_300, reason.into()));
    }
}

#[test]
fn crash_review_counts_only_checkpoint_not_overnight_downtime() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("First"))), at(0))
        .unwrap();
    engine.apply(None, at(10_000)).unwrap();
    drop(engine);
    let mut restored = Engine::new(
        Store::open(&dir.path().join("focus.sqlite3")).unwrap(),
        at(86_400_000),
    );
    assert!(restored.snapshot(at(86_400_000)).unwrap().needs_review);
    assert!(
        restored
            .apply(Some(Command::Toggle), at(86_400_000))
            .is_err()
    );
    restored
        .apply(Some(Command::AcceptRecovery), at(86_400_001))
        .unwrap();
    let state = restored.snapshot(at(86_400_001)).unwrap();
    assert_eq!(state.total_ms, 10_000);
    assert!(!state.running);
}

#[test]
fn simulated_lock_and_suspend_stop_without_automatic_resume() {
    for reason in ["lock", "suspend"] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        engine
            .apply(Some(Command::Start(activity("First"))), at(0))
            .unwrap();
        engine
            .apply(
                Some(Command::Interrupt {
                    reason,
                    boundary: None,
                    done: None,
                }),
                at(2_000),
            )
            .unwrap();
        engine.apply(None, at(86_400_000)).unwrap();
        let state = engine.snapshot(at(86_400_000)).unwrap();
        assert!(!state.running);
        assert_eq!(state.total_ms, 2_000);
        assert!(!state.needs_review);
    }
}

#[test]
fn missed_heartbeat_and_clock_jumps_require_review() {
    for moment in [
        at(50_000),
        Moment {
            wall_ms: 990_000,
            mono_ms: 11_000,
        },
        Moment {
            wall_ms: 1_100_000,
            mono_ms: 11_000,
        },
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        engine.apply(Some(Command::Toggle), at(0)).unwrap();
        engine.apply(None, at(10_000)).unwrap();
        engine.apply(None, moment).unwrap();
        assert!(engine.snapshot(moment).unwrap().needs_review);
        engine.apply(Some(Command::AcceptRecovery), moment).unwrap();
        assert_eq!(engine.snapshot(moment).unwrap().total_ms, 10_000);
        if moment.wall_ms < 1_010_000 {
            assert!(engine.apply(Some(Command::Toggle), moment).is_err());
        }
    }
}

#[test]
fn legacy_migration_keeps_backup_and_existing_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    let old = Connection::open(&path).unwrap();
    old.execute_batch("CREATE TABLE focus_sessions(id INTEGER PRIMARY KEY,started_ms INTEGER NOT NULL,checkpoint_ms INTEGER NOT NULL,ended_ms INTEGER,elapsed_ms INTEGER NOT NULL DEFAULT 0,state TEXT NOT NULL);
        INSERT INTO focus_sessions VALUES(1,1000,6000,6000,5000,'stopped');").unwrap();
    drop(old);
    let store = Store::open(&path).unwrap();
    assert_eq!(store.snapshot().unwrap().total_ms, 5_000);
    assert_eq!(store.snapshot().unwrap().activity, Activity::default());
    let backup = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("focus-before-v1-")
        })
        .unwrap();
    let backup =
        Connection::open_with_flags(backup, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        backup
            .query_row("SELECT SUM(elapsed_ms) FROM focus_sessions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        5_000
    );
}

#[test]
fn committed_selection_persists_and_database_enforces_one_active_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Select(activity("Selected"))), at(0))
        .unwrap();
    engine.apply(Some(Command::Toggle), at(1)).unwrap();
    assert_eq!(
        engine.snapshot(at(2)).unwrap().activity,
        activity("Selected")
    );
    assert!(
        engine
            .store
            .begin(at(3).wall_ms, &activity("Duplicate"), None)
            .is_err()
    );
    engine.apply(Some(Command::Stop), at(100)).unwrap();
    engine.apply(Some(Command::Stop), at(101)).unwrap();
    assert_eq!(engine.snapshot(at(101)).unwrap().completed, 1);
    drop(engine);
    let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
    assert_eq!(store.selected().unwrap(), activity("Selected"));
}

#[test]
fn worker_exit_and_lifecycle_confirmation_follow_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    let (events, receiver) = std::sync::mpsc::channel();
    let commands = spawn(path.clone(), move |event| {
        let _ = events.send(event);
    });
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::State(_)
    ));
    commands.send(Command::Start(activity("First"))).unwrap();
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::Committed(Snapshot { running: true, .. })
    ));
    let (done, saved) = std::sync::mpsc::sync_channel(1);
    commands
        .send(Command::Interrupt {
            reason: "lock",
            boundary: None,
            done: Some(done),
        })
        .unwrap();
    saved.recv_timeout(WORKER_WAIT).unwrap();
    // Windows interruptions did not come from the interface, so they finish no command of its.
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::State(Snapshot { running: false, .. })
    ));
    commands.send(Command::Start(activity("Next"))).unwrap();
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::Committed(Snapshot { running: true, .. })
    ));
    commands.send(Command::Exit).unwrap();
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::ExitReady
    ));
    assert_eq!(Store::open(&path).unwrap().snapshot().unwrap().completed, 2);
}

#[test]
fn delayed_suspend_command_uses_captured_event_time() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    engine.apply(Some(Command::Toggle), at(0)).unwrap();
    let boot = Instant::now();
    let command = Some(Command::Interrupt {
        reason: "suspend",
        boundary: Some(Boundary {
            wall_ms: at(100).wall_ms,
            instant: boot + Duration::from_millis(100),
        }),
        done: None,
    });
    let moment = command_moment(&command, at(20_000), boot);
    engine.apply(command, moment).unwrap();
    assert_eq!(engine.snapshot(at(20_000)).unwrap().total_ms, 100);
    assert!(!engine.snapshot(at(20_000)).unwrap().running);
}

#[test]
fn stored_report_separates_saved_live_and_recovered_time_without_live_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    let request = reports::Request {
        from: "1970-01-01".into(),
        through: "1970-01-01".into(),
        timezone: "UTC".into(),
        from_time: "00:00".into(),
        through_time: "24:00".into(),
    };
    engine
        .apply(Some(Command::Start(activity("Archived project"))), at(0))
        .unwrap();
    engine
        .apply(Some(Command::Switch(activity("Current project"))), at(1000))
        .unwrap();
    let report = engine.store.report(&request, at(1500), 500).unwrap();
    assert_eq!(report.saved_ms, 1000);
    assert_eq!(report.running_ms, 500);
    assert_eq!(report.groups[0].project, "Archived project");
    engine.apply(None, at(11000)).unwrap();
    drop(engine);
    let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
    let report = store.report(&request, at(86000000), 0).unwrap();
    assert_eq!(report.saved_ms, 1000);
    assert_eq!(report.review_ms, 10000);
    assert_eq!(report.running_ms, 0);
    assert_eq!(
        report.projects.iter().map(|p| p.saved_ms).sum::<u64>(),
        1000
    );
}

#[test]
fn invalid_report_does_not_block_stop_or_acknowledge_a_tracking_command() {
    let dir = tempfile::tempdir().unwrap();
    let (events, receiver) = std::sync::mpsc::channel();
    let commands = spawn(dir.path().join("focus.sqlite3"), move |event| {
        let _ = events.send(event);
    });
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::State(_)
    ));
    commands.send(Command::Start(Activity::default())).unwrap();
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::Committed(_)
    ));
    commands
        .send(Command::Report {
            id: 42,
            request: reports::Request {
                from: "invalid".into(),
                through: "1970-01-01".into(),
                timezone: "UTC".into(),
                from_time: "00:00".into(),
                through_time: "24:00".into(),
            },
        })
        .unwrap();
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::Report {
            id: 42,
            result: Err(_)
        }
    ));
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::State(Snapshot { running: true, .. })
    ));
    commands.send(Command::Stop).unwrap();
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::Committed(Snapshot { running: false, .. })
    ));
    commands.send(Command::Exit).unwrap();
    assert!(matches!(
        receiver.recv_timeout(WORKER_WAIT).unwrap(),
        Event::ExitReady
    ));
}

fn stored_text(connection: &Connection, sql: &str) -> Vec<String> {
    connection
        .prepare(sql)
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}

#[test]
fn fresh_history_starts_at_the_current_schema_without_backups_or_projects() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
    assert_eq!(
        store
            .connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
    let state = store.snapshot().unwrap();
    assert!(state.projects.is_empty() && state.legacy_projects.is_empty());
    assert!(state.available(&Activity::default()));
    assert!(backup_files(dir.path(), "focus-before-").is_empty());
}

#[test]
fn schema_three_upgrade_turns_recorded_activities_into_projects_and_keeps_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    let original_row = legacy_activity(1, "alpha", "Old alpha", None);
    legacy_database(
        &path,
        3,
        &[
            (
                1_000_000,
                Some(1_001_000),
                1_000,
                "stopped",
                original_row.clone(),
            ),
            (
                1_001_000,
                Some(1_002_000),
                1_000,
                "stopped",
                legacy_activity(1, "alpha", "Alpha \u{1f331}", Some("Task one")),
            ),
            // The same folder name in another source was a separate project.
            (
                1_002_000,
                Some(1_003_000),
                1_000,
                "stopped",
                legacy_activity(2, "alpha", "Other source", None),
            ),
            (
                1_003_000,
                Some(1_004_000),
                1_000,
                "stopped",
                unassigned_activity(),
            ),
            (
                1_004_000,
                Some(1_004_500),
                500,
                "interrupted",
                legacy_activity(1, "alpha", "Alpha \u{1f331}", None),
            ),
            (
                1_005_000,
                None,
                200,
                "running",
                legacy_activity(1, "alpha", "Alpha \u{1f331}", None),
            ),
        ],
    );
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE focus_selection SET activity=?1",
            [legacy_activity(
                1,
                "chosen",
                "Chosen only",
                Some("Later task"),
            )],
        )
        .unwrap();
    connection
        .execute("UPDATE focus_preferences SET split_on_day=0", [])
        .unwrap();
    for (key, order) in [
        (
            r#"{"project":[1,"synthetic-source-1","alpha"],"task":null}"#,
            1,
        ),
        (
            r#"{"project":[1,"synthetic-source-1","alpha"],"task":"Task one"}"#,
            2,
        ),
        (
            r#"{"project":[2,"synthetic-source-2","alpha"],"task":null}"#,
            3,
        ),
        (r#"{"project":null,"task":null}"#, 4),
        (
            r#"{"project":[1,"synthetic-source-1","gone"],"task":null}"#,
            5,
        ),
    ] {
        connection
            .execute(
                "INSERT INTO focus_recent VALUES(?1,?2)",
                rusqlite::params![key, order],
            )
            .unwrap();
    }
    drop(connection);

    let store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
    let state = store.snapshot().unwrap();
    let project = |title: &str| {
        state
            .projects
            .iter()
            .find(|p| p.title == title)
            .unwrap_or_else(|| panic!("Missing project {title}"))
    };
    assert_eq!(
        state.projects.len(),
        3,
        "the newest title names each project"
    );
    let alpha = project("Alpha \u{1f331}");
    let other = project("Other source");
    let chosen = project("Chosen only");
    assert!(alpha.used && other.used && !chosen.used);
    assert!(state.projects.iter().all(|p| !p.archived));
    assert_eq!(alpha.tasks.len(), 1);
    assert_eq!(
        (
            alpha.tasks[0].title.as_str(),
            alpha.tasks[0].done,
            alpha.tasks[0].used
        ),
        ("Task one", false, true)
    );
    assert_eq!(
        (chosen.tasks[0].title.as_str(), chosen.tasks[0].used),
        ("Later task", false)
    );
    // Saved totals and preferences are unchanged, and recovery still applies.
    assert_eq!((state.total_ms, state.completed), (4_000, 4));
    assert!(state.needs_review && !state.running);
    assert!(!state.split_on_day);
    // The selection points at its project and keeps its task and note.
    let selected = state.activity.project.as_ref().unwrap();
    assert_eq!(selected.id, chosen.id);
    assert_eq!(selected.task.as_deref(), Some("Later task"));
    assert_eq!(state.activity.note, "Example note");
    assert!(state.available(&state.activity));
    // Finished sessions keep the title they recorded, under the shared project ID.
    let sessions = stored_text(
        &store.connection,
        "SELECT activity FROM focus_sessions ORDER BY id",
    );
    let session = |index: usize| serde_json::from_str::<Activity>(&sessions[index]).unwrap();
    assert_eq!(session(0).project.as_ref().unwrap().title, "Old alpha");
    assert_eq!(session(0).project.as_ref().unwrap().id, alpha.id);
    assert_eq!(session(1).project.as_ref().unwrap().id, alpha.id);
    assert_eq!(session(2).project.as_ref().unwrap().id, other.id);
    assert_eq!(session(3), Activity::default());
    // Recent-use order carries over; a name with no session cannot become a project.
    assert_eq!(state.recent.len(), 4);
    let by_id = |id: i64| Activity {
        project: Some(Project {
            id,
            title: "Any".into(),
            task: None,
        }),
        note: String::new(),
    };
    assert!(state.project_rank(&Activity::default()) > state.project_rank(&by_id(other.id)));
    assert!(state.project_rank(&by_id(other.id)) > state.project_rank(&by_id(alpha.id)));
    assert_eq!(state.project_rank(&by_id(chosen.id)), 0);
    // Nothing from the former source stays in history, except the conversion aliases.
    for sql in [
        "SELECT activity FROM focus_sessions",
        "SELECT activity FROM focus_selection",
        "SELECT activity_key FROM focus_recent",
    ] {
        assert!(
            stored_text(&store.connection, sql)
                .iter()
                .all(|text| !text.contains("synthetic-source") && !text.contains("slug")),
            "{sql}"
        );
    }
    let identity = |source: i64, slug: &str| {
        (
            source,
            format!("synthetic-source-{source}"),
            slug.to_owned(),
        )
    };
    assert_eq!(state.legacy_projects.len(), 3);
    assert!(
        state
            .legacy_projects
            .contains(&(identity(1, "alpha"), alpha.id))
    );
    assert!(
        state
            .legacy_projects
            .contains(&(identity(2, "alpha"), other.id))
    );
    assert!(
        state
            .legacy_projects
            .contains(&(identity(1, "chosen"), chosen.id))
    );
    // Reports total the same time per project, under current names.
    let request = whole_day("1970-01-01", "UTC");
    let now = Moment {
        wall_ms: 2_000_000,
        mono_ms: 0,
    };
    let report = store.report(&request, now, 0).unwrap();
    assert_eq!((report.saved_ms, report.review_ms), (4_000, 700));
    let totals: Vec<(&str, u64)> = report
        .projects
        .iter()
        .map(|g| (g.project.as_str(), g.saved_ms))
        .collect();
    assert_eq!(
        totals,
        [
            ("Alpha \u{1f331}", 2_000),
            ("Other source", 1_000),
            ("Unassigned focus", 1_000)
        ]
    );
    drop(store);
    assert_eq!(
        export_snapshot(&path, &request).unwrap().saved_ms,
        4_000,
        "exports read the upgraded history"
    );
    // The backup is the unchanged schema-three database, made once.
    Store::open(&path).unwrap();
    let backups = backup_files(dir.path(), "focus-before-v4-");
    assert_eq!(backups.len(), 1, "reopening must not back up again");
    let backup = Connection::open(&backups[0]).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        stored_text(
            &backup,
            "SELECT activity FROM focus_sessions ORDER BY id LIMIT 1"
        ),
        [original_row]
    );
    assert!(backup.prepare("SELECT id FROM focus_projects").is_err());
}

#[test]
fn upgraded_projects_keep_working_for_tracking_and_merge_in_reports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    legacy_database(
        &path,
        3,
        &[(
            1_000_000,
            Some(1_001_000),
            1_000,
            "stopped",
            legacy_activity(1, "alpha", "Alpha", None),
        )],
    );
    let moment = |mono_ms: u64| Moment {
        wall_ms: 1_002_000 + mono_ms as i64,
        mono_ms,
    };
    let mut engine = Engine::new(Store::open(&path).unwrap(), moment(0));
    engine.zone = Some(jiff::tz::TimeZone::UTC);
    let state = engine.snapshot(moment(0)).unwrap();
    let activity = Activity {
        project: Some(Project {
            id: state.projects[0].id,
            title: "Alpha".into(),
            task: None,
        }),
        note: String::new(),
    };
    assert!(state.available(&activity));
    engine
        .apply(Some(Command::Start(activity)), moment(0))
        .unwrap();
    engine.apply(Some(Command::Stop), moment(500)).unwrap();
    let report = engine
        .store
        .report(&whole_day("1970-01-01", "UTC"), moment(500), 0)
        .unwrap();
    assert_eq!(
        report.projects.len(),
        1,
        "old and new sessions are one project"
    );
    assert_eq!(report.projects[0].saved_ms, 1_500);
}

#[test]
fn failed_schema_four_upgrade_leaves_schema_three_history_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    let first = legacy_activity(1, "alpha", "Alpha", Some("Task"));
    legacy_database(
        &path,
        3,
        &[
            (1_000_000, Some(1_001_000), 1_000, "stopped", first.clone()),
            (
                1_001_000,
                Some(1_002_000),
                1_000,
                "stopped",
                "invalid synthetic JSON".to_owned(),
            ),
        ],
    );
    assert!(Store::open(&path).is_err());
    let original = Connection::open(&path).unwrap();
    assert_eq!(
        original
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        stored_text(&original, "SELECT activity FROM focus_sessions ORDER BY id"),
        [first, "invalid synthetic JSON".to_owned()],
        "the valid row converted before the failure is rolled back"
    );
    assert!(
        stored_text(
            &original,
            "SELECT name FROM sqlite_master WHERE name LIKE 'focus_project%' OR name='focus_tasks' OR name='focus_sessions_ended'"
        )
        .is_empty()
    );
    assert_eq!(backup_files(dir.path(), "focus-before-v4-").len(), 1);
}

fn add(engine: &mut Engine, titles: &[&str]) -> Result<bool> {
    engine.apply(
        Some(Command::AddProjects(
            titles.iter().map(|title| (*title).to_owned()).collect(),
        )),
        at(0),
    )
}
fn project_id(engine: &Engine, title: &str) -> i64 {
    engine
        .snapshot(at(0))
        .unwrap()
        .projects
        .iter()
        .find(|p| p.title == title)
        .unwrap_or_else(|| panic!("Missing project {title}"))
        .id
}
fn chosen(engine: &Engine, title: &str, task: Option<&str>) -> Activity {
    Activity {
        project: Some(Project {
            id: project_id(engine, title),
            title: title.into(),
            task: task.map(Into::into),
        }),
        note: "Example note".into(),
    }
}

#[test]
fn projects_are_listed_sorted_keep_any_language_and_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    let unicode = "\u{41f}\u{440}\u{43e}\u{435}\u{43a}\u{442} \u{1f331}";
    add(&mut engine, &["  beta  ", "Alpha", unicode]).unwrap();
    let titles = |state: &Snapshot| {
        state
            .projects
            .iter()
            .map(|p| p.title.clone())
            .collect::<Vec<_>>()
    };
    let state = engine.snapshot(at(0)).unwrap();
    assert_eq!(titles(&state), ["Alpha", "beta", unicode]);
    assert!(state.projects.iter().all(|p| !p.archived && !p.used));
    drop(engine);
    let restored = Store::open(&dir.path().join("focus.sqlite3"))
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(titles(&restored), titles(&state));
}

#[test]
fn adding_skips_existing_names_rejects_bad_ones_and_applies_all_or_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    add(&mut engine, &["Alpha"]).unwrap();
    // A pasted list can repeat existing names, including with different case.
    add(&mut engine, &["ALPHA", "Beta", "beta"]).unwrap();
    assert_eq!(engine.snapshot(at(0)).unwrap().projects.len(), 2);
    // Adding only known names is an error, so it does not silently do nothing.
    assert!(add(&mut engine, &["alpha"]).is_err());
    assert!(add(&mut engine, &["   "]).is_err());
    let long = "x".repeat(201);
    assert!(add(&mut engine, &["Gamma", &long]).is_err());
    assert_eq!(
        engine.snapshot(at(0)).unwrap().projects.len(),
        2,
        "one bad name rejects the whole list"
    );
    add(&mut engine, &[&"x".repeat(200)]).unwrap();
    // Control characters cannot break one-line displays; other text is preserved.
    add(&mut engine, &["Tab\there"]).unwrap();
    assert!(project_id(&engine, "Tab here") > 0);
}

#[test]
fn renaming_moves_the_selection_and_running_interval_but_not_finished_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    add(&mut engine, &["Old name", "Other"]).unwrap();
    let id = project_id(&engine, "Old name");
    engine
        .apply(
            Some(Command::Start(chosen(&engine, "Old name", None))),
            at(0),
        )
        .unwrap();
    // A note change makes this a real switch within the same project.
    let mut noted = chosen(&engine, "Old name", None);
    noted.note = "Second".into();
    engine.apply(Some(Command::Switch(noted)), at(100)).unwrap();
    engine
        .apply(
            Some(Command::RenameProject {
                id,
                title: "  New name ".into(),
            }),
            at(150),
        )
        .unwrap();
    let state = engine.snapshot(at(200)).unwrap();
    assert_eq!(state.activity.label(), "New name");
    assert_eq!(state.project(id).unwrap().title, "New name");
    assert!(state.running);
    assert_eq!(state.elapsed_ms, 100, "the timer is not disturbed");
    assert_eq!(engine.started.unwrap().mono_ms, 100);
    // The stored running row follows, so a crash now would keep the new name.
    let running: String = engine
        .store
        .connection
        .query_row(
            "SELECT activity FROM focus_sessions WHERE state='running'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Activity>(&running).unwrap().label(),
        "New name"
    );
    engine.apply(Some(Command::Stop), at(300)).unwrap();
    let report = engine
        .store
        .report(&whole_day("1970-01-01", "UTC"), at(300), 0)
        .unwrap();
    // Totals use the current name; the earlier session row keeps what it recorded.
    assert_eq!(report.projects.len(), 1);
    assert_eq!(report.projects[0].project, "New name");
    assert_eq!(report.groups[0].project, "New name");
    let labels: Vec<&str> = report
        .history
        .iter()
        .map(|h| h.record.activity.label())
        .collect();
    assert!(labels.contains(&"Old name") && labels.contains(&"New name"));
    // Names stay unambiguous, and a rename can change only the case of its own name.
    for title in ["other", "   "] {
        assert!(
            engine
                .apply(
                    Some(Command::RenameProject {
                        id,
                        title: title.into(),
                    }),
                    at(300),
                )
                .is_err()
        );
    }
    engine
        .apply(
            Some(Command::RenameProject {
                id,
                title: "NEW NAME".into(),
            }),
            at(300),
        )
        .unwrap();
    assert!(
        engine
            .apply(
                Some(Command::RenameProject {
                    id: 9_999,
                    title: "Missing".into(),
                }),
                at(300),
            )
            .is_err()
    );
    assert_eq!(
        engine.snapshot(at(300)).unwrap().project(id).unwrap().title,
        "NEW NAME"
    );
}

#[test]
fn archiving_blocks_new_starts_but_keeps_running_time_and_reports() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    add(&mut engine, &["Alpha"]).unwrap();
    let id = project_id(&engine, "Alpha");
    let activity = chosen(&engine, "Alpha", None);
    assert!(engine.snapshot(at(0)).unwrap().available(&activity));
    engine
        .apply(Some(Command::Start(activity.clone())), at(0))
        .unwrap();
    engine
        .apply(
            Some(Command::SetProjectArchived { id, archived: true }),
            at(50),
        )
        .unwrap();
    let state = engine.snapshot(at(60)).unwrap();
    assert!(
        state.running,
        "archiving does not stop the running interval"
    );
    assert!(!state.available(&activity));
    assert!(state.project(id).unwrap().archived);
    engine.apply(Some(Command::Stop), at(100)).unwrap();
    let report = engine
        .store
        .report(&whole_day("1970-01-01", "UTC"), at(100), 0)
        .unwrap();
    assert_eq!(report.projects[0].project, "Alpha");
    assert_eq!(report.projects[0].saved_ms, 100);
    // The name stays reserved until restored, so restoring cannot create a duplicate.
    assert!(add(&mut engine, &["alpha"]).is_err());
    engine
        .apply(
            Some(Command::SetProjectArchived {
                id,
                archived: false,
            }),
            at(100),
        )
        .unwrap();
    assert!(engine.snapshot(at(100)).unwrap().available(&activity));
    assert!(
        engine
            .apply(
                Some(Command::SetProjectArchived {
                    id: 9_999,
                    archived: true,
                }),
                at(100),
            )
            .is_err()
    );
}

#[test]
fn queued_commands_cannot_start_on_archived_projects_or_finished_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    add(&mut engine, &["Alpha", "Beta"]).unwrap();
    let alpha = project_id(&engine, "Alpha");
    let beta = project_id(&engine, "Beta");
    engine
        .apply(
            Some(Command::AddTasks {
                project: beta,
                titles: vec!["Task".into()],
            }),
            at(0),
        )
        .unwrap();
    let archived = chosen(&engine, "Alpha", None);
    engine
        .apply(Some(Command::Select(archived.clone())), at(0))
        .unwrap();
    engine
        .apply(
            Some(Command::SetProjectArchived {
                id: alpha,
                archived: true,
            }),
            at(0),
        )
        .unwrap();
    // A tray toggle decided before the archive arrived starts nothing.
    assert!(engine.apply(Some(Command::Toggle), at(1)).is_err());
    assert!(
        engine
            .apply(Some(Command::Start(archived.clone())), at(1))
            .is_err()
    );
    assert!(rows(&engine).is_empty());
    assert!(!engine.snapshot(at(1)).unwrap().running);
    let finished = chosen(&engine, "Beta", Some("Task"));
    engine
        .apply(
            Some(Command::SetTaskDone {
                project: beta,
                title: "Task".into(),
                done: true,
            }),
            at(1),
        )
        .unwrap();
    assert!(
        engine
            .apply(Some(Command::Start(finished.clone())), at(1))
            .is_err()
    );
    // While running, neither a switch nor a reassignment can move onto them either.
    engine
        .apply(Some(Command::Start(chosen(&engine, "Beta", None))), at(2))
        .unwrap();
    for mode in [ChangeMode::NewInterval, ChangeMode::EditCurrent] {
        engine
            .apply(Some(Command::SetChangeMode(mode)), at(3))
            .unwrap();
        for target in [&archived, &finished] {
            assert!(
                engine
                    .apply(Some(Command::ChangeActivity(target.clone())), at(4))
                    .is_err()
            );
        }
    }
    let state = engine.snapshot(at(5)).unwrap();
    assert!(state.running);
    assert_eq!(state.activity.label(), "Beta");
    assert_eq!(rows(&engine).len(), 1);
}

#[test]
fn project_only_selected_before_the_upgrade_can_be_deleted_after_choosing_another() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    legacy_database(&path, 3, &[]);
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE focus_selection SET activity=?1",
            [legacy_activity(1, "chosen", "Chosen only", None)],
        )
        .unwrap();
    let mut engine = Engine::new(Store::open(&path).unwrap(), at(0));
    engine.zone = Some(jiff::tz::TimeZone::UTC);
    let id = project_id(&engine, "Chosen only");
    assert!(
        engine
            .apply(Some(Command::DeleteProject(id)), at(0))
            .is_err(),
        "it is still the saved selection"
    );
    engine
        .apply(Some(Command::Select(Activity::default())), at(0))
        .unwrap();
    engine
        .apply(Some(Command::DeleteProject(id)), at(0))
        .unwrap();
    let state = engine.snapshot(at(0)).unwrap();
    assert!(state.projects.is_empty() && state.legacy_projects.is_empty());
}

#[test]
fn odd_recorded_activities_do_not_stop_the_list_from_loading() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("focus.sqlite3");
    let store = Store::open(&path).unwrap();
    for (start, activity) in [
        (1_000_i64, "not JSON"),
        (2_000, r#"{"project":{"id":"text","task":5},"note":""}"#),
        (3_000, r#"{"project":{"id":7,"task":5},"note":""}"#),
    ] {
        store.connection.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,ended_ms,elapsed_ms,state,activity) VALUES(?1,?1,?1,0,'stopped',?2)", rusqlite::params![start, activity]).unwrap();
    }
    drop(store);
    assert!(Store::open(&path).unwrap().snapshot().is_ok());
}

#[test]
fn refused_interface_commands_answer_once_and_later_commands_still_work() {
    let dir = tempfile::tempdir().unwrap();
    let (events, receiver) = std::sync::mpsc::channel();
    let commands = spawn(dir.path().join("focus.sqlite3"), move |event| {
        let _ = events.send(event);
    });
    let timeout = WORKER_WAIT;
    assert!(matches!(
        receiver.recv_timeout(timeout).unwrap(),
        Event::State(_)
    ));
    commands
        .send(Command::AddProjects(vec!["  ".into()]))
        .unwrap();
    assert!(matches!(
        receiver.recv_timeout(timeout).unwrap(),
        Event::Rejected(_)
    ));
    assert!(matches!(
        receiver.recv_timeout(timeout).unwrap(),
        Event::State(_)
    ));
    commands
        .send(Command::AddProjects(vec!["Alpha".into()]))
        .unwrap();
    assert!(matches!(
        receiver.recv_timeout(timeout).unwrap(),
        Event::Committed(_)
    ));
    commands.send(Command::Exit).unwrap();
    assert!(matches!(
        receiver.recv_timeout(timeout).unwrap(),
        Event::ExitReady
    ));
}

#[test]
fn only_never_tracked_projects_and_tasks_can_be_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    add(&mut engine, &["Tracked", "Unused", "Selected"]).unwrap();
    let tracked = project_id(&engine, "Tracked");
    let unused = project_id(&engine, "Unused");
    let selected = project_id(&engine, "Selected");
    for project in [tracked, unused] {
        engine
            .apply(
                Some(Command::AddTasks {
                    project,
                    titles: vec!["Used task".into(), "Spare task".into()],
                }),
                at(0),
            )
            .unwrap();
    }
    engine
        .apply(
            Some(Command::Start(chosen(
                &engine,
                "Tracked",
                Some("Used task"),
            ))),
            at(0),
        )
        .unwrap();
    engine.apply(Some(Command::Stop), at(100)).unwrap();
    let state = engine.snapshot(at(100)).unwrap();
    let entry = state.project(tracked).unwrap();
    let task = |title: &str| entry.tasks.iter().find(|t| t.title == title).unwrap();
    assert!(entry.used && task("Used task").used && !task("Spare task").used);
    let delete =
        |engine: &mut Engine, id: i64| engine.apply(Some(Command::DeleteProject(id)), at(100));
    assert!(delete(&mut engine, tracked).is_err(), "recorded time stays");
    let delete_task = |engine: &mut Engine, project: i64, title: &str| {
        engine.apply(
            Some(Command::DeleteTask {
                project,
                title: title.into(),
            }),
            at(100),
        )
    };
    assert!(delete_task(&mut engine, tracked, "Used task").is_err());
    delete_task(&mut engine, tracked, "Spare task").unwrap();
    assert!(delete_task(&mut engine, tracked, "Missing").is_err());
    // The saved next activity must stay valid, so it blocks deletion too.
    engine
        .apply(
            Some(Command::ChangeActivity(chosen(&engine, "Selected", None))),
            at(100),
        )
        .unwrap();
    assert!(delete(&mut engine, selected).is_err());
    delete(&mut engine, unused).unwrap();
    let state = engine.snapshot(at(100)).unwrap();
    assert!(state.project(unused).is_none());
    assert_eq!(state.project(tracked).unwrap().tasks.len(), 1);
    let orphans: i64 = engine
        .store
        .connection
        .query_row(
            "SELECT COUNT(*) FROM focus_tasks WHERE project_id=?1",
            [unused],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(orphans, 0, "tasks are removed with their project");
    assert!(delete(&mut engine, unused).is_err());
}

#[test]
fn tasks_can_be_finished_reopened_and_added_without_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    add(&mut engine, &["Alpha"]).unwrap();
    let project = project_id(&engine, "Alpha");
    let add_tasks = |engine: &mut Engine, titles: &[&str]| {
        engine.apply(
            Some(Command::AddTasks {
                project,
                titles: titles.iter().map(|t| (*t).to_owned()).collect(),
            }),
            at(0),
        )
    };
    add_tasks(&mut engine, &["First \u{1f331}", "Second"]).unwrap();
    assert!(add_tasks(&mut engine, &["first \u{1f331}"]).is_err());
    let task = chosen(&engine, "Alpha", Some("Second"));
    assert!(engine.snapshot(at(0)).unwrap().available(&task));
    assert!(
        !engine
            .snapshot(at(0))
            .unwrap()
            .available(&chosen(&engine, "Alpha", Some("Unknown"))),
        "a task must be listed to start"
    );
    let set_done = |engine: &mut Engine, title: &str, done: bool| {
        engine.apply(
            Some(Command::SetTaskDone {
                project,
                title: title.into(),
                done,
            }),
            at(0),
        )
    };
    set_done(&mut engine, "Second", true).unwrap();
    assert!(!engine.snapshot(at(0)).unwrap().available(&task));
    assert!(set_done(&mut engine, "Missing", true).is_err());
    // Adding a finished task's name again reopens it instead of duplicating it.
    add_tasks(&mut engine, &["SECOND"]).unwrap();
    let state = engine.snapshot(at(0)).unwrap();
    assert!(state.available(&task));
    assert_eq!(state.project(project).unwrap().tasks.len(), 2);
    assert!(
        engine
            .apply(
                Some(Command::AddTasks {
                    project: 9_999,
                    titles: vec!["Lost".into()],
                }),
                at(0),
            )
            .is_err()
    );
}

#[test]
fn list_changes_leave_the_timer_history_and_recent_use_alone() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    add(&mut engine, &["Alpha"]).unwrap();
    engine
        .apply(Some(Command::Start(chosen(&engine, "Alpha", None))), at(0))
        .unwrap();
    let before = engine.snapshot(at(100)).unwrap();
    engine
        .apply(Some(Command::AddProjects(vec!["Beta".into()])), at(100))
        .unwrap();
    let id = project_id(&engine, "Beta");
    for (ms, command) in [
        (
            100,
            Command::AddTasks {
                project: id,
                titles: vec!["Task".into()],
            },
        ),
        (
            200,
            Command::RenameProject {
                id,
                title: "Gamma".into(),
            },
        ),
        (300, Command::SetProjectArchived { id, archived: true }),
    ] {
        engine.apply(Some(command), at(ms)).unwrap();
    }
    let after = engine.snapshot(at(400)).unwrap();
    assert!(after.running);
    assert_eq!(after.elapsed_ms, 400);
    assert_eq!(after.completed, before.completed);
    assert_eq!(after.recent.len(), before.recent.len());
    assert_eq!(after.activity, before.activity);
    assert_eq!(rows(&engine).len(), 1);
}

/// The steps of a query plan, as SQLite reports them.
fn query_plan(connection: &Connection, sql: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .unwrap();
    let values = std::iter::repeat_n(0_i64, statement.parameter_count());
    statement
        .query_map(rusqlite::params_from_iter(values), |r| {
            r.get::<_, String>(3)
        })
        .unwrap()
        .map(|step| step.unwrap())
        .collect()
}

#[test]
fn period_queries_read_through_the_end_index_instead_of_the_whole_history() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
    for sql in [store::REPORT_ROWS, engine::DAILY_ROWS] {
        let plan = query_plan(&store.connection, sql);
        let reads: Vec<_> = plan
            .iter()
            .filter(|step| step.contains("focus_sessions"))
            .collect();
        assert!(!reads.is_empty(), "{plan:?}");
        for step in reads {
            assert!(
                step.starts_with("SEARCH focus_sessions USING INDEX focus_sessions_ended"),
                "a period query must not scan every session: {plan:?}"
            );
        }
    }
}

#[test]
fn known_usage_matches_a_fresh_history_scan_after_every_kind_of_write() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine(dir.path());
    let mut listed = |title: &str| {
        engine.store.add_projects(&[title.into()]).unwrap();
        let id = engine
            .store
            .catalog()
            .unwrap()
            .iter()
            .find(|p| p.title == title)
            .unwrap()
            .id;
        engine.store.add_tasks(id, &["Task".into()]).unwrap();
        Activity {
            project: Some(Project {
                id,
                title: title.into(),
                task: Some("Task".into()),
            }),
            note: String::new(),
        }
    };
    let [a, b, c, d, e] = ["A", "B", "C", "D", "E"].map(&mut listed);
    let key = |activity: &Activity| {
        let project = activity.project.as_ref().unwrap();
        (project.id, project.task.clone())
    };
    let known = |engine: &Engine| {
        engine.store.catalog().unwrap();
        engine.store.usage.borrow().clone().unwrap()
    };
    let fresh = |engine: &Engine| catalog::scan_usage(&engine.store.connection).unwrap();
    let mut ms = 0;
    let mut apply = |engine: &mut Engine, command: Command| {
        ms += 1000;
        engine.apply(Some(command), at(ms)).unwrap();
        engine.snapshot(at(ms)).unwrap();
    };

    // Starting adds the key to what the store already knows, without reading the history.
    assert!(known(&engine).is_empty());
    apply(&mut engine, Command::Start(a.clone()));
    apply(&mut engine, Command::Switch(b.clone()));
    apply(&mut engine, Command::Stop);
    assert_eq!(known(&engine), fresh(&engine));
    assert_eq!(known(&engine).len(), 2);

    // A change made outside the store is not noticed, which proves the known set is reused.
    engine.store.connection.execute(
        "INSERT INTO focus_sessions(started_ms,ended_ms,checkpoint_ms,elapsed_ms,state,activity) VALUES(10,20,20,10,'stopped',?1)",
        [serde_json::to_string(&e).unwrap()],
    ).unwrap();
    apply(&mut engine, Command::Start(a.clone()));
    apply(&mut engine, Command::Stop);
    assert!(!known(&engine).contains(&key(&e)));
    engine
        .store
        .connection
        .execute("DELETE FROM focus_sessions WHERE started_ms=10", [])
        .unwrap();

    // Editing the running interval can remove the last use of the replaced activity.
    apply(&mut engine, Command::SetChangeMode(ChangeMode::EditCurrent));
    apply(&mut engine, Command::Start(c.clone()));
    apply(&mut engine, Command::ChangeActivity(d.clone()));
    assert!(!known(&engine).contains(&key(&c)));
    assert!(known(&engine).contains(&key(&d)));
    assert_eq!(known(&engine), fresh(&engine));
    apply(&mut engine, Command::Stop);

    // Manual intervals add and remove keys.
    apply(
        &mut engine,
        Command::SaveInterval(IntervalEdit {
            id: None,
            recorded: None,
            start_ms: 100_000,
            end_ms: 200_000,
            activity: e.clone(),
            timezone: "UTC".into(),
        }),
    );
    assert!(known(&engine).contains(&key(&e)));
    assert_eq!(known(&engine), fresh(&engine));
    let id: i64 = engine
        .store
        .connection
        .query_row(
            "SELECT id FROM focus_sessions WHERE started_ms=100000",
            [],
            |r| r.get(0),
        )
        .unwrap();
    apply(
        &mut engine,
        Command::DeleteInterval {
            id,
            recorded: (100_000, 200_000),
        },
    );
    assert!(!known(&engine).contains(&key(&e)));
    assert_eq!(known(&engine), fresh(&engine));

    // The list follows: a project used only by the deleted interval can be deleted again.
    let project = key(&e).0;
    apply(&mut engine, Command::DeleteProject(project));
    assert!(
        engine
            .store
            .catalog()
            .unwrap()
            .iter()
            .all(|p| p.id != project)
    );
    assert_eq!(known(&engine), fresh(&engine));
}
