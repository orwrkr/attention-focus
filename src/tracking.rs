//! Durable unassigned focus sessions. All production access is owned by one worker.

use rusqlite::{Connection, OptionalExtension, params};
use std::{
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub running: bool,
    pub elapsed_ms: u64,
    pub total_ms: u64,
    pub completed: u64,
    pub needs_review: bool,
}

pub enum Command {
    Toggle,
    AcceptRecovery,
    Exit,
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
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS focus_sessions (
                id INTEGER PRIMARY KEY,
                started_ms INTEGER NOT NULL,
                checkpoint_ms INTEGER NOT NULL,
                ended_ms INTEGER,
                elapsed_ms INTEGER NOT NULL DEFAULT 0 CHECK(elapsed_ms >= 0),
                state TEXT NOT NULL CHECK(state IN ('running', 'stopped', 'interrupted'))
            );
            CREATE UNIQUE INDEX IF NOT EXISTS one_running ON focus_sessions((1)) WHERE state='running';")?;
        // Never count the interval between the last checkpoint and the next launch.
        connection.execute("UPDATE focus_sessions SET state='interrupted', ended_ms=checkpoint_ms WHERE state='running'", [])?;
        Ok(Self { connection })
    }

    pub fn start(&self, now_ms: i64) -> rusqlite::Result<()> {
        if self.snapshot()?.needs_review {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.connection.execute("INSERT INTO focus_sessions(started_ms, checkpoint_ms, state) VALUES (?1, ?1, 'running')", [now_ms])?;
        Ok(())
    }

    pub fn checkpoint(&self, now_ms: i64, elapsed_ms: u64) -> rusqlite::Result<()> {
        self.connection.execute(
            "UPDATE focus_sessions SET checkpoint_ms=?1, elapsed_ms=?2 WHERE state='running'",
            params![now_ms, elapsed_ms.min(i64::MAX as u64) as i64],
        )?;
        Ok(())
    }

    pub fn stop(&mut self, now_ms: i64, elapsed_ms: u64) -> rusqlite::Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("UPDATE focus_sessions SET checkpoint_ms=?1, ended_ms=?1, elapsed_ms=?2, state='stopped' WHERE state='running'", params![now_ms, elapsed_ms.min(i64::MAX as u64) as i64])?;
        tx.commit()
    }

    pub fn interrupt(&self) -> rusqlite::Result<()> {
        self.connection.execute("UPDATE focus_sessions SET state='interrupted', ended_ms=checkpoint_ms WHERE state='running'", [])?;
        Ok(())
    }

    pub fn accept_recovery(&self) -> rusqlite::Result<()> {
        self.connection.execute(
            "UPDATE focus_sessions SET state='stopped' WHERE state='interrupted'",
            [],
        )?;
        Ok(())
    }

    pub fn snapshot(&self) -> rusqlite::Result<Snapshot> {
        let active: Option<i64> = self
            .connection
            .query_row(
                "SELECT elapsed_ms FROM focus_sessions WHERE state='running'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let (total_ms, completed): (i64, i64) = self.connection.query_row("SELECT COALESCE(SUM(elapsed_ms),0), COUNT(*) FROM focus_sessions WHERE state='stopped'", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let needs_review = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM focus_sessions WHERE state='interrupted')",
            [],
            |r| r.get(0),
        )?;
        Ok(Snapshot {
            running: active.is_some(),
            elapsed_ms: active.unwrap_or(0).max(0) as u64,
            total_ms: total_ms.max(0) as u64,
            completed: completed.max(0) as u64,
            needs_review,
        })
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

/// Spawn one SQLite owner; notify only after committed state changes.
pub fn spawn(
    path: std::path::PathBuf,
    notify: impl Fn(Event) + Send + 'static,
) -> std::sync::mpsc::SyncSender<Command> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let mut store = match Store::open(&path) {
            Ok(store) => store,
            Err(error) => {
                notify(Event::Error(format!("Cannot open focus history: {error}")));
                // Keep Exit working when storage initialization failed.
                while let Ok(command) = receiver.recv() {
                    if matches!(command, Command::Exit) {
                        notify(Event::ExitReady);
                        break;
                    }
                    notify(Event::Error(
                        "Tracking is unavailable. Restart after resolving the storage error."
                            .into(),
                    ));
                }
                return;
            }
        };
        let mut started: Option<Instant> = None;
        let mut last_tick = Instant::now();
        let mut last_wall = now_ms();
        let mut last_checkpoint = Instant::now();
        let publish = |store: &Store, started: Option<Instant>| match store.snapshot() {
            Ok(mut state) => {
                if let Some(start) = started {
                    state.elapsed_ms = start.elapsed().as_millis() as u64;
                }
                notify(Event::State(state));
            }
            Err(error) => notify(Event::Error(error.to_string())),
        };
        publish(&store, started);
        loop {
            let command = if started.is_some() {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(command) => Some(command),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(_) => {
                        let _ = store.interrupt();
                        break;
                    }
                }
            } else {
                match receiver.recv() {
                    Ok(command) => Some(command),
                    Err(_) => break,
                }
            };
            let current_wall = now_ms();
            let gap = last_tick.elapsed();
            let clock_jump =
                ((current_wall - last_wall) - gap.as_millis() as i64).unsigned_abs() > 5_000;
            if started.is_some() && (gap > Duration::from_secs(30) || clock_jump) {
                if let Err(error) = store.interrupt() {
                    notify(Event::Error(format!("Cannot save interruption: {error}")));
                    continue;
                }
                started = None;
            }
            last_tick = Instant::now();
            last_wall = current_wall;
            let result = match command {
                Some(Command::Toggle) => {
                    if let Some(start) = started {
                        store
                            .stop(current_wall, start.elapsed().as_millis() as u64)
                            .map(|()| started = None)
                    } else if store.snapshot().is_ok_and(|state| state.needs_review) {
                        notify(Event::Error(
                            "Review the interrupted session before starting another.".into(),
                        ));
                        publish(&store, started);
                        continue;
                    } else {
                        store.start(current_wall).map(|()| {
                            started = Some(Instant::now());
                            last_checkpoint = Instant::now();
                        })
                    }
                }
                Some(Command::AcceptRecovery) => store.accept_recovery(),
                Some(Command::Exit) => {
                    let result = if let Some(start) = started {
                        store.stop(current_wall, start.elapsed().as_millis() as u64)
                    } else {
                        Ok(())
                    };
                    match result {
                        Ok(()) => {
                            notify(Event::ExitReady);
                            break;
                        }
                        Err(error) => {
                            notify(Event::Error(format!("Cannot save before exit: {error}")));
                            continue;
                        }
                    }
                }
                None => {
                    if let Some(start) =
                        started.filter(|_| last_checkpoint.elapsed() >= Duration::from_secs(10))
                    {
                        store
                            .checkpoint(current_wall, start.elapsed().as_millis() as u64)
                            .map(|()| last_checkpoint = Instant::now())
                    } else {
                        Ok(())
                    }
                }
            };
            if let Err(error) = result {
                notify(Event::Error(format!("History was not updated: {error}")));
            }
            publish(&store, started);
        }
    });
    sender
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_time_survives_restart_without_counting_downtime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("focus.sqlite3");
        let mut store = Store::open(&path).unwrap();
        store.start(1_000).unwrap();
        store.stop(6_000, 5_000).unwrap();
        drop(store);
        let state = Store::open(&path).unwrap().snapshot().unwrap();
        assert!(!state.running);
        assert_eq!(state.total_ms, 5_000);
        assert_eq!(state.completed, 1);
    }

    #[test]
    fn interrupted_session_requires_review_and_uses_only_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("focus.sqlite3");
        let store = Store::open(&path).unwrap();
        store.start(1_000).unwrap();
        store.checkpoint(11_000, 10_000).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        assert!(store.snapshot().unwrap().needs_review);
        assert!(store.start(100_000).is_err());
        store.accept_recovery().unwrap();
        assert_eq!(store.snapshot().unwrap().total_ms, 10_000);
        assert!(!store.snapshot().unwrap().running);
    }

    #[test]
    fn database_rejects_a_second_running_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        store.start(1_000).unwrap();
        assert!(store.start(2_000).is_err());
        assert!(store.snapshot().unwrap().running);
    }

    #[test]
    fn failed_stop_preserves_running_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("focus.sqlite3")).unwrap();
        store.start(1_000).unwrap();
        store
            .connection
            .execute_batch("PRAGMA query_only=ON;")
            .unwrap();
        assert!(store.stop(2_000, 1_000).is_err());
        assert!(store.snapshot().unwrap().running);
        assert_eq!(store.snapshot().unwrap().completed, 0);
    }

    #[test]
    fn worker_toggle_and_exit_commit_before_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("focus.sqlite3");
        let (events, receiver) = std::sync::mpsc::channel();
        let commands = spawn(path.clone(), move |event| {
            let _ = events.send(event);
        });
        assert!(matches!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Event::State(Snapshot { running: false, .. })
        ));
        commands.send(Command::Toggle).unwrap();
        assert!(matches!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Event::State(Snapshot { running: true, .. })
        ));
        commands.send(Command::Exit).unwrap();
        assert!(matches!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Event::ExitReady
        ));
        let state = Store::open(&path).unwrap().snapshot().unwrap();
        assert!(!state.running);
        assert!(!state.needs_review);
        assert_eq!(state.completed, 1);
    }
}
