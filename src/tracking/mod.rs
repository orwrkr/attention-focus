//! Serialized, clock-injected tracking with durable activity snapshots.
use crate::reports;
use serde::{Deserialize, Serialize};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

mod catalog;
mod engine;
mod migrate;
mod store;
mod worker;

pub use catalog::{CatalogProject, CatalogTask};
pub use engine::Engine;
pub use migrate::LegacyProject;
pub use store::{Store, export_snapshot};
pub use worker::spawn;

pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn invalid(message: &str) -> std::io::Error {
    std::io::Error::other(message)
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub project: Option<Project>,
    pub note: String,
}
/// A session's activity: the project ID identifies it, while the title and task text stay
/// as recorded, so later renames do not rewrite finished history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub title: String,
    pub task: Option<String>,
}
impl Activity {
    fn key(&self) -> ActivityKey {
        ActivityKey {
            project: self.project.as_ref().map(|p| p.id),
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
            || self
                .project
                .as_ref()
                .is_some_and(|p| p.id <= 0 || p.title.is_empty())
        {
            return Err(invalid("Invalid activity snapshot.").into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ActivityKey {
    project: Option<i64>,
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
    pub split_on_day: bool,
    pub recent: Vec<RecentUse>,
    pub projects: Vec<CatalogProject>,
    /// Earlier project identities, for converting preferences saved by older versions.
    pub legacy_projects: Vec<(LegacyProject, i64)>,
}
impl Snapshot {
    pub fn project(&self, id: i64) -> Option<&CatalogProject> {
        self.projects.iter().find(|project| project.id == id)
    }
    /// Unassigned focus is always available; otherwise the project must be listed and not
    /// archived, and a chosen task must be listed and not done.
    pub fn available(&self, activity: &Activity) -> bool {
        let Some(chosen) = &activity.project else {
            return true;
        };
        self.project(chosen.id).is_some_and(|project| {
            !project.archived
                && chosen.task.as_ref().is_none_or(|title| {
                    project
                        .tasks
                        .iter()
                        .any(|task| !task.done && task.title == *title)
                })
        })
    }
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
    SetSplitOnDay(bool),
    AddProjects(Vec<String>),
    RenameProject {
        id: i64,
        title: String,
    },
    SetProjectArchived {
        id: i64,
        archived: bool,
    },
    DeleteProject(i64),
    AddTasks {
        project: i64,
        titles: Vec<String>,
    },
    SetTaskDone {
        project: i64,
        title: String,
        done: bool,
    },
    DeleteTask {
        project: i64,
        title: String,
    },
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
    /// Current state, from a timer tick, a report or a Windows interruption.
    State(Snapshot),
    /// A command from the interface succeeded.
    Committed(Snapshot),
    Report {
        id: u64,
        result: std::result::Result<Box<reports::Report>, String>,
    },
    /// A command from the interface failed.
    Rejected(String),
    /// Background work failed; no interface command is affected.
    Error(String),
    ExitReady,
}
fn bounded(ms: u64) -> i64 {
    ms.min(i64::MAX as u64) as i64
}
#[derive(Clone, Copy, Debug)]
pub struct Moment {
    pub wall_ms: i64,
    pub mono_ms: u64,
}
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests;
