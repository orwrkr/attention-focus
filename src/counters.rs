//! Responsive focus counters with measured frame margins and one native egui hint.
use crate::{
    i18n::{self, t},
    presentation::duration,
    tracking::Snapshot,
};
use eframe::egui::{self, Rect};
use std::borrow::Cow;

pub struct Response {
    pub toggle: bool,
    pub bounds: [Rect; 3],
}

pub fn show(ui: &mut egui::Ui, state: &Snapshot, ready: bool, enabled: bool) -> Response {
    if i18n::current().is_rtl() {
        // Cards are placed by measure, so the row itself must run from left to right.
        return ui
            .with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                cards(ui, state, ready, enabled, true)
            })
            .inner;
    }
    cards(ui, state, ready, enabled, false)
}

fn cards(
    ui: &mut egui::Ui,
    state: &Snapshot,
    ready: bool,
    enabled: bool,
    mirrored: bool,
) -> Response {
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
        let active = index == 0 && state.running && ready;
        let accent = if ui.visuals().dark_mode {
            egui::Color32::from_rgb(67, 188, 143)
        } else {
            egui::Color32::from_rgb(28, 134, 99)
        };
        let card_frame = if active {
            frame
                .stroke(egui::Stroke::new(1.0, accent))
                .fill(ui.visuals().panel_fill.lerp_to_gamma(accent, 0.05))
        } else {
            frame
        };
        bounds[index] = card_frame
            .show(ui, |ui| {
                let body = |ui: &mut egui::Ui| {
                    ui.set_width(width);
                    ui.set_min_height(120.0);
                    ui.strong(
                        [
                            t("counters.current_interval"),
                            t("counters.today_project"),
                            t("counters.today_all"),
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
                        ui.label(t("counters.unavailable"));
                    }
                    if index == 0 {
                        // Reserve the same row and indicator space in every state.
                        ui.horizontal(|ui| {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(10.0, 16.0), egui::Sense::hover());
                            if active {
                                ui.painter().circle_filled(rect.center(), 3.0, accent);
                                ui.label(
                                    egui::RichText::new(t("counters.tracking"))
                                        .strong()
                                        .color(accent),
                                );
                            } else {
                                ui.label(if !ready {
                                    t("counters.opening_history")
                                } else if state.running {
                                    t("counters.tracking")
                                } else {
                                    t("counters.stopped")
                                });
                            }
                        });
                        toggle |= crate::ui_support::primary_action(
                            ui,
                            if state.running {
                                t("tray.stop")
                            } else {
                                t("tray.start")
                            },
                            enabled,
                        )
                        .clicked();
                    } else {
                        ui.add(
                            egui::Label::new(if index == 1 {
                                state.activity.painted_label()
                            } else {
                                Cow::Borrowed(t("counters.includes_unassigned"))
                            })
                            .truncate(),
                        );
                        ui.weak(t("counters.all_tasks_local_day"));
                    }
                };
                if mirrored {
                    let layout = egui::Layout::top_down(egui::Align::Max);
                    ui.allocate_ui_with_layout(egui::vec2(width, 0.0), layout, body);
                } else {
                    ui.vertical(body);
                }
            })
            .response
            .rect;
        crate::motion::animate_border(
            ui,
            bounds[index],
            f32::from(frame.corner_radius.nw),
            accent,
            active && crate::motion::visible(ui, bounds[index]) && crate::motion::allowed(),
        );
    };
    if wide {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let order = if mirrored { [2, 1, 0] } else { [0, 1, 2] };
            for index in order {
                card(ui, index, width);
            }
        });
    } else {
        card(ui, 0, available - margin);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let order = if mirrored { [2, 1] } else { [1, 2] };
            for index in order {
                card(ui, index, width);
            }
        });
    }
    Response { toggle, bounds }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn running_accent_preserves_counter_geometry_and_stopped_state_is_plain() {
        for width in [404.0, 744.0] {
            let mut bounds = Vec::new();
            let mut button_positions = Vec::new();
            for running in [false, true] {
                let context = egui::Context::default();
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 800.0),
                        )),
                        time: Some(4.0),
                        ..Default::default()
                    },
                    |ui| {
                        bounds.push(
                            show(
                                ui,
                                &Snapshot {
                                    running,
                                    ..Default::default()
                                },
                                true,
                                true,
                            )
                            .bounds,
                        );
                    },
                );
                button_positions.push(
                    output
                        .shapes
                        .iter()
                        .find_map(|shape| {
                            if let egui::Shape::Text(text) = &shape.shape
                                && matches!(
                                    text.galley.job.text.as_str(),
                                    "Start tracking" | "Stop tracking"
                                )
                            {
                                Some(text.pos)
                            } else {
                                None
                            }
                        })
                        .expect("tracking button must be painted"),
                );
                let lights = output
                    .shapes
                    .iter()
                    .filter(|shape| {
                        matches!(
                            &shape.shape, egui::Shape::Circle(circle) if circle.radius == 3.0
                        )
                    })
                    .count();
                assert_eq!(lights, usize::from(running));
                output.textures_delta.clear();
            }
            assert_eq!(bounds[0], bounds[1], "status must not move counters");
            assert_eq!(
                button_positions[0], button_positions[1],
                "status must not move the tracking action"
            );
        }
    }
    #[test]
    fn counters_fit_the_narrowest_window_in_every_language() {
        let overflow = std::cell::RefCell::new(Vec::new());
        let overlap = std::cell::RefCell::new(Vec::new());
        for language in crate::i18n::Language::ALL {
            crate::i18n::with_language(language, || {
                let context = egui::Context::default();
                crate::fonts::Fallbacks::new(language).apply(&context);
                for pass in 0..3 {
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(f64::from(pass)),
                            screen_rect: Some(Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(380.0, 800.0),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            let right = ui.max_rect().right();
                            // The application mirrors its root layout in this case.
                            let response = crate::i18n::mirrored(ui, |ui| {
                                show(ui, &Snapshot::default(), true, false)
                            });
                            let [_, second, third] = response.bounds;
                            if second.intersects(third) {
                                overlap.borrow_mut().push(language);
                            }
                            // A failure inside a frame would abort the whole test process.
                            let widest = response
                                .bounds
                                .iter()
                                .map(|bounds| bounds.right())
                                .fold(ui.max_rect().right(), f32::max);
                            if widest > right + 0.5 {
                                overflow.borrow_mut().push((language, widest - right));
                            }
                        },
                    );
                    output.textures_delta.clear();
                }
            });
        }
        assert!(overflow.borrow().is_empty(), "{:?}", overflow.borrow());
        assert!(overlap.borrow().is_empty(), "{:?}", overlap.borrow());
    }
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
                    crate::ui_support::window_frame(ui.style()).show(ui, |ui| {
                        crate::ui_support::body(ui, |ui| {
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
                    id: 1,
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
