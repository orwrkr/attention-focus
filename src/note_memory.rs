//! Profile-local note suggestions, updated only from confirmed activities.
use crate::tracking::{Activity, LegacyProject, Project};
use eframe::egui;
use serde::{Deserialize, Serialize};

/// Preference key since project IDs replaced the former source identity.
pub const STORAGE_KEY: &str = "project_notes_v2";
/// Preference key written by older versions, converted once and then removed.
pub const LEGACY_STORAGE_KEY: &str = "project_notes";

#[derive(Default, Serialize, Deserialize)]
pub struct NoteMemory {
    projects: Vec<Entry>,
    #[serde(skip)]
    popup_open: bool,
}
#[derive(Serialize, Deserialize)]
struct Entry {
    key: Option<i64>,
    last: String,
    recent: Vec<String>,
}
/// Note memory as saved by older versions.
#[derive(Deserialize)]
pub struct LegacyNotes {
    projects: Vec<LegacyEntry>,
}
#[derive(Deserialize)]
struct LegacyEntry {
    key: Option<LegacyProject>,
    last: String,
    recent: Vec<String>,
}
fn key(project: &Option<Project>) -> Option<i64> {
    project.as_ref().map(|p| p.id)
}
impl NoteMemory {
    /// Keep notes whose project still exists. Notes already recorded under the new key win.
    pub fn import_legacy(&mut self, legacy: LegacyNotes, aliases: &[(LegacyProject, i64)]) {
        for entry in legacy.projects {
            let key = match &entry.key {
                None => None,
                Some(identity) => match aliases.iter().find(|(known, _)| known == identity) {
                    Some((_, id)) => Some(*id),
                    None => continue,
                },
            };
            if !self.projects.iter().any(|existing| existing.key == key) {
                self.projects.push(Entry {
                    key,
                    last: entry.last,
                    recent: entry.recent,
                });
            }
        }
    }
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
            id: 1,
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
        activity.project.as_mut().unwrap().id = 2;
        assert_eq!(memory.last(&activity.project), "");
        let restored: NoteMemory =
            serde_json::from_str(&serde_json::to_string(&memory).unwrap()).unwrap();
        assert_eq!(restored.projects[0].recent, memory.projects[0].recent);
    }
    #[test]
    fn notes_saved_by_older_versions_follow_their_converted_project() {
        // Mirrors the fields older versions saved, through the same preference storage.
        #[derive(Serialize)]
        struct OldMemory {
            projects: Vec<OldEntry>,
        }
        #[derive(Serialize)]
        struct OldEntry {
            key: Option<(i64, String, String)>,
            last: String,
            recent: Vec<String>,
        }
        #[derive(Default)]
        struct Memory(std::collections::HashMap<String, String>);
        impl eframe::Storage for Memory {
            fn get_string(&self, key: &str) -> Option<String> {
                self.0.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.0.insert(key.into(), value);
            }
            fn remove_string(&mut self, key: &str) {
                self.0.remove(key);
            }
            fn flush(&mut self) {}
        }
        let identity = |slug: &str| Some((1, "synthetic-source".to_owned(), slug.to_owned()));
        let entry = |key, last: &str, recent: &[&str]| OldEntry {
            key,
            last: last.to_owned(),
            recent: recent.iter().map(|note| (*note).to_owned()).collect(),
        };
        let mut storage = Memory::default();
        eframe::set_value(
            &mut storage,
            LEGACY_STORAGE_KEY,
            &OldMemory {
                projects: vec![
                    entry(
                        identity("alpha"),
                        "Old \u{1f331}",
                        &["Old \u{1f331}", "Older"],
                    ),
                    entry(identity("removed"), "Lost", &["Lost"]),
                    entry(None, "Loose", &["Loose"]),
                ],
            },
        );
        let legacy: LegacyNotes = eframe::get_value(&storage, LEGACY_STORAGE_KEY).unwrap();
        let mut memory = NoteMemory::default();
        let mut current = Activity {
            project: Some(Project {
                id: 7,
                title: "Alpha".into(),
                task: None,
            }),
            note: String::new(),
        };
        let aliases = [((1, "synthetic-source".to_owned(), "alpha".to_owned()), 7)];
        memory.import_legacy(legacy, &aliases);
        assert_eq!(memory.last(&current.project), "Old \u{1f331}");
        assert_eq!(memory.suggestions(&current), ["Old \u{1f331}", "Older"]);
        assert_eq!(memory.last(&None), "Loose");
        assert_eq!(memory.projects.len(), 2, "unknown projects are dropped");
        // A note confirmed by this version is never replaced by an older copy.
        current.note = "Newer".into();
        memory.remember(&current);
        let again: LegacyNotes = eframe::get_value(&storage, LEGACY_STORAGE_KEY).unwrap();
        memory.import_legacy(again, &aliases);
        assert_eq!(memory.last(&current.project), "Newer");
    }
}
