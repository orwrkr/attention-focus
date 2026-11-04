use super::*;
use migrate::SCHEMA_VERSION;
use rusqlite::Connection;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use worker::command_moment;
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
    other.project.as_mut().unwrap().source_id = 2;
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
            source_id: 1,
            source_path: "synthetic-source".into(),
            slug: title.into(),
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
    // Saved times never move backward, so the next interval cannot overlap this one.
    engine
        .apply(Some(Command::Switch(activity("B"))), corrected(12_000))
        .unwrap();
    let stored = rows(&engine);
    assert_eq!(stored[0].end, Some(at(10_000).wall_ms));
    assert_eq!(stored[0].elapsed, 12_000);
    assert_eq!(stored[1].start, at(10_000).wall_ms);
    engine.apply(None, corrected(14_000)).unwrap();
    engine
        .apply(Some(Command::Stop), corrected(15_000))
        .unwrap();
    let stored = rows(&engine);
    assert_eq!(stored[1].end, Some(corrected(15_000).wall_ms));
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

fn schema_two_database(path: &Path) {
    let mut engine = Engine::new(
        Store::open(path).unwrap(),
        Moment {
            wall_ms: 1_000_000,
            mono_ms: 0,
        },
    );
    engine.zone = Some(jiff::tz::TimeZone::UTC);
    engine
        .apply(Some(Command::Start(activity("Saved"))), at(0))
        .unwrap();
    engine.apply(Some(Command::Stop), at(1_000)).unwrap();
    drop(engine);
    Connection::open(path)
        .unwrap()
        .execute_batch(
            "DROP TABLE focus_preferences;
            CREATE TABLE focus_preferences(singleton INTEGER PRIMARY KEY CHECK(singleton=1), split_on_change INTEGER NOT NULL CHECK(split_on_change IN (0,1)));
            INSERT INTO focus_preferences VALUES(1,0);
            PRAGMA user_version=2;",
        )
        .unwrap();
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
    let backups = || {
        std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("focus-before-v3-")
            })
            .collect::<Vec<_>>()
    };
    Store::open(&path).unwrap();
    let found = backups();
    assert_eq!(found.len(), 1, "reopening must not back up again");
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
fn recency_is_durable_source_scoped_and_drives_project_and_task_ordering() {
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
    let mut other_source = first.clone();
    other_source.project.as_mut().unwrap().source_id = 2;
    assert_eq!(state.project_rank(&other_source), 0);
    other_source.project.as_mut().unwrap().source_id = 1;
    other_source.project.as_mut().unwrap().source_path = "another-source".into();
    assert_eq!(state.project_rank(&other_source), 0);
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
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("Older"))), at(0))
        .unwrap();
    engine
        .apply(Some(Command::Switch(activity("Newest"))), at(100))
        .unwrap();
    engine.apply(Some(Command::Stop), at(200)).unwrap();
    engine
        .store
        .connection
        .execute_batch(
            "DROP TABLE focus_preferences; DROP TABLE focus_recent; PRAGMA user_version=1;",
        )
        .unwrap();
    drop(engine);
    let store = Store::open(&path).unwrap();
    let state = store.snapshot().unwrap();
    assert_eq!((state.total_ms, state.completed), (200, 2));
    assert_eq!(state.change_mode, ChangeMode::NewInterval);
    assert!(state.project_rank(&activity("Newest")) > state.project_rank(&activity("Older")));
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
    let mut engine = engine(dir.path());
    engine
        .apply(Some(Command::Start(activity("Original"))), at(0))
        .unwrap();
    engine.apply(Some(Command::Stop), at(100)).unwrap();
    engine.store.connection.execute_batch("DROP TABLE focus_preferences; DROP TABLE focus_recent; UPDATE focus_sessions SET activity='invalid synthetic JSON'; PRAGMA user_version=1;").unwrap();
    drop(engine);
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
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::State(_)
    ));
    commands.send(Command::Start(activity("First"))).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
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
    saved.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::Committed(Snapshot { running: false, .. })
    ));
    commands.send(Command::Start(activity("Next"))).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::Committed(Snapshot { running: true, .. })
    ));
    commands.send(Command::Exit).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
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
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::State(_)
    ));
    commands.send(Command::Start(Activity::default())).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
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
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::Report {
            id: 42,
            result: Err(_)
        }
    ));
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::State(Snapshot { running: true, .. })
    ));
    commands.send(Command::Stop).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::Committed(Snapshot { running: false, .. })
    ));
    commands.send(Command::Exit).unwrap();
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::ExitReady
    ));
}
