//! Explicit schema versions. Every upgrade backs up an existing database first and applies
//! its changes in one transaction.
use super::{Activity, Result, Store, invalid};
use rusqlite::Connection;
use std::{path::Path, time::Duration};

pub(super) const SCHEMA_VERSION: i64 = 3;

/// Copy the database beside itself through SQLite's backup API before an upgrade.
fn backup_before_upgrade(connection: &Connection, path: &Path, prefix: &str) -> Result<()> {
    let file = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".sqlite3")
        .tempfile_in(
            path.parent()
                .ok_or_else(|| invalid("Missing storage directory."))?,
        )?;
    let mut destination = Connection::open(file.path())?;
    {
        let backup = rusqlite::backup::Backup::new(connection, &mut destination)?;
        backup.run_to_completion(64, Duration::from_millis(10), None)?;
    }
    destination.close().map_err(|(_, e)| e)?;
    file.keep().map_err(|e| e.error)?;
    Ok(())
}

pub(super) fn upgrade(connection: &mut Connection, path: &Path) -> Result<()> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(invalid("History was created by a newer application version.").into());
    }
    if version == 0 {
        let legacy: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='focus_sessions')", [], |r| r.get(0))?;
        if legacy {
            backup_before_upgrade(connection, path, "focus-before-v1-")?;
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
            backup_before_upgrade(connection, path, "focus-before-v2-")?;
        }
        let tx = connection.transaction()?;
        tx.execute_batch("CREATE TABLE focus_preferences(singleton INTEGER PRIMARY KEY CHECK(singleton=1), split_on_change INTEGER NOT NULL CHECK(split_on_change IN (0,1)));
            INSERT INTO focus_preferences VALUES(1,1);
            CREATE TABLE focus_recent(activity_key TEXT PRIMARY KEY, used_order INTEGER NOT NULL);
            CREATE INDEX focus_recent_order ON focus_recent(used_order);")?;
        {
            let mut statement = tx.prepare("SELECT activity FROM focus_sessions ORDER BY id")?;
            let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
            for text in rows {
                let activity: Activity = serde_json::from_str(&text?)?;
                Store::remember(&tx, &activity)?;
            }
        }
        tx.execute_batch("PRAGMA user_version=2;")?;
        tx.commit()?;
    }
    if version < 3 {
        if version == 2 {
            backup_before_upgrade(connection, path, "focus-before-v3-")?;
        }
        let tx = connection.transaction()?;
        tx.execute_batch("ALTER TABLE focus_preferences ADD COLUMN split_on_day INTEGER NOT NULL DEFAULT 1 CHECK(split_on_day IN (0,1));
            PRAGMA user_version=3;")?;
        tx.commit()?;
    }
    Ok(())
}
