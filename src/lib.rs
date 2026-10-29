//! Local focus tracking, read-only MetaPlan snapshots, and desktop assets.

pub mod counters;
pub mod export;
#[cfg(windows)]
pub mod folder_picker;
pub mod icons;
#[cfg(windows)]
pub mod lifecycle;
pub mod motion;
#[cfg(windows)]
pub mod platform;
pub mod profiles;
pub mod report_ui;
pub mod reports;
pub mod source;
#[cfg(windows)]
pub mod startup;
#[cfg(windows)]
pub mod startup_ui;
pub mod tracking;
pub mod tray_interaction;
pub mod ui_support;

/// Verify bundled SQLite persistence without leaving application data behind.
pub fn sqlite_probe() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("probe.sqlite3");
    {
        let mut connection = rusqlite::Connection::open(&path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA synchronous = FULL;
            CREATE TABLE probe (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        let transaction = connection.transaction()?;
        transaction.execute("INSERT INTO probe(value) VALUES (?1)", ["Storage ready"])?;
        transaction.commit()?;
        connection.close().map_err(|(_, error)| error)?;
    }
    let connection =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let value: String =
        connection.query_row("SELECT value FROM probe WHERE id = 1", [], |row| row.get(0))?;
    connection.close().map_err(|(_, error)| error)?;
    directory.close()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn sqlite_survives_close_and_reopen() {
        assert_eq!(super::sqlite_probe().unwrap(), "Storage ready");
    }
}
pub mod date_picker;
pub mod presentation;
