//! Local focus tracking with an application-owned project list, and desktop assets.

pub mod bidi;
pub mod charts;
pub mod counters;
pub mod export;
pub mod fonts;
pub mod i18n;
pub mod icons;
pub mod interval_dialog;
#[cfg(windows)]
pub mod lifecycle;
pub mod motion;
pub mod note_memory;
pub mod notices;
pub mod outbox;
pub mod paths;
#[cfg(windows)]
pub mod platform;
pub mod profiles;
pub mod project_ui;
pub mod report_ui;
pub mod reports;
#[cfg(windows)]
pub mod startup;
#[cfg(windows)]
pub mod startup_ui;
pub mod time_picker;
pub mod tracking;
pub mod tray_interaction;
pub mod ui_support;

/// The project site, linked from About. The application never fetches it: a click hands the
/// address to the default browser.
pub const SITE: &str = env!("CARGO_PKG_HOMEPAGE");

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

    #[test]
    fn the_site_is_the_pages_address_of_the_repository() {
        // GitHub Pages serves a repository `owner/name` at `https://owner.github.io/name/`.
        let repository = env!("CARGO_PKG_REPOSITORY");
        let (owner, name) = repository
            .strip_prefix("https://github.com/")
            .and_then(|path| path.split_once('/'))
            .unwrap();
        assert_eq!(super::SITE, format!("https://{owner}.github.io/{name}/"));
    }
}
pub mod date_picker;
pub mod presentation;
