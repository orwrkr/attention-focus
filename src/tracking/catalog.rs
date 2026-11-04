//! The application's own project and task list. Finished sessions keep the titles they
//! recorded, so anything with recorded time can be archived or completed but not deleted.
use super::{Activity, Result, Store, invalid, reject};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};

const MAX_TITLE_CHARS: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogTask {
    pub title: String,
    pub done: bool,
    /// Recorded in at least one session, so it cannot be deleted.
    pub used: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogProject {
    pub id: i64,
    pub title: String,
    pub archived: bool,
    /// Recorded in at least one session, so it cannot be deleted.
    pub used: bool,
    pub tasks: Vec<CatalogTask>,
}

/// Titles keep their language and characters; only surrounding space and control characters
/// that cannot be shown on one line are removed.
fn clean_title(text: &str) -> Result<String> {
    let title: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let title = title.trim();
    if title.is_empty() {
        return Err(reject("errors.enter_name").into());
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(invalid(&crate::i18n::message(
            "errors.name_too_long",
            &[("limit", &MAX_TITLE_CHARS.to_string())],
        ))
        .into());
    }
    Ok(title.to_owned())
}
/// Titles that differ only by case would be hard to tell apart in the pickers.
fn same_title(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}
fn project_titles(connection: &Connection) -> Result<Vec<(i64, String)>> {
    let mut statement = connection.prepare("SELECT id,title FROM focus_projects")?;
    let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}
fn require_project(connection: &Connection, id: i64) -> Result<()> {
    let found: Option<i64> = connection
        .query_row("SELECT id FROM focus_projects WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    found
        .map(|_| ())
        .ok_or_else(|| reject("errors.project_missing").into())
}
/// Rewrite one stored activity when it belongs to the project.
fn retitled(text: &str, id: i64, title: &str) -> Result<Option<String>> {
    let mut activity: Activity = serde_json::from_str(text)?;
    match activity.project.as_mut() {
        Some(project) if project.id == id => {
            project.title = title.to_owned();
            Ok(Some(serde_json::to_string(&activity)?))
        }
        _ => Ok(None),
    }
}

impl Store {
    pub(super) fn catalog(&self) -> Result<Vec<CatalogProject>> {
        // A malformed historical row must not hide the list; it is reported by the report.
        let mut used = HashSet::<(i64, Option<String>)>::new();
        let mut statement = self.connection.prepare("SELECT DISTINCT json_extract(activity,'$.project.id'),CASE WHEN json_type(activity,'$.project.task')='text' THEN json_extract(activity,'$.project.task') END FROM focus_sessions WHERE CASE WHEN json_valid(activity) THEN json_type(activity,'$.project.id')='integer' ELSE 0 END")?;
        let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        for row in rows {
            used.insert(row?);
        }
        let mut tasks = HashMap::<i64, Vec<CatalogTask>>::new();
        let mut statement = self
            .connection
            .prepare("SELECT project_id,title,done FROM focus_tasks ORDER BY id")?;
        let rows = statement.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?;
        for row in rows {
            let (project, title, done) = row?;
            let used = used.contains(&(project, Some(title.clone())));
            tasks
                .entry(project)
                .or_default()
                .push(CatalogTask { title, done, used });
        }
        let mut statement = self
            .connection
            .prepare("SELECT id,title,archived FROM focus_projects")?;
        let rows = statement.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?;
        let mut projects = Vec::new();
        for row in rows {
            let (id, title, archived) = row?;
            projects.push(CatalogProject {
                id,
                title,
                archived,
                used: used.iter().any(|(project, _)| *project == id),
                tasks: tasks.remove(&id).unwrap_or_default(),
            });
        }
        projects.sort_by_cached_key(|p| (p.title.to_lowercase(), p.id));
        Ok(projects)
    }
    /// New intervals cannot use what the list marks as archived or finished, even if a queued
    /// command was decided before that change. Only the interface can name other activities.
    pub(super) fn check_startable(&self, activity: &Activity) -> Result<()> {
        let Some(project) = &activity.project else {
            return Ok(());
        };
        let archived: Option<bool> = self
            .connection
            .query_row(
                "SELECT archived FROM focus_projects WHERE id=?1",
                [project.id],
                |r| r.get(0),
            )
            .optional()?;
        if archived == Some(true) {
            return Err(reject("errors.project_archived").into());
        }
        if let Some(task) = &project.task {
            let done: Option<bool> = self
                .connection
                .query_row(
                    "SELECT done FROM focus_tasks WHERE project_id=?1 AND title=?2",
                    params![project.id, task],
                    |r| r.get(0),
                )
                .optional()?;
            if done == Some(true) {
                return Err(reject("errors.task_finished").into());
            }
        }
        Ok(())
    }
    pub(super) fn current_titles(&self) -> Result<HashMap<i64, String>> {
        Ok(project_titles(&self.connection)?.into_iter().collect())
    }
    /// Existing names are skipped, so a pasted list can repeat some of them. Adding only
    /// names that already exist is an error, so a single addition does not silently vanish.
    pub(super) fn add_projects(&mut self, titles: &[String]) -> Result<()> {
        let titles = titles
            .iter()
            .map(|title| clean_title(title))
            .collect::<Result<Vec<_>>>()?;
        let tx = self.connection.transaction()?;
        let mut known: Vec<String> = project_titles(&tx)?
            .into_iter()
            .map(|(_, title)| title)
            .collect();
        let mut added = 0;
        for title in titles {
            if !known.iter().any(|other| same_title(other, &title)) {
                tx.execute("INSERT INTO focus_projects(title) VALUES(?1)", [&title])?;
                known.push(title);
                added += 1;
            }
        }
        if added == 0 {
            return Err(reject("errors.project_exists").into());
        }
        tx.commit()?;
        Ok(())
    }
    /// The selection and a running interval follow the new title. Finished sessions keep theirs.
    pub(super) fn rename_project(&mut self, id: i64, title: &str) -> Result<()> {
        let title = clean_title(title)?;
        let tx = self.connection.transaction()?;
        require_project(&tx, id)?;
        if project_titles(&tx)?
            .iter()
            .any(|(other, name)| *other != id && same_title(name, &title))
        {
            return Err(reject("errors.project_name_taken").into());
        }
        tx.execute(
            "UPDATE focus_projects SET title=?1 WHERE id=?2",
            params![title, id],
        )?;
        let selected: String = tx.query_row(
            "SELECT activity FROM focus_selection WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        if let Some(text) = retitled(&selected, id, &title)? {
            tx.execute(
                "UPDATE focus_selection SET activity=?1 WHERE singleton=1",
                [text],
            )?;
        }
        let running: Option<String> = tx
            .query_row(
                "SELECT activity FROM focus_sessions WHERE state='running'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(running) = running
            && let Some(text) = retitled(&running, id, &title)?
        {
            tx.execute(
                "UPDATE focus_sessions SET activity=?1 WHERE state='running'",
                [text],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    /// A running interval may finish on an archived project; new ones cannot start on it.
    pub(super) fn set_project_archived(&self, id: i64, archived: bool) -> Result<()> {
        require_project(&self.connection, id)?;
        self.connection.execute(
            "UPDATE focus_projects SET archived=?1 WHERE id=?2",
            params![archived, id],
        )?;
        Ok(())
    }
    pub(super) fn delete_project(&mut self, id: i64) -> Result<()> {
        let catalog = self.catalog()?;
        let project = catalog
            .iter()
            .find(|project| project.id == id)
            .ok_or_else(|| reject("errors.project_missing"))?;
        if project.used {
            return Err(reject("errors.project_used").into());
        }
        if self.selected()?.project.is_some_and(|p| p.id == id) {
            return Err(reject("errors.project_selected").into());
        }
        // Tasks are removed with the project through their foreign key.
        self.connection
            .execute("DELETE FROM focus_projects WHERE id=?1", [id])?;
        Ok(())
    }
    /// As for projects, existing names are skipped. A finished task is reopened instead.
    pub(super) fn add_tasks(&mut self, project: i64, titles: &[String]) -> Result<()> {
        let titles = titles
            .iter()
            .map(|title| clean_title(title))
            .collect::<Result<Vec<_>>>()?;
        let tx = self.connection.transaction()?;
        require_project(&tx, project)?;
        let mut known: Vec<(String, bool)> = {
            let mut statement =
                tx.prepare("SELECT title,done FROM focus_tasks WHERE project_id=?1")?;
            let rows = statement.query_map([project], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let mut changed = 0;
        for title in titles {
            match known
                .iter_mut()
                .find(|(other, _)| same_title(other, &title))
            {
                Some((other, done)) if *done => {
                    tx.execute(
                        "UPDATE focus_tasks SET done=0 WHERE project_id=?1 AND title=?2",
                        params![project, other.as_str()],
                    )?;
                    *done = false;
                    changed += 1;
                }
                Some(_) => {}
                None => {
                    tx.execute(
                        "INSERT INTO focus_tasks(project_id,title) VALUES(?1,?2)",
                        params![project, title],
                    )?;
                    known.push((title, false));
                    changed += 1;
                }
            }
        }
        if changed == 0 {
            return Err(reject("errors.task_exists").into());
        }
        tx.commit()?;
        Ok(())
    }
    /// A running interval may finish on a completed task; new ones cannot start on it.
    pub(super) fn set_task_done(&self, project: i64, title: &str, done: bool) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE focus_tasks SET done=?1 WHERE project_id=?2 AND title=?3",
            params![done, project, title],
        )?;
        if changed != 1 {
            return Err(reject("errors.task_missing").into());
        }
        Ok(())
    }
    pub(super) fn delete_task(&mut self, project: i64, title: &str) -> Result<()> {
        let catalog = self.catalog()?;
        let task = catalog
            .iter()
            .find(|entry| entry.id == project)
            .and_then(|entry| entry.tasks.iter().find(|task| task.title == title))
            .ok_or_else(|| reject("errors.task_missing"))?;
        if task.used {
            return Err(reject("errors.task_used").into());
        }
        if self
            .selected()?
            .project
            .is_some_and(|p| p.id == project && p.task.as_deref() == Some(title))
        {
            return Err(reject("errors.task_selected").into());
        }
        self.connection.execute(
            "DELETE FROM focus_tasks WHERE project_id=?1 AND title=?2",
            params![project, title],
        )?;
        Ok(())
    }
}
