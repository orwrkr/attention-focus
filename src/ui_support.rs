//! Stable scroll geometry and primary action feedback.
use eframe::egui;

pub fn primary_action(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;
        for (visuals, color) in [
            (&mut widgets.inactive, egui::Color32::from_rgb(31, 91, 151)),
            (&mut widgets.hovered, egui::Color32::from_rgb(44, 116, 187)),
            (&mut widgets.active, egui::Color32::from_rgb(20, 61, 105)),
        ] {
            visuals.weak_bg_fill = color;
            visuals.bg_fill = color;
        }
        ui.add_enabled(
            enabled,
            egui::Button::new(
                egui::RichText::new(label)
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .min_size(egui::vec2(ui.available_width(), 32.0)),
        )
    })
    .inner
}

pub fn body<R>(
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    // The floating bar occupies the reserved right margin, not the content. Symmetric
    // margins keep width independent of whether a scrollbar is currently needed.
    let scroll = &mut ui.spacing_mut().scroll;
    scroll.floating = true;
    scroll.floating_allocated_width = 0.0;
    scroll.bar_width = 6.0;
    scroll.floating_width = 6.0;
    scroll.dormant_handle_opacity = 1.0;
    scroll.dormant_background_opacity = 0.15;
    let output = egui::ScrollArea::vertical()
        .id_salt("window-content")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .content_margin(egui::Margin::symmetric(12, 0))
        .auto_shrink([false, false])
        .show(ui, add);
    let size_id = ui.make_persistent_id("last-body-size");
    let previous = ui.data(|data| data.get_temp::<egui::Vec2>(size_id));
    if previous != Some(output.content_size) {
        ui.data_mut(|data| data.insert_temp(size_id, output.content_size));
        ui.ctx().request_repaint();
    }
    output
}

pub fn maximized_edge(ctx: &egui::Context) {
    if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
        let width = 1.0 / ctx.pixels_per_point();
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("maximized-edge"),
        ))
        .rect_stroke(
            ctx.viewport_rect().shrink(width * 0.5),
            0.0,
            egui::Stroke::new(width, egui::Color32::from_rgb(135, 145, 155)),
            egui::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primary_action_paints_distinct_idle_hover_and_pressed_states() {
        let context = egui::Context::default();
        let mut bounds = egui::Rect::NOTHING;
        let mut colors = Vec::new();
        for stage in 0..4 {
            let events = match stage {
                1 => vec![egui::Event::PointerMoved(bounds.center())],
                2 => vec![egui::Event::PointerButton {
                    pos: bounds.center(),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                }],
                3 => vec![egui::Event::PointerButton {
                    pos: bounds.center(),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                }],
                _ => vec![],
            };
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(stage as f64),
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(300.0, 200.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let response = primary_action(ui, "Start tracking", stage != 3);
                    bounds = response.rect;
                    if stage == 2 {
                        assert!(response.is_pointer_button_down_on());
                    }
                    if stage == 3 {
                        assert!(!response.enabled());
                        assert!(!response.clicked());
                    }
                },
            );
            let color = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Rect(rect) = &shape.shape
                        && rect.rect.contains(bounds.center())
                        && rect.fill.a() > 0
                    {
                        Some(rect.fill)
                    } else {
                        None
                    }
                })
                .unwrap();
            colors.push(color);
            output.textures_delta.clear();
        }
        assert_ne!(colors[0], colors[1]);
        assert_ne!(colors[1], colors[2]);
        assert_ne!(colors[0], colors[2]);
    }

    #[test]
    fn repeated_section_reflow_clamps_scroll_and_never_duplicates_counter_paint() {
        for width in [420.0, 760.0] {
            let context = egui::Context::default();
            let mut frame = 0;
            for open in [false, true, false, true, false] {
                for settling in 0..12 {
                    let mut overflowing = false;
                    let mut offset = 0.0;
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(frame as f64 * 0.1),
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, 800.0),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            let area = body(ui, |ui| {
                                crate::counters::show(
                                    ui,
                                    &crate::tracking::Snapshot {
                                        elapsed_ms: 3000,
                                        ..Default::default()
                                    },
                                    true,
                                    false,
                                );
                                egui::CollapsingHeader::new("Synthetic details")
                                    .open(Some(open))
                                    .show(ui, |ui| {
                                        for _ in 0..100 {
                                            ui.label("Synthetic content row");
                                        }
                                    });
                                if open && settling == 10 {
                                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                                }
                            });
                            overflowing = area.content_size.y > area.inner_rect.height() + 1.0;
                            offset = area.state.unclamped_offset().y;
                        },
                    );
                    let timers = output.shapes.iter().filter(|shape|
                        matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "00:00:03")).count();
                    assert!(timers <= 1, "duplicate timer after reflow");
                    if settling == 11 {
                        assert_eq!(
                            overflowing, open,
                            "incorrect overflow after reflow at {width}"
                        );
                        if !open {
                            assert!(offset.abs() < 1.0);
                            assert_eq!(timers, 1);
                        }
                    }
                    output.textures_delta.clear();
                    frame += 1;
                }
            }
        }
    }
}
