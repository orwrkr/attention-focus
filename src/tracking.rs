//! Serialized, clock-injected tracking with durable activity snapshots.
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
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub running: bool,
    pub elapsed_ms: u64,
    pub total_ms: u64,
    pub completed: u64,
    pub needs_review: bool,
    pub activity: Activity,
}
pub enum Command {
    Toggle,
    Start(Activity),
    Stop,
    Switch(Activity),
    Select(Activity),
    AcceptRecovery,
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
        if version > 1 {
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
        let active: Option<i64> = self
            .connection
            .query_row(
                "SELECT elapsed_ms FROM focus_sessions WHERE state='running'",
                [],
                |r| r.get(0),
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
            elapsed_ms: active.unwrap_or(0).max(0) as u64,
            total_ms: total.max(0) as u64,
            completed: count.max(0) as u64,
            needs_review: review,
            activity: self.selected()?,
        })
    }
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
}
impl Engine {
    pub fn new(store: Store, now: Moment) -> Self {
        Self {
            store,
            started: None,
            observed: now,
            checkpoint_mono: now.mono_ms,
        }
    }
    pub fn snapshot(&self, now: Moment) -> Result<Snapshot> {
        let mut snapshot = self.store.snapshot()?;
        if let Some(start) = self.started {
            snapshot.elapsed_ms = now.mono_ms.saturating_sub(start.mono_ms);
        }
        Ok(snapshot)
    }
    /// Returns true only after a successful explicit exit.
    pub fn apply(&mut self, command: Option<Command>, now: Moment) -> Result<bool> {
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
            None => {
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
        let publish = |engine: &Engine| match engine.snapshot(now()) {
            Ok(state) => notify(Event::State(state)),
            Err(error) => notify(Event::Error(error.to_string())),
        };
        publish(&engine);
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
                match receiver.recv() {
                    Ok(command) => Some(command),
                    Err(_) => break,
                }
            };
            let completion = match &command {
                Some(Command::Interrupt { done, .. }) => done.clone(),
                _ => None,
            };
            // Preserve the event boundary even if the worker resumes after a short sleep.
            let moment = command_moment(&command, now(), boot);
            match engine.apply(command, moment) {
                Ok(true) => {
                    notify(Event::ExitReady);
                    break;
                }
                Ok(false) => {
                    if let Some(done) = completion {
                        let _ = done.send(());
                    }
                }
                Err(error) => notify(Event::Error(format!("History was not updated: {error}"))),
            }
            publish(&engine);
        }
    });
    sender
}

#[cfg(test)]
mod tests {
    use super::*;
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
            Event::State(Snapshot { running: true, .. })
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
            Event::State(Snapshot { running: false, .. })
        ));
        commands.send(Command::Start(activity("Next"))).unwrap();
        assert!(matches!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Event::State(Snapshot { running: true, .. })
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
}
