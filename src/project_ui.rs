//! Controls for the application's own project and task list; changes go through the worker.
use crate::{
    i18n::{message, paragraph, t, tf, tooltip, user},
    tracking::{CatalogProject, Command},
};
use eframe::egui;
use std::collections::HashMap;

/// One name per line, so a whole list can be pasted at once.
fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Text sent to the worker, kept until the result in case it has to come back.
enum Submitted {
    Projects(String),
    Tasks(i64, String),
    Rename(i64, String),
}

#[derive(Default)]
pub struct Panel {
    new_projects: String,
    new_tasks: HashMap<i64, String>,
    /// Only names being edited; every other row shows the project's current title.
    renames: HashMap<i64, String>,
    show_archived: bool,
    submitted: Option<Submitted>,
}

impl Panel {
    pub fn committed(&mut self) {
        self.submitted = None;
    }

    /// A refused change gives its text back, unless a new draft has already started.
    pub fn rejected(&mut self) {
        match self.submitted.take() {
            Some(Submitted::Projects(text)) if self.new_projects.is_empty() => {
                self.new_projects = text;
            }
            Some(Submitted::Tasks(project, text)) => {
                let draft = self.new_tasks.entry(project).or_default();
                if draft.is_empty() {
                    *draft = text;
                }
            }
            Some(Submitted::Rename(project, text)) => {
                self.renames.entry(project).or_insert(text);
            }
            _ => {}
        }
    }

    /// Returns the changes the user asked for during this frame.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        projects: &[CatalogProject],
        enabled: bool,
    ) -> Vec<Command> {
        let mut commands = Vec::new();
        ui.add_enabled_ui(enabled, |ui| {
            let wrap = ui.available_width();
            ui.add(
                egui::TextEdit::multiline(&mut self.new_projects)
                    .hint_text(t("projects.new_project_hint"))
                    .desired_rows(2)
                    .desired_width(wrap),
            );
            let names = lines(&self.new_projects);
            if ui
                .add_enabled(
                    !names.is_empty(),
                    egui::Button::new(t("projects.add_projects")),
                )
                .clicked()
            {
                commands.push(Command::AddProjects(names));
                self.submitted = Some(Submitted::Projects(std::mem::take(&mut self.new_projects)));
            }
            let archived = projects.iter().filter(|p| p.archived).count();
            if archived > 0 {
                ui.checkbox(
                    &mut self.show_archived,
                    tf(
                        "projects.show_archived",
                        &[("count", &archived.to_string())],
                    ),
                );
            }
            ui.separator();
            if projects.is_empty() {
                paragraph(ui, &message("projects.none_yet", &[]));
            }
            for project in projects {
                if project.archived && !self.show_archived {
                    continue;
                }
                ui.push_id(project.id, |ui| self.project(ui, project, &mut commands));
            }
        });
        commands
    }

    fn project(&mut self, ui: &mut egui::Ui, project: &CatalogProject, out: &mut Vec<Command>) {
        let open = project.tasks.iter().filter(|task| !task.done).count();
        let title = match (project.archived, open) {
            (true, _) => tf("projects.archived_title", &[("name", &project.title)]),
            (false, 0) => user(&project.title).into_owned(),
            (false, count) => tf(
                "projects.open_tasks_title",
                &[("name", &project.title), ("count", &count.to_string())],
            ),
        };
        egui::CollapsingHeader::new(title)
            .id_salt("details")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut draft = self
                        .renames
                        .get(&project.id)
                        .cloned()
                        .unwrap_or_else(|| project.title.clone());
                    if ui
                        .add(egui::TextEdit::singleline(&mut draft).desired_width(200.0))
                        .changed()
                    {
                        self.renames.insert(project.id, draft.clone());
                    }
                    let changed = draft.trim() != project.title && !draft.trim().is_empty();
                    let rename = ui.add_enabled(changed, egui::Button::new(t("projects.rename")));
                    if tooltip(rename, &message("projects.rename_hint", &[])).clicked() {
                        out.push(Command::RenameProject {
                            id: project.id,
                            title: draft.clone(),
                        });
                        self.renames.remove(&project.id);
                        self.submitted = Some(Submitted::Rename(project.id, draft));
                    }
                });
                let (done, unfinished): (Vec<_>, Vec<_>) =
                    project.tasks.iter().partition(|task| task.done);
                for task in unfinished.into_iter().chain(done) {
                    ui.horizontal(|ui| {
                        let mut checked = task.done;
                        let label = if task.done {
                            egui::RichText::new(user(&task.title).as_ref()).strikethrough()
                        } else {
                            egui::RichText::new(user(&task.title).as_ref())
                        };
                        if ui.checkbox(&mut checked, label).changed() {
                            out.push(Command::SetTaskDone {
                                project: project.id,
                                title: task.title.clone(),
                                done: checked,
                            });
                        }
                        if !task.used
                            && tooltip(
                                ui.small_button(t("projects.delete")),
                                &message("projects.delete_hint", &[]),
                            )
                            .clicked()
                        {
                            out.push(Command::DeleteTask {
                                project: project.id,
                                title: task.title.clone(),
                            });
                        }
                    });
                }
                if !project.archived {
                    let draft = self.new_tasks.entry(project.id).or_default();
                    ui.add(
                        egui::TextEdit::multiline(draft)
                            .hint_text(t("projects.new_task_hint"))
                            .desired_rows(1)
                            .desired_width(ui.available_width()),
                    );
                    let names = lines(draft);
                    if ui
                        .add_enabled(
                            !names.is_empty(),
                            egui::Button::new(t("projects.add_tasks")),
                        )
                        .clicked()
                    {
                        out.push(Command::AddTasks {
                            project: project.id,
                            titles: names,
                        });
                        self.submitted = Some(Submitted::Tasks(project.id, std::mem::take(draft)));
                    }
                }
                ui.horizontal(|ui| {
                    let archive = ui.button(if project.archived {
                        t("projects.restore")
                    } else {
                        t("projects.archive")
                    });
                    if tooltip(archive, &message("projects.archive_hint", &[])).clicked() {
                        out.push(Command::SetProjectArchived {
                            id: project.id,
                            archived: !project.archived,
                        });
                    }
                    if !project.used
                        && tooltip(
                            ui.button(t("projects.delete")),
                            &message("projects.delete_hint", &[]),
                        )
                        .clicked()
                    {
                        out.push(Command::DeleteProject(project.id));
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracking::CatalogTask;

    fn project(id: i64, title: &str, used: bool, archived: bool) -> CatalogProject {
        CatalogProject {
            id,
            title: title.into(),
            archived,
            used,
            tasks: vec![
                CatalogTask {
                    title: "Open task".into(),
                    done: false,
                    used: false,
                },
                CatalogTask {
                    title: "Done task".into(),
                    done: true,
                    used: true,
                },
            ],
        }
    }
    struct Harness {
        context: egui::Context,
        panel: Panel,
        projects: Vec<CatalogProject>,
        enabled: bool,
    }
    impl Harness {
        fn new(projects: Vec<CatalogProject>) -> Self {
            Self {
                context: egui::Context::default(),
                panel: Panel::default(),
                projects,
                enabled: true,
            }
        }
        fn frame(&mut self, events: Vec<egui::Event>) -> (egui::FullOutput, Vec<Command>) {
            let mut commands = Vec::new();
            let (panel, projects, enabled) = (&mut self.panel, &self.projects, self.enabled);
            let mut output = self.context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| commands = panel.show(ui, projects, enabled),
            );
            output.textures_delta.clear();
            (output, commands)
        }
        /// Sections expand with an animation, so let it finish before reading the layout.
        fn settled(&mut self) -> egui::FullOutput {
            for _ in 0..30 {
                self.frame(vec![]);
            }
            self.frame(vec![]).0
        }
        fn position(&mut self, label: &str) -> Option<egui::Pos2> {
            let output = self.settled();
            output.shapes.iter().find_map(|s| match &s.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.pos + egui::vec2(3.0, 3.0))
                }
                _ => None,
            })
        }
        fn visible(&mut self, label: &str) -> bool {
            self.position(label).is_some()
        }
        fn click(&mut self, label: &str) -> Vec<Command> {
            let pos = self
                .position(label)
                .unwrap_or_else(|| panic!("{label} must be visible"));
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            self.frame(vec![egui::Event::PointerMoved(pos), button(true)]);
            let (_, commands) = self.frame(vec![egui::Event::PointerMoved(pos), button(false)]);
            commands
        }
    }

    #[test]
    fn lines_keep_every_language_and_skip_blank_rows() {
        assert_eq!(
            lines("  Alpha \n\n\u{41f}\u{440}\u{43e}\u{435}\u{43a}\u{442}\r\n \n\u{1f331} Gamma"),
            [
                "Alpha",
                "\u{41f}\u{440}\u{43e}\u{435}\u{43a}\u{442}",
                "\u{1f331} Gamma"
            ]
        );
        assert!(lines(" \n\r\n").is_empty());
    }

    #[test]
    fn pasted_names_become_one_addition_and_clear_the_draft() {
        let mut harness = Harness::new(vec![]);
        assert!(harness.visible("No projects yet. You can still track Unassigned focus."));
        assert!(harness.click("Add projects").is_empty(), "nothing to add");
        harness.panel.new_projects = "Alpha\n\nBeta\n".into();
        match harness.click("Add projects").as_slice() {
            [Command::AddProjects(titles)] => assert_eq!(titles, &["Alpha", "Beta"]),
            _ => panic!("Expected one addition"),
        }
        assert!(harness.panel.new_projects.is_empty());
    }

    #[test]
    fn expanded_project_offers_rename_tasks_and_archive() {
        let mut harness = Harness::new(vec![project(5, "Alpha", false, false)]);
        harness.click("Alpha (1)");
        assert!(
            harness.click("Rename").is_empty(),
            "the name has not changed"
        );
        harness.panel.renames.insert(5, "  ".into());
        assert!(
            harness.click("Rename").is_empty(),
            "a blank name is not sent"
        );
        harness.panel.renames.insert(5, "Renamed".into());
        match harness.click("Rename").as_slice() {
            [Command::RenameProject { id: 5, title }] => assert_eq!(title, "Renamed"),
            _ => panic!("Expected a rename"),
        }
        harness.panel.new_tasks.insert(5, "One\nTwo".into());
        match harness.click("Add tasks").as_slice() {
            [Command::AddTasks { project: 5, titles }] => assert_eq!(titles, &["One", "Two"]),
            _ => panic!("Expected new tasks"),
        }
        assert!(harness.panel.new_tasks[&5].is_empty());
        match harness.click("Open task").as_slice() {
            [
                Command::SetTaskDone {
                    project: 5,
                    title,
                    done: true,
                },
            ] => {
                assert_eq!(title, "Open task");
            }
            _ => panic!("Expected the task to be marked done"),
        }
        match harness.click("Done task").as_slice() {
            [
                Command::SetTaskDone {
                    project: 5,
                    title,
                    done: false,
                },
            ] => {
                assert_eq!(title, "Done task");
            }
            _ => panic!("Expected the task to be reopened"),
        }
        assert!(matches!(
            harness.click("Archive").as_slice(),
            [Command::SetProjectArchived {
                id: 5,
                archived: true
            }]
        ));
    }

    #[test]
    fn rename_box_follows_the_saved_title_and_cannot_send_the_old_name_back() {
        let mut harness = Harness::new(vec![project(5, "Alpha", false, false)]);
        harness.click("Alpha (1)");
        harness.panel.renames.insert(5, "Renamed".into());
        assert_eq!(harness.click("Rename").len(), 1);
        // Frames keep running before the worker confirms, without keeping the old name.
        harness.settled();
        assert!(harness.panel.renames.is_empty());
        harness.projects[0].title = "Renamed".into();
        harness.panel.committed();
        assert!(harness.visible("Renamed"));
        assert!(
            harness.click("Rename").is_empty(),
            "the box shows the saved name, so there is nothing to send"
        );
    }

    #[test]
    fn refused_changes_give_their_text_back() {
        let mut harness = Harness::new(vec![project(5, "Alpha", false, false)]);
        harness.panel.new_projects = "One\nTwo".into();
        assert_eq!(harness.click("Add projects").len(), 1);
        assert!(harness.panel.new_projects.is_empty());
        harness.panel.rejected();
        assert_eq!(harness.panel.new_projects, "One\nTwo");
        // A success forgets the text, so a later refusal cannot bring it back.
        assert_eq!(harness.click("Add projects").len(), 1);
        harness.panel.committed();
        harness.panel.rejected();
        assert!(harness.panel.new_projects.is_empty());
        harness.click("Alpha (1)");
        harness.panel.new_tasks.insert(5, "Task".into());
        assert_eq!(harness.click("Add tasks").len(), 1);
        harness.panel.rejected();
        assert_eq!(harness.panel.new_tasks[&5], "Task");
        harness.panel.renames.insert(5, "Taken name".into());
        assert_eq!(harness.click("Rename").len(), 1);
        harness.panel.rejected();
        assert_eq!(harness.panel.renames[&5], "Taken name");
        // A refusal does not replace what the user has started typing since.
        harness.panel.submitted = Some(Submitted::Projects("Old".into()));
        harness.panel.new_projects = "Newer".into();
        harness.panel.rejected();
        assert_eq!(harness.panel.new_projects, "Newer");
    }

    #[test]
    fn delete_is_offered_only_for_what_was_never_tracked() {
        let mut harness = Harness::new(vec![project(5, "Alpha", true, false)]);
        harness.click("Alpha (1)");
        // Only the task without recorded time offers a Delete button.
        let output = harness.settled();
        let deletes = output
            .shapes
            .iter()
            .filter(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == "Delete"))
            .count();
        assert_eq!(deletes, 1);
        let mut unused = Harness::new(vec![project(6, "Beta", false, false)]);
        unused.click("Beta (1)");
        let output = unused.settled();
        let deletes = output
            .shapes
            .iter()
            .filter(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == "Delete"))
            .count();
        assert_eq!(deletes, 2);
    }

    #[test]
    fn archived_projects_stay_hidden_until_requested_and_can_be_restored() {
        let mut harness = Harness::new(vec![
            project(5, "Alpha", true, true),
            project(6, "Beta", false, false),
        ]);
        assert!(!harness.visible("Alpha (Archived)"));
        assert!(harness.visible("Beta (1)"));
        harness.click("Show archived projects (1)");
        assert!(harness.visible("Alpha (Archived)"));
        harness.click("Alpha (Archived)");
        assert!(
            !harness.visible("Add tasks"),
            "archived projects take no new tasks"
        );
        assert!(matches!(
            harness.click("Restore").as_slice(),
            [Command::SetProjectArchived {
                id: 5,
                archived: false
            }]
        ));
    }

    #[test]
    fn disabled_panel_sends_nothing_while_a_change_is_saving() {
        let mut harness = Harness::new(vec![project(5, "Alpha", false, false)]);
        harness.click("Alpha (1)");
        harness.enabled = false;
        assert!(harness.click("Archive").is_empty());
    }
}
