//! Serialized, clock-injected tracking with durable activity snapshots.
use crate::reports;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn invalid(message: &str) -> std::io::Error {
    std::io::Error::other(message)
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub project: Option<Project>,
    pub note: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub source_id: i64,
    pub source_path: String,
    pub slug: String,
    pub title: String,
    pub task: Option<String>,
}
impl Activity {
    fn key(&self) -> ActivityKey {
        ActivityKey {
            project: self
                .project
                .as_ref()
                .map(|p| (p.source_id, p.source_path.clone(), p.slug.clone())),
            task: self.project.as_ref().and_then(|p| p.task.clone()),
        }
    }
    pub fn label(&self) -> &str {
        self.project
            .as_ref()
            .map_or("Unassigned focus", |p| p.title.as_str())
    }
    fn validate(&self) -> Result<()> {
        if self.note.len() > 16_384
            || self.project.as_ref().is_some_and(|p| {
                p.source_id <= 0
                    || p.slug.is_empty()
                    || p.title.is_empty()
                    || p.source_path.is_empty()
            })
        {
            return Err(invalid("Invalid activity snapshot.").into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ActivityKey {
    project: Option<(i64, String, String)>,
    task: Option<String>,
}
#[derive(Clone, Debug)]
pub struct RecentUse {
    key: ActivityKey,
    order: i64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChangeMode {
    #[default]
    NewInterval,
    EditCurrent,
}
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub running: bool,
    pub active_start_ms: Option<i64>,
    pub elapsed_ms: u64,
    pub total_ms: u64,
    pub daily_project_ms: Option<u64>,
    pub daily_all_ms: Option<u64>,
    pub completed: u64,
    pub needs_review: bool,
    pub activity: Activity,
    pub change_mode: ChangeMode,
    pub recent: Vec<RecentUse>,
}
impl Snapshot {
    pub fn sort_projects(&self, choices: &mut [Activity]) {
        choices.sort_by_cached_key(|a| {
            (
                std::cmp::Reverse(self.project_rank(a)),
                a.label().to_lowercase(),
                a.label().to_owned(),
                a.key().project,
            )
        });
    }
    pub fn sort_tasks(&self, choices: &mut [Activity]) {
        choices.sort_by_cached_key(|a| {
            let label = a
                .project
                .as_ref()
                .and_then(|p| p.task.as_deref())
                .unwrap_or("No task");
            (
                std::cmp::Reverse(self.task_rank(a)),
                label.to_lowercase(),
                label.to_owned(),
            )
        });
    }
    pub fn project_rank(&self, activity: &Activity) -> i64 {
        let key = activity.key();
        self.recent
            .iter()
            .filter(|r| r.key.project == key.project)
            .map(|r| r.order)
            .max()
            .unwrap_or(0)
    }
    pub fn task_rank(&self, activity: &Activity) -> i64 {
        let key = activity.key();
        self.recent
            .iter()
            .find(|r| r.key == key)
            .map_or(0, |r| r.order)
    }
}
pub enum Command {
    Toggle,
    Start(Activity),
    Stop,
    Switch(Activity),
    Select(Activity),
    ChangeActivity(Activity),
    SetChangeMode(ChangeMode),
    AcceptRecovery,
    Report {
        id: u64,
        request: reports::Request,
    },
    Interrupt {
        reason: &'static str,
        boundary: Option<Boundary>,
        done: Option<std::sync::mpsc::SyncSender<()>>,
    },
    Exit,
}
#[derive(Clone, Copy)]
pub struct Boundary {
    wall_ms: i64,
    instant: Instant,
}
impl Boundary {
    pub fn now() -> Self {
        Self {
            wall_ms: now_ms(),
            instant: Instant::now(),
        }
    }
}
pub enum Event {
    State(Snapshot),
    Committed(Snapshot),
    Report {
        id: u64,
        result: std::result::Result<Box<reports::Report>, String>,
    },
    Error(String),
    ExitReady,
}
pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 2 {
            return Err(invalid("History was created by a newer application version.").into());
        }
        if version == 0 {
            let legacy: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='focus_sessions')", [], |r| r.get(0))?;
            if legacy {
                let backup_file = tempfile::Builder::new()
                    .prefix("focus-before-v1-")
                    .suffix(".sqlite3")
                    .tempfile_in(
                        path.parent()
                            .ok_or_else(|| invalid("Missing storage directory."))?,
                    )?;
                let mut destination = Connection::open(backup_file.path())?;
                {
                    let backup = rusqlite::backup::Backup::new(&connection, &mut destination)?;
                    backup.run_to_completion(64, Duration::from_millis(10), None)?;
                }
                destination.close().map_err(|(_, e)| e)?;
                backup_file.keep().map_err(|e| e.error)?;
            }
            let tx = connection.transaction()?;
            if legacy {
                tx.execute_batch("ALTER TABLE focus_sessions ADD COLUMN activity TEXT NOT NULL DEFAULT '{\"project\":null,\"note\":\"\"}'; ALTER TABLE focus_sessions ADD COLUMN end_reason TEXT;")?;
            } else {
                tx.execute_batch("CREATE TABLE focus_sessions (id INTEGER PRIMARY KEY, started_ms INTEGER NOT NULL, checkpoint_ms INTEGER NOT NULL, ended_ms INTEGER, elapsed_ms INTEGER NOT NULL DEFAULT 0 CHECK(elapsed_ms>=0), state TEXT NOT NULL CHECK(state IN ('running','stopped','interrupted')), activity TEXT NOT NULL, end_reason TEXT);")?;
            }
            tx.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS one_running ON focus_sessions((1)) WHERE state='running';
                CREATE TABLE focus_selection(singleton INTEGER PRIMARY KEY CHECK(singleton=1), activity TEXT NOT NULL);
                INSERT INTO focus_selection VALUES(1, '{\"project\":null,\"note\":\"\"}'); PRAGMA user_version=1;")?;
            tx.commit()?;
        }
        if version < 2 {
            if version == 1 {
                let file = tempfile::Builder::new()
                    .prefix("focus-before-v2-")
                    .suffix(".sqlite3")
                    .tempfile_in(
                        path.parent()
                            .ok_or_else(|| invalid("Missing storage directory."))?,
                    )?;
                let mut destination = Connection::open(file.path())?;
                {
                    let backup = rusqlite::backup::Backup::new(&connection, &mut destination)?;
                    backup.run_to_completion(64, Duration::from_millis(10), None)?;
                }
                destination.close().map_err(|(_, e)| e)?;
                file.keep().map_err(|e| e.error)?;
            }
            let tx = connection.transaction()?;
            tx.execute_batch("CREATE TABLE focus_preferences(singleton INTEGER PRIMARY KEY CHECK(singleton=1), split_on_change INTEGER NOT NULL CHECK(split_on_change IN (0,1)));
                INSERT INTO focus_preferences VALUES(1,1);
                CREATE TABLE focus_recent(activity_key TEXT PRIMARY KEY, used_order INTEGER NOT NULL);
                CREATE INDEX focus_recent_order ON focus_recent(used_order);")?;
            {
                let mut statement =
                    tx.prepare("SELECT activity FROM focus_sessions ORDER BY id")?;
                let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
                for text in rows {
                    let activity: Activity = serde_json::from_str(&text?)?;
                    Self::remember(&tx, &activity)?;
                }
            }
            tx.execute_batch("PRAGMA user_version=2;")?;
            tx.commit()?;
        }
        connection.execute("UPDATE focus_sessions SET state='interrupted', ended_ms=checkpoint_ms, end_reason='crash' WHERE state='running'", [])?;
        let store = Self { connection };
        store.snapshot()?;
        Ok(store)
    }
    fn selected(&self) -> Result<Activity> {
        let text: String = self.connection.query_row(
            "SELECT activity FROM focus_selection WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        let activity: Activity = serde_json::from_str(&text)?;
        activity.validate()?;
        Ok(activity)
    }
    fn select(&self, activity: &Activity) -> Result<()> {
        activity.validate()?;
        self.connection.execute(
            "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
            [serde_json::to_string(activity)?],
        )?;
        Ok(())
    }
    fn change_mode(&self) -> Result<ChangeMode> {
        let split: bool = self.connection.query_row(
            "SELECT split_on_change FROM focus_preferences WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        Ok(if split {
            ChangeMode::NewInterval
        } else {
            ChangeMode::EditCurrent
        })
    }
    fn remember(connection: &Connection, activity: &Activity) -> Result<()> {
        activity.validate()?;
        connection.execute("INSERT INTO focus_recent(activity_key,used_order) VALUES(?1,(SELECT COALESCE(MAX(used_order),0)+1 FROM focus_recent)) ON CONFLICT(activity_key) DO UPDATE SET used_order=excluded.used_order", [serde_json::to_string(&activity.key())?])?;
        Ok(())
    }
    fn reassign(&mut self, activity: &Activity, now: i64, elapsed: u64) -> Result<()> {
        activity.validate()?;
        let text = serde_json::to_string(activity)?;
        let tx = self.connection.transaction()?;
        let changed = tx.execute("UPDATE focus_sessions SET activity=?1,checkpoint_ms=?2,elapsed_ms=?3 WHERE state='running' AND started_ms<=?2", params![text, now, bounded(elapsed)])?;
        if changed != 1 {
            return Err(invalid("No active interval can be edited at this time.").into());
        }
        tx.execute(
            "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
            [text],
        )?;
        Self::remember(&tx, activity)?;
        tx.commit()?;
        Ok(())
    }
    fn begin(&mut self, now: i64, activity: &Activity, switching: Option<u64>) -> Result<()> {
        activity.validate()?;
        let text = serde_json::to_string(activity)?;
        let tx = self.connection.transaction()?;
        let review: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM focus_sessions WHERE state='interrupted')",
            [],
            |r| r.get(0),
        )?;
        if review {
            return Err(invalid("Review interrupted time before starting.").into());
        }
        let previous: Option<i64> =
            tx.query_row("SELECT MAX(ended_ms) FROM focus_sessions", [], |r| r.get(0))?;
        if previous.is_some_and(|end| now < end) {
            return Err(invalid("The clock is earlier than saved history. Correct the system clock before starting.").into());
        }
        if let Some(elapsed) = switching {
            let changed = tx.execute("UPDATE focus_sessions SET state='stopped', ended_ms=?1, checkpoint_ms=?1, elapsed_ms=?2, end_reason='switch' WHERE state='running' AND started_ms<=?1", params![now, bounded(elapsed)])?;
            if changed != 1 {
                return Err(invalid("No active session can be switched at this time.").into());
            }
        }
        tx.execute("INSERT INTO focus_sessions(started_ms,checkpoint_ms,state,activity) VALUES(?1,?1,'running',?2)", params![now, text])?;
        tx.execute(
            "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
            [text],
        )?;
        Self::remember(&tx, activity)?;
        tx.commit()?;
        Ok(())
    }
    fn finish(&mut self, now: i64, elapsed: u64, reason: &str) -> Result<()> {
        let tx = self.connection.transaction()?;
        let changed = tx.execute("UPDATE focus_sessions SET state='stopped', ended_ms=?1, checkpoint_ms=?1, elapsed_ms=?2, end_reason=?3 WHERE state='running' AND started_ms<=?1", params![now, bounded(elapsed), reason])?;
        if changed != 1 {
            return Err(invalid("No active session can be stopped at this time.").into());
        }
        tx.commit()?;
        Ok(())
    }
    fn checkpoint(&self, now: i64, elapsed: u64) -> Result<()> {
        self.connection.execute(
            "UPDATE focus_sessions SET checkpoint_ms=?1, elapsed_ms=?2 WHERE state='running'",
            params![now, bounded(elapsed)],
        )?;
        Ok(())
    }
    fn interrupt(&self, reason: &str) -> Result<()> {
        self.connection.execute("UPDATE focus_sessions SET state='interrupted', ended_ms=checkpoint_ms, end_reason=?1 WHERE state='running'", [reason])?;
        Ok(())
    }
    fn accept_recovery(&self) -> Result<()> {
        self.connection.execute(
            "UPDATE focus_sessions SET state='stopped' WHERE state='interrupted'",
            [],
        )?;
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let mut recent = Vec::new();
        let mut statement = self
            .connection
            .prepare("SELECT activity_key,used_order FROM focus_recent ORDER BY used_order DESC")?;
        let rows =
            statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (key, order) = row?;
            recent.push(RecentUse {
                key: serde_json::from_str(&key)?,
                order,
            });
        }
        let active: Option<(i64, i64)> = self
            .connection
            .query_row(
                "SELECT elapsed_ms,started_ms FROM focus_sessions WHERE state='running'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (total, count): (i64,i64) = self.connection.query_row("SELECT COALESCE(SUM(elapsed_ms),0), COUNT(*) FROM focus_sessions WHERE state='stopped'", [], |r| Ok((r.get(0)?,r.get(1)?)))?;
        let review = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM focus_sessions WHERE state='interrupted')",
            [],
            |r| r.get(0),
        )?;
        Ok(Snapshot {
            running: active.is_some(),
            active_start_ms: active.map(|a| a.1),
            elapsed_ms: active.map_or(0, |a| a.0).max(0) as u64,
            total_ms: total.max(0) as u64,
            daily_project_ms: None,
            daily_all_ms: None,
            completed: count.max(0) as u64,
            needs_review: review,
            activity: self.selected()?,
            change_mode: self.change_mode()?,
            recent,
        })
    }

    fn report(
        &self,
        request: &reports::Request,
        now: Moment,
        active_ms: u64,
    ) -> Result<reports::Report> {
        let (range, records) = self.report_records(request, now, Some(active_ms))?;
        Ok(reports::compute(range, records, now.wall_ms))
    }
    fn report_records(
        &self,
        request: &reports::Request,
        now: Moment,
        active_ms: Option<u64>,
    ) -> Result<(reports::Range, Vec<reports::Record>)> {
        let range = reports::Range::new(request)?;
        let mut statement = self.connection.prepare("SELECT id,started_ms,COALESCE(ended_ms,?3),elapsed_ms,state,activity,COALESCE(end_reason,''),checkpoint_ms FROM focus_sessions WHERE started_ms<?2 AND COALESCE(ended_ms,?3)>=?1 ORDER BY id")?;
        let raw =
            statement.query_map(params![range.start_ms, range.end_ms, now.wall_ms], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })?;
        let raw = raw.collect::<std::result::Result<Vec<_>, _>>()?;
        drop(statement);
        // Release the export snapshot's read lock before JSON decoding or output formatting.
        if active_ms.is_none() {
            self.connection.execute_batch("COMMIT")?;
        }
        let mut records = Vec::new();
        for (id, start_ms, mut end_ms, elapsed, state, activity, reason, checkpoint) in raw {
            let state = match state.as_str() {
                "stopped" => reports::State::Saved,
                "running" => reports::State::Running,
                "interrupted" => reports::State::Review,
                _ => return Err(invalid("Unknown history state.").into()),
            };
            let activity: Activity = serde_json::from_str(&activity)?;
            activity.validate()?;
            if state == reports::State::Running && active_ms.is_none() {
                end_ms = checkpoint;
            }
            records.push(reports::Record {
                id,
                start_ms,
                end_ms,
                elapsed_ms: if state == reports::State::Running {
                    active_ms.unwrap_or(elapsed.max(0) as u64)
                } else {
                    elapsed.max(0) as u64
                },
                state,
                activity,
                reason,
            });
        }
        Ok((range, records))
    }
}

/// Read one immutable checkpoint snapshot without migrations, recovery writes or UI row limits.
pub struct ExportSnapshot {
    pub report: reports::Report,
    pub sources: Vec<std::path::PathBuf>,
}
pub fn export_snapshot(path: &Path, request: &reports::Request) -> Result<ExportSnapshot> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(Duration::from_millis(250))?;
    if connection.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? != 2 {
        return Err(invalid("Unsupported history version for export.").into());
    }
    connection.execute_batch("BEGIN")?;
    let _: Option<i64> = connection
        .query_row("SELECT id FROM focus_sessions LIMIT 1", [], |r| r.get(0))
        .optional()?;
    let now = Moment {
        wall_ms: now_ms(),
        mono_ms: 0,
    };
    let store = Store { connection };
    let sources = {
        let mut statement=store.connection.prepare("SELECT DISTINCT CASE WHEN json_valid(activity) THEN json_extract(activity,'$.project.source_path') END FROM focus_sessions")?;
        statement
            .query_map([], |r| r.get::<_, Option<String>>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .map(std::path::PathBuf::from)
            .collect()
    };
    let (range, records) = store.report_records(request, now, None)?;
    Ok(ExportSnapshot {
        report: reports::compute_all(range, records, now.wall_ms),
        sources,
    })
}
fn bounded(ms: u64) -> i64 {
    ms.min(i64::MAX as u64) as i64
}
#[derive(Clone, Copy, Debug)]
pub struct Moment {
    pub wall_ms: i64,
    pub mono_ms: u64,
}
pub struct Engine {
    store: Store,
    started: Option<Moment>,
    observed: Moment,
    checkpoint_mono: u64,
    daily_cache: std::cell::RefCell<Option<DailyTotals>>,
}
struct DailyTotals {
    begin: i64,
    end: i64,
    key: ActivityKey,
    project_ms: u64,
    all_ms: u64,
}
impl Engine {
    pub fn new(store: Store, now: Moment) -> Self {
        Self {
            store,
            started: None,
            observed: now,
            checkpoint_mono: now.mono_ms,
            daily_cache: std::cell::RefCell::new(None),
        }
    }
    pub fn snapshot(&self, now: Moment) -> Result<Snapshot> {
        self.snapshot_in_zone(now, jiff::tz::TimeZone::system())
    }
    fn snapshot_in_zone(&self, now: Moment, zone: jiff::tz::TimeZone) -> Result<Snapshot> {
        let mut snapshot = self.store.snapshot()?;
        if let Some(start) = self.started {
            snapshot.elapsed_ms = now.mono_ms.saturating_sub(start.mono_ms);
        }
        let daily = (|| -> Result<(u64, u64)> {
            let day = jiff::Timestamp::from_millisecond(now.wall_ms)?
                .to_zoned(zone.clone())
                .date();
            let begin = day
                .to_zoned(zone.clone())?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            let end = day
                .tomorrow()?
                .to_zoned(zone)?
                .start_of_day()?
                .timestamp()
                .as_millisecond();
            let mut key = snapshot.activity.key();
            key.task = None;
            let mut cache = self.daily_cache.borrow_mut();
            if cache
                .as_ref()
                .is_none_or(|entry| entry.begin != begin || entry.end != end || entry.key != key)
            {
                let mut total = 0u64;
                let mut all = 0u64;
                let mut query = self.store.connection.prepare("SELECT started_ms,ended_ms,elapsed_ms,activity FROM focus_sessions WHERE state='stopped' AND started_ms<?2 AND ended_ms>?1")?;
                let rows = query.query_map(params![begin, end], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })?;
                for row in rows {
                    let (start_ms, end_ms, elapsed, text) = row?;
                    let activity: Activity = serde_json::from_str(&text)?;
                    let matches_project = activity.key().project == key.project;
                    let allocated = reports::allocated(
                        &reports::Record {
                            id: 0,
                            start_ms,
                            end_ms,
                            elapsed_ms: elapsed.max(0) as u64,
                            state: reports::State::Saved,
                            activity,
                            reason: String::new(),
                        },
                        begin,
                        end,
                    );
                    all = all.saturating_add(allocated);
                    if matches_project {
                        total = total.saturating_add(allocated);
                    }
                }
                *cache = Some(DailyTotals {
                    begin,
                    end,
                    key,
                    project_ms: total,
                    all_ms: all,
                });
            }
            let mut total = cache.as_ref().unwrap().project_ms;
            let mut all = cache.as_ref().unwrap().all_ms;
            if let Some(start) = self.started {
                let live = reports::allocated(
                    &reports::Record {
                        id: 0,
                        start_ms: start.wall_ms,
                        end_ms: now.wall_ms,
                        elapsed_ms: snapshot.elapsed_ms,
                        state: reports::State::Running,
                        activity: snapshot.activity.clone(),
                        reason: String::new(),
                    },
                    begin,
                    end,
                );
                total = total.saturating_add(live);
                all = all.saturating_add(live);
            }
            Ok((total, all))
        })();
        // A broken historical row must not prevent publishing successful Start/Stop state.
        if let Ok((project, all)) = daily {
            snapshot.daily_project_ms = Some(project);
            snapshot.daily_all_ms = Some(all);
        }
        Ok(snapshot)
    }
    /// Returns true only after a successful explicit exit.
    pub fn apply(&mut self, command: Option<Command>, now: Moment) -> Result<bool> {
        if command
            .as_ref()
            .is_some_and(|c| !matches!(c, Command::Report { .. }))
        {
            self.daily_cache.borrow_mut().take();
        }
        let gap = now.mono_ms.saturating_sub(self.observed.mono_ms);
        let drift =
            ((i128::from(now.wall_ms) - i128::from(self.observed.wall_ms)) - i128::from(gap)).abs();
        if self.started.is_some()
            && (gap > 30_000 || drift > 5_000 || now.wall_ms < self.observed.wall_ms)
        {
            self.store.interrupt("clock_or_heartbeat")?;
            self.started = None;
        }
        self.observed = now;
        let elapsed = self
            .started
            .map(|start| now.mono_ms.saturating_sub(start.mono_ms));
        match command {
            Some(Command::Start(activity)) => {
                if self.started.is_some() {
                    return Err(invalid("Tracking is already running.").into());
                }
                self.start(now, &activity, None)?;
            }
            Some(Command::Switch(activity)) => {
                let elapsed =
                    elapsed.ok_or_else(|| invalid("Start tracking before switching focus."))?;
                if self.store.selected()? != activity {
                    self.start(now, &activity, Some(elapsed))?;
                }
            }
            Some(Command::Select(activity)) => {
                if self.started.is_some() {
                    return Err(invalid("Use Switch focus while tracking.").into());
                }
                self.store.select(&activity)?;
            }
            Some(Command::ChangeActivity(activity)) => {
                if let Some(elapsed) = elapsed {
                    if self.store.selected()? != activity {
                        match self.store.change_mode()? {
                            ChangeMode::NewInterval => self.start(now, &activity, Some(elapsed))?,
                            ChangeMode::EditCurrent => {
                                self.store.reassign(&activity, now.wall_ms, elapsed)?;
                                self.checkpoint_mono = now.mono_ms;
                            }
                        }
                    }
                } else {
                    if self.store.snapshot()?.needs_review {
                        return Err(
                            invalid("Review interrupted time before changing activity.").into()
                        );
                    }
                    self.store.select(&activity)?;
                }
            }
            Some(Command::SetChangeMode(mode)) => {
                self.store.connection.execute(
                    "UPDATE focus_preferences SET split_on_change=?1 WHERE singleton=1",
                    [mode == ChangeMode::NewInterval],
                )?;
            }
            Some(Command::Stop) => {
                if let Some(elapsed) = elapsed {
                    self.stop(now, elapsed, "stop")?;
                }
            }
            Some(Command::Toggle) => {
                if let Some(elapsed) = elapsed {
                    self.stop(now, elapsed, "stop")?;
                } else {
                    self.start(now, &self.store.selected()?, None)?;
                }
            }
            Some(Command::Interrupt { reason, .. }) => {
                if let Some(elapsed) = elapsed {
                    self.stop(now, elapsed, reason)?;
                }
            }
            Some(Command::AcceptRecovery) => self.store.accept_recovery()?,
            Some(Command::Exit) => {
                if let Some(elapsed) = elapsed {
                    self.stop(now, elapsed, "exit")?;
                }
                return Ok(true);
            }
            Some(Command::Report { .. }) | None => {
                if let Some(elapsed) = elapsed
                    && now.mono_ms.saturating_sub(self.checkpoint_mono) >= 10_000
                {
                    self.store.checkpoint(now.wall_ms, elapsed)?;
                    self.checkpoint_mono = now.mono_ms;
                }
            }
        }
        Ok(false)
    }
    fn start(&mut self, now: Moment, activity: &Activity, switching: Option<u64>) -> Result<()> {
        self.store.begin(now.wall_ms, activity, switching)?;
        self.started = Some(now);
        self.checkpoint_mono = now.mono_ms;
        Ok(())
    }
    fn stop(&mut self, now: Moment, elapsed: u64, reason: &str) -> Result<()> {
        self.store.finish(now.wall_ms, elapsed, reason)?;
        self.started = None;
        Ok(())
    }
}
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
fn command_moment(command: &Option<Command>, fallback: Moment, boot: Instant) -> Moment {
    match command {
        Some(Command::Interrupt {
            boundary: Some(boundary),
            ..
        }) => Moment {
            wall_ms: boundary.wall_ms,
            mono_ms: boundary
                .instant
                .saturating_duration_since(boot)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        },
        _ => fallback,
    }
}
pub fn spawn(
    path: std::path::PathBuf,
    notify: impl Fn(Event) + Send + 'static,
) -> std::sync::mpsc::SyncSender<Command> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let boot = Instant::now();
        let now = || Moment {
            wall_ms: now_ms(),
            mono_ms: boot.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        };
        let store = match Store::open(&path) {
            Ok(store) => store,
            Err(error) => {
                notify(Event::Error(format!("Cannot open focus history: {error}")));
                while let Ok(command) = receiver.recv() {
                    if matches!(command, Command::Exit) {
                        notify(Event::ExitReady);
                        break;
                    }
                    notify(Event::Error(
                        "Tracking is unavailable. Resolve the storage error and restart.".into(),
                    ));
                }
                return;
            }
        };
        let mut engine = Engine::new(store, now());
        let publish = |engine: &Engine, committed: bool| match engine.snapshot(now()) {
            Ok(state) => notify(if committed {
                Event::Committed(state)
            } else {
                Event::State(state)
            }),
            Err(error) => notify(Event::Error(error.to_string())),
        };
        publish(&engine, false);
        loop {
            let command = if engine.started.is_some() {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(command) => Some(command),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(_) => {
                        let _ = engine.store.interrupt("controller_lost");
                        break;
                    }
                }
            } else {
                match receiver.recv_timeout(Duration::from_secs(30)) {
                    Ok(command) => Some(command),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(_) => break,
                }
            };
            let completion = match &command {
                Some(Command::Interrupt { done, .. }) => done.clone(),
                _ => None,
            };
            // Preserve the event boundary even if the worker resumes after a short sleep.
            let moment = command_moment(&command, now(), boot);
            let report_request = match &command {
                Some(Command::Report { id, request }) => Some((*id, request.clone())),
                _ => None,
            };
            let committed = command.is_some() && report_request.is_none();
            let succeeded = match engine.apply(command, moment) {
                Ok(true) => {
                    notify(Event::ExitReady);
                    break;
                }
                Ok(false) => {
                    if let Some(done) = completion {
                        let _ = done.send(());
                    }
                    if let Some((id, request)) = report_request {
                        let result = engine
                            .snapshot(moment)
                            .and_then(|state| {
                                engine.store.report(&request, moment, state.elapsed_ms)
                            })
                            .map(Box::new)
                            .map_err(|error| format!("Cannot load report: {error}"));
                        notify(Event::Report { id, result });
                    }
                    true
                }
                Err(error) => {
                    let message = format!("History was not updated: {error}");
                    if let Some((id, _)) = report_request {
                        notify(Event::Report {
                            id,
                            result: Err(message.clone()),
                        });
                    }
                    notify(Event::Error(message));
                    false
                }
            };
            publish(&engine, committed && succeeded);
        }
    });
    sender
}

#[cfg(test)]
mod tests {
    use super::*;
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
        Engine::new(Store::open(&dir.join("focus.sqlite3")).unwrap(), at(0))
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
            Connection::open_with_flags(backup, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
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
}
