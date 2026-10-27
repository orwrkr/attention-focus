//! Responsive focus counters with measured frame margins and one native egui hint.
use crate::{presentation::duration, tracking::Snapshot};
use eframe::egui::{self, Rect};

pub struct Response {
    pub toggle: bool,
    pub bounds: [Rect; 3],
}

pub fn show(ui: &mut egui::Ui, state: &Snapshot, ready: bool, enabled: bool) -> Response {
    let available = ui.available_width();
    let gap = 12.0;
    let frame = egui::Frame::group(ui.style()).inner_margin(8.0);
    let margin = frame.total_margin().sum().x;
    let wide = available >= 600.0;
    let columns = if wide { 3.0 } else { 2.0 };
    let width = (available - gap * (columns - 1.0)) / columns - margin;
    let mut toggle = false;
    let mut bounds = [Rect::NOTHING; 3];
    let mut card = |ui: &mut egui::Ui, index: usize, width: f32| {
        bounds[index] = frame
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.set_min_height(120.0);
                    ui.strong(
                        [
                            "Current interval",
                            "Today for project",
                            "Today - all projects",
                        ][index],
                    );
                    let value = [
                        Some(state.elapsed_ms),
                        state.daily_project_ms,
                        state.daily_all_ms,
                    ][index];
                    if let Some(value) = value {
                        ui.label(egui::RichText::new(duration(value)).size(32.0).monospace());
                    } else {
                        ui.label("Unavailable");
                    }
                    if index == 0 {
                        ui.label(if !ready {
                            "Opening history..."
                        } else if state.running {
                            "Tracking"
                        } else {
                            "Stopped"
                        });
                        toggle |= ui
                            .add_enabled(
                                enabled,
                                egui::Button::new(
                                    egui::RichText::new(if state.running {
                                        "Stop tracking"
                                    } else {
                                        "Start tracking"
                                    })
                                    .strong()
                                    .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(31, 91, 151))
                                .min_size(egui::vec2(ui.available_width(), 32.0)),
                            )
                            .clicked();
                    } else {
                        ui.add(
                            egui::Label::new(if index == 1 {
                                state.activity.label()
                            } else {
                                "Includes unassigned focus"
                            })
                            .truncate(),
                        );
                        ui.weak("All tasks, local day");
                    }
                });
            })
            .response
            .rect;
    };
    if wide {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for index in 0..3 {
                card(ui, index, width);
            }
        });
    } else {
        card(ui, 0, available - margin);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            card(ui, 1, width);
            card(ui, 2, width);
        });
    }
    Response { toggle, bounds }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reserved_scrollbar_and_mirrored_gutter_keep_window_edge_margins_equal() {
        for width in [420.0, 620.0, 760.0] {
            let context = egui::Context::default();
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 500.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::Frame::central_panel(ui.style()).show(ui, |ui| {
                        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                        let gutter = ui.spacing().scroll.allocated_width() as i8;
                        egui::ScrollArea::vertical()
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                            )
                            .content_margin(egui::Margin {
                                left: gutter,
                                ..egui::Margin::ZERO
                            })
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let response = show(ui, &Snapshot::default(), true, false);
                                let left = response.bounds[0].left();
                                let right = width - response.bounds[2].right();
                                assert!(
                                    (left - right).abs() < 0.6,
                                    "asymmetric gutters at {width}: {left}, {right}"
                                );
                                ui.add_space(1000.0);
                            });
                    });
                },
            );
            output.textures_delta.clear();
        }
    }
    #[test]
    fn counter_frames_fit_and_share_both_content_edges_at_narrow_and_wide_sizes() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            for width in [404.0, 604.0, 744.0] {
                let context = egui::Context::default();
                context.set_pixels_per_point(scale);
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 800.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let edges = ui.available_rect_before_wrap();
                        let response = show(ui, &Snapshot::default(), true, false);
                        for rect in response.bounds {
                            assert!(
                                rect.left() >= edges.left() - 0.5,
                                "left overflow at {width}"
                            );
                            assert!(
                                rect.right() <= edges.right() + 0.5,
                                "right overflow at {width}"
                            );
                            assert!(rect.height() >= 108.0);
                        }
                        assert!((response.bounds[0].left() - edges.left()).abs() < 0.5);
                        assert!((response.bounds[2].right() - edges.right()).abs() < 0.5);
                        assert!((response.bounds[1].top() - response.bounds[2].top()).abs() < 0.5);
                    },
                );
                output.textures_delta.clear();
            }
        }
    }

    #[test]
    fn truncated_project_name_has_exactly_one_full_text_tooltip() {
        let context = egui::Context::default();
        let name = "Example project with a deliberately long title for checking a single tooltip";
        let state = Snapshot {
            activity: crate::tracking::Activity {
                project: Some(crate::tracking::Project {
                    source_id: 1,
                    source_path: "synthetic".into(),
                    slug: "example".into(),
                    title: name.into(),
                    task: None,
                }),
                note: String::new(),
            },
            ..Default::default()
        };
        let mut point = None;
        let mut full_hints = 0;
        for time in [0.0, 1.0, 2.0, 3.0] {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(404.0, 800.0),
                    )),
                    events: point
                        .filter(|_| time == 1.0)
                        .map(|p| vec![egui::Event::PointerMoved(p)])
                        .unwrap_or_default(),
                    ..Default::default()
                },
                |ui| {
                    show(ui, &state, true, false);
                },
            );
            full_hints = 0;
            for shape in &output.shapes {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.job.text == name
                {
                    if text.galley.elided {
                        point = Some(text.pos + egui::vec2(5.0, 5.0));
                    } else {
                        full_hints += 1;
                    }
                }
            }
            output.textures_delta.clear();
        }
        assert!(point.is_some());
        assert_eq!(full_hints, 1);
    }
}
