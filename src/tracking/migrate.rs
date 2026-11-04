//! Explicit schema versions. Every upgrade backs up an existing database first and applies
//! its changes in one transaction.
use super::{Activity, ActivityKey, Project, Result, invalid};
use rusqlite::{Connection, Transaction, params};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, time::Duration};

pub(super) const SCHEMA_VERSION: i64 = 4;

/// Project identity before schema four: source ID, source path and source folder name.
pub type LegacyProject = (i64, String, String);

/// Activity snapshot format before schema four.
#[derive(Deserialize)]
struct LegacyActivity {
    project: Option<LegacySnapshot>,
    note: String,
}
#[derive(Deserialize)]
struct LegacySnapshot {
    source_id: i64,
    source_path: String,
    slug: String,
    title: String,
    task: Option<String>,
}
impl LegacySnapshot {
    fn identity(&self) -> LegacyProject {
        (self.source_id, self.source_path.clone(), self.slug.clone())
    }
}
/// Recent-use key format before schema four.
#[derive(Serialize, Deserialize)]
struct LegacyKey {
    project: Option<LegacyProject>,
    task: Option<String>,
}

/// Turns snapshots from the former external source into application projects, in the order
/// they were recorded, so each project ends up with its newest recorded title.
struct ProjectConversion<'a> {
    tx: &'a Transaction<'a>,
    ids: HashMap<LegacyProject, i64>,
}
impl ProjectConversion<'_> {
    fn activity(&mut self, text: &str) -> Result<String> {
        let legacy: LegacyActivity = serde_json::from_str(text)?;
        let project = match legacy.project {
            None => None,
            Some(snapshot) => {
                let identity = snapshot.identity();
                let id = match self.ids.get(&identity) {
                    Some(&id) => {
                        self.tx.execute(
                            "UPDATE focus_projects SET title=?1 WHERE id=?2",
                            params![snapshot.title, id],
                        )?;
                        id
                    }
                    None => {
                        self.tx.execute(
                            "INSERT INTO focus_projects(title) VALUES(?1)",
                            [&snapshot.title],
                        )?;
                        let id = self.tx.last_insert_rowid();
                        self.tx.execute(
                            "INSERT INTO focus_project_aliases(legacy_key,project_id) VALUES(?1,?2)",
                            params![serde_json::to_string(&identity)?, id],
                        )?;
                        self.ids.insert(identity, id);
                        id
                    }
                };
                if let Some(task) = &snapshot.task {
                    self.tx.execute(
                        "INSERT OR IGNORE INTO focus_tasks(project_id,title) VALUES(?1,?2)",
                        params![id, task],
                    )?;
                }
                Some(Project {
                    id,
                    title: snapshot.title,
                    task: snapshot.task,
                })
            }
        };
        let activity = Activity {
            project,
            note: legacy.note,
        };
        activity.validate()?;
        Ok(serde_json::to_string(&activity)?)
    }
}

fn upgrade_to_application_projects(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch("CREATE TABLE focus_projects(id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)));
        CREATE TABLE focus_tasks(id INTEGER PRIMARY KEY AUTOINCREMENT, project_id INTEGER NOT NULL REFERENCES focus_projects(id) ON DELETE CASCADE, title TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0 CHECK(done IN (0,1)), UNIQUE(project_id,title));
        CREATE TABLE focus_project_aliases(legacy_key TEXT PRIMARY KEY, project_id INTEGER NOT NULL REFERENCES focus_projects(id));
        CREATE INDEX focus_sessions_ended ON focus_sessions(ended_ms);")?;
    let mut conversion = ProjectConversion {
        tx,
        ids: HashMap::new(),
    };
    let sessions: Vec<(i64, String)> = {
        let mut statement = tx.prepare("SELECT id,activity FROM focus_sessions ORDER BY id")?;
        let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    for (id, text) in sessions {
        tx.execute(
            "UPDATE focus_sessions SET activity=?1 WHERE id=?2",
            params![conversion.activity(&text)?, id],
        )?;
    }
    let selected: String = tx.query_row(
        "SELECT activity FROM focus_selection WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    tx.execute(
        "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
        [conversion.activity(&selected)?],
    )?;
    let recent: Vec<(String, i64)> = {
        let mut statement = tx.prepare("SELECT activity_key,used_order FROM focus_recent")?;
        let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    tx.execute_batch("DELETE FROM focus_recent;")?;
    for (text, order) in recent {
        let legacy: LegacyKey = serde_json::from_str(&text)?;
        let project = match &legacy.project {
            None => None,
            Some(identity) => match conversion.ids.get(identity) {
                Some(&id) => Some(id),
                // A corrected interval can leave behind a name with no recorded session.
                None => continue,
            },
        };
        let key = ActivityKey {
            project,
            task: legacy.task,
        };
        tx.execute("INSERT INTO focus_recent(activity_key,used_order) VALUES(?1,?2) ON CONFLICT(activity_key) DO UPDATE SET used_order=MAX(used_order,excluded.used_order)", params![serde_json::to_string(&key)?, order])?;
    }
    tx.execute_batch("PRAGMA user_version=4;")?;
    Ok(())
}

/// Earlier identities let preferences saved by older versions find their project.
pub(super) fn legacy_projects(connection: &Connection) -> Result<Vec<(LegacyProject, i64)>> {
    let mut statement =
        connection.prepare("SELECT legacy_key,project_id FROM focus_project_aliases")?;
    let rows = statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    let mut aliases = Vec::new();
    for row in rows {
        let (key, project) = row?;
        aliases.push((serde_json::from_str(&key)?, project));
    }
    Ok(aliases)
}

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
                let activity: LegacyActivity = serde_json::from_str(&text?)?;
                let key = LegacyKey {
                    task: activity.project.as_ref().and_then(|p| p.task.clone()),
                    project: activity.project.as_ref().map(LegacySnapshot::identity),
                };
                tx.execute("INSERT INTO focus_recent(activity_key,used_order) VALUES(?1,(SELECT COALESCE(MAX(used_order),0)+1 FROM focus_recent)) ON CONFLICT(activity_key) DO UPDATE SET used_order=excluded.used_order", [serde_json::to_string(&key)?])?;
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
    if version < 4 {
        if version == 3 {
            backup_before_upgrade(connection, path, "focus-before-v4-")?;
        }
        let tx = connection.transaction()?;
        upgrade_to_application_projects(&tx)?;
        tx.commit()?;
    }
    Ok(())
}
