//! Profile-local note suggestions, updated only from confirmed activities.
use crate::tracking::{Activity, Project};
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct NoteMemory {
    projects: Vec<Entry>,
    #[serde(skip)]
    popup_open: bool,
}
#[derive(Serialize, Deserialize)]
struct Entry {
    key: Option<(i64, String, String)>,
    last: String,
    recent: Vec<String>,
}
fn key(project: &Option<Project>) -> Option<(i64, String, String)> {
    project
        .as_ref()
        .map(|p| (p.source_id, p.source_path.clone(), p.slug.clone()))
}
impl NoteMemory {
    pub fn remember(&mut self, activity: &Activity) -> bool {
        let key = key(&activity.project);
        let index = self.projects.iter().position(|e| e.key == key);
        let entry = match index {
            Some(index) => &mut self.projects[index],
            None => {
                self.projects.push(Entry {
                    key,
                    last: activity.note.clone(),
                    recent: Vec::new(),
                });
                self.projects.last_mut().unwrap()
            }
        };
        let changed = index.is_none() || entry.last != activity.note;
        if changed {
            entry.last = activity.note.clone();
            if !activity.note.trim().is_empty() {
                entry.recent.retain(|note| note != &activity.note);
                entry.recent.insert(0, activity.note.clone());
                entry.recent.truncate(8);
            }
        }
        changed
    }
    pub fn last(&self, project: &Option<Project>) -> String {
        self.projects
            .iter()
            .find(|e| e.key == key(project))
            .map(|e| e.last.clone())
            .unwrap_or_default()
    }
    pub fn suggestions(&self, activity: &Activity) -> Vec<String> {
        let query = activity.note.to_lowercase();
        self.projects
            .iter()
            .find(|e| e.key == key(&activity.project))
            .map(|e| {
                e.recent
                    .iter()
                    .filter(|note| note.to_lowercase().contains(&query) && *note != &activity.note)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn show(&mut self, ui: &mut egui::Ui, activity: &mut Activity) {
        let response = ui
            .add(egui::TextEdit::singleline(&mut activity.note).hint_text("Optional session note"));
        let suggestions = self.suggestions(activity);
        if response.gained_focus() || response.changed() || response.clicked() {
            self.popup_open = true;
        }
        if suggestions.is_empty() {
            self.popup_open = false;
        }
        egui::Popup::from_response(&response)
            .open_bool(&mut self.popup_open)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .width(response.rect.width())
            .show(|ui| {
                ui.weak("Recent notes for this project");
                for note in suggestions {
                    if ui.selectable_label(false, &note).clicked() {
                        activity.note = note;
                        ui.close();
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suggestion_survives_focus_loss_until_pointer_release() {
        let context = egui::Context::default();
        let mut memory = NoteMemory::default();
        let mut activity = Activity {
            note: "Planning".into(),
            ..Default::default()
        };
        memory.remember(&activity);
        activity.note.clear();
        let mut frame = |events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 400.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| memory.show(ui, &mut activity),
            );
            output.textures_delta.clear();
            output
        };
        fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ]
        }
        frame(vec![]);
        frame(pointer(egui::pos2(30.0, 12.0), true));
        frame(pointer(egui::pos2(30.0, 12.0), false));
        let output = frame(vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Planning" => {
                    Some(text.pos + egui::vec2(8.0, 5.0))
                }
                _ => None,
            })
            .expect("Focused note field must show its suggestion");
        frame(pointer(position, true));
        frame(pointer(position, false));
        assert_eq!(activity.note, "Planning");
    }
    #[test]
    fn notes_are_bounded_deduplicated_and_isolated_by_project_not_task() {
        let mut memory = NoteMemory::default();
        let mut activity = Activity::default();
        for n in 0..10 {
            activity.note = format!("Note {n}");
            assert!(memory.remember(&activity));
        }
        assert!(!memory.remember(&activity));
        activity.note = "Note 4".into();
        memory.remember(&activity);
        activity.note.clear();
        memory.remember(&activity);
        assert_eq!(memory.last(&None), "");
        let suggestions = memory.suggestions(&activity);
        assert_eq!(suggestions.len(), 8);
        assert_eq!(suggestions[0], "Note 4");
        assert!(!suggestions.contains(&"Note 0".into()));
        activity.project = Some(Project {
            source_id: 1,
            source_path: "synthetic".into(),
            slug: "alpha".into(),
            title: "Alpha".into(),
            task: None,
        });
        assert!(memory.suggestions(&activity).is_empty());
        activity.note = "Planning".into();
        memory.remember(&activity);
        activity.project.as_mut().unwrap().task = Some("Review".into());
        activity.project.as_mut().unwrap().title = "Renamed".into();
        assert_eq!(memory.last(&activity.project), "Planning");
        activity.note = "PLAN".into();
        assert_eq!(memory.suggestions(&activity), ["Planning"]);
        activity.project.as_mut().unwrap().source_id = 2;
        assert_eq!(memory.last(&activity.project), "");
        let restored: NoteMemory =
            serde_json::from_str(&serde_json::to_string(&memory).unwrap()).unwrap();
        assert_eq!(restored.projects[0].recent, memory.projects[0].recent);
    }
}
