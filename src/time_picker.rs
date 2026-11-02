//! Minute-precision civil time picker; range validation remains in reports.
use eframe::egui;

#[derive(Default)]
pub struct TimePicker {
    draft: Option<(u8, u8)>,
}
fn parse(text: &str, end: bool) -> Option<(u8, u8)> {
    let (hour, minute) = text.split_once(':')?;
    let (hour, minute) = (hour.parse::<u8>().ok()?, minute.parse::<u8>().ok()?);
    ((hour < 24 && minute < 60) || (end && hour == 24 && minute == 0)).then_some((hour, minute))
}
impl TimePicker {
    pub fn show(&mut self, ui: &mut egui::Ui, text: &mut String, id: &str, end: bool) -> bool {
        let mut changed = false;
        let mut open = false;
        ui.push_id(id, |ui| {
            ui.horizontal(|ui| {
                changed |= ui
                    .add(egui::TextEdit::singleline(text).desired_width(70.0))
                    .changed();
                egui::containers::menu::MenuButton::new("Time")
                    .config(
                        egui::containers::menu::MenuConfig::default()
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                    )
                    .ui(ui, |ui| {
                        open = true;
                        let (hour, minute) = self
                            .draft
                            .get_or_insert_with(|| parse(text, end).unwrap_or((0, 0)));
                        ui.horizontal(|ui| {
                            ui.label("Hour");
                            ui.add(egui::DragValue::new(hour).range(0..=if end { 24 } else { 23 }));
                            ui.label("Minute");
                            if *hour == 24 {
                                *minute = 0;
                            }
                            ui.add_enabled(*hour < 24, egui::DragValue::new(minute).range(0..=59));
                        });
                        if end && ui.button("End of day (24:00)").clicked() {
                            *text = "24:00".into();
                            changed = true;
                            ui.close();
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() {
                                *text = format!("{hour:02}:{minute:02}");
                                changed = true;
                                ui.close();
                            }
                            if ui.button("Cancel").clicked() {
                                ui.close();
                            }
                        });
                    });
            });
        });
        if !open || changed {
            self.draft = None;
        }
        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn popup_cancel_preserves_text_and_end_of_day_applies() {
        let context = egui::Context::default();
        let mut picker = TimePicker::default();
        let mut text = "10:15".to_owned();
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
                |ui| {
                    picker.show(ui, &mut text, "test-time", true);
                },
            );
            output.textures_delta.clear();
            output
        };
        fn position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + egui::vec2(5.0, 5.0))
                    }
                    _ => None,
                })
                .expect("Picker control must be visible")
        }
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
        for label in ["Time", "10", "Cancel", "Time", "End of day (24:00)"] {
            frame(vec![]);
            let output = frame(vec![]);
            let pos = position(&output, label);
            frame(pointer(pos, true));
            frame(pointer(pos, false));
        }
        assert_eq!(text, "24:00");
    }
    #[test]
    fn minute_limits_and_end_of_day_are_explicit() {
        assert_eq!(parse("23:59", false), Some((23, 59)));
        assert_eq!(parse("24:00", true), Some((24, 0)));
        for text in ["24:00", "23:60", "24:01", "-1:00", "invalid"] {
            assert_eq!(parse(text, false), None);
        }
        assert_eq!(parse("24:01", true), None);
    }
}
