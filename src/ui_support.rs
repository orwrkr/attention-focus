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

/// One look for every scroll bar: a slim handle in a muted shade of the theme's text color,
/// beside the content and never over it. Text is not faded at the edges of an area either.
pub fn scroll_style() -> egui::style::ScrollStyle {
    egui::style::ScrollStyle {
        floating: true,
        bar_width: 8.0,
        floating_width: 6.0,
        bar_outer_margin: 2.0,
        // Room for the widest bar and its outer margin, plus a gap before the content.
        floating_allocated_width: 14.0,
        handle_min_length: 24.0,
        foreground_color: true,
        dormant_handle_opacity: 0.35,
        active_handle_opacity: 0.55,
        interact_handle_opacity: 0.8,
        dormant_background_opacity: 0.0,
        active_background_opacity: 0.4,
        interact_background_opacity: 0.6,
        fade: egui::style::ScrollFadeStyle {
            strength: 0.0,
            size: 0.0,
        },
        ..egui::style::ScrollStyle::solid()
    }
}

/// Applies the shared interface style to both themes, and so to popups as well.
pub fn apply_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| style.spacing.scroll = scroll_style());
}

/// A scroll area inside the window body, as wide as its column so its bar lines up at the
/// right. Without a minimum, egui limits it to the height left in the window, which is usually
/// nothing inside a body that scrolls, leaving room for only a couple of rows.
pub fn inner_area(id: &str, height: f32) -> egui::ScrollArea {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(height)
        .min_scrolled_height(height)
        .auto_shrink([false, true])
}

const WINDOW_MARGIN: i8 = 8;
const BODY_MARGIN: i8 = 12;

/// The window's outer frame. The right margin moves inside the header and the body, so the
/// body's scroll bar can lie at the window edge.
pub fn window_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::central_panel(style).inner_margin(egui::Margin {
        right: 0,
        ..egui::Margin::same(WINDOW_MARGIN)
    })
}
/// The fixed area above the body, with the right margin that `window_frame` leaves to it.
pub fn header_frame() -> egui::Frame {
    egui::Frame::NONE.inner_margin(egui::Margin {
        right: WINDOW_MARGIN,
        ..egui::Margin::ZERO
    })
}

pub fn body<R>(
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    // The bar lies in the right margin, which reserves its space in advance. Content width
    // therefore does not depend on whether a scroll bar is currently needed.
    let output = ui
        .scope(|ui| {
            ui.spacing_mut().scroll.floating_allocated_width = 0.0;
            egui::ScrollArea::vertical()
                .id_salt("window-content")
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .content_margin(egui::Margin {
                    left: BODY_MARGIN,
                    right: BODY_MARGIN + WINDOW_MARGIN,
                    top: 0,
                    bottom: 0,
                })
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Scroll areas inside the body reserve room for their own bars.
                    ui.spacing_mut().scroll = scroll_style();
                    add(ui)
                })
        })
        .inner;
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

    /// Thin vertical rectangles are scroll bars: nothing else in this test paints a filled one.
    fn scroll_bars(output: &egui::FullOutput) -> Vec<egui::Rect> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect)
                    if rect.fill.a() > 0
                        && rect.rect.width() <= 8.01
                        && rect.rect.height() >= 20.0 =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .collect()
    }
    fn text_boxes(output: &egui::FullOutput) -> Vec<egui::Rect> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.visual_bounding_rect()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn scroll_bars_never_cover_content_in_the_body_or_inside_it() {
        let context = egui::Context::default();
        apply_style(&context);
        let right_aligned_rows = |ui: &mut egui::Ui, count: usize| {
            for row in 0..count {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    ui.label(format!("Row {row}"));
                });
            }
        };
        let width = 500.0;
        let mut area_width = (0.0, 0.0);
        let mut output = None;
        for frame in 0..8 {
            let mut result = context.run_ui(
                egui::RawInput {
                    time: Some(f64::from(frame)),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 300.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    area_width.0 = body(ui, |ui| {
                        area_width.1 = inner_area("nested", 100.0)
                            .show(ui, |ui| {
                                right_aligned_rows(ui, 30);
                            })
                            .inner_rect
                            .width();
                        right_aligned_rows(ui, 60);
                    })
                    .inner_rect
                    .width();
                },
            );
            result.textures_delta.clear();
            output = Some(result);
        }
        let output = output.unwrap();
        let bars = scroll_bars(&output);
        assert!(bars.len() >= 2, "both areas overflow: {bars:?}");
        let texts = text_boxes(&output);
        assert!(!texts.is_empty());
        for bar in &bars {
            for text in &texts {
                assert!(!bar.intersects(*text), "bar {bar:?} covers text {text:?}");
            }
        }
        assert!(
            area_width.1 <= area_width.0 - scroll_style().floating_allocated_width,
            "the inner area gives up room for its own bar: {area_width:?}"
        );
    }

    #[test]
    fn inner_areas_keep_their_height_far_down_a_scrolling_body() {
        let context = egui::Context::default();
        apply_style(&context);
        let mut height = 0.0;
        let mut short = 0.0;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(500.0, 300.0),
                )),
                ..Default::default()
            },
            |ui| {
                body(ui, |ui| {
                    ui.add_space(2000.0);
                    let top = ui.cursor().top();
                    inner_area("long", 240.0).show(ui, |ui| ui.add_space(5000.0));
                    height = ui.cursor().top() - top - ui.spacing().item_spacing.y;
                    let top = ui.cursor().top();
                    inner_area("short", 240.0).show(ui, |ui| ui.add_space(50.0));
                    short = ui.cursor().top() - top - ui.spacing().item_spacing.y;
                });
            },
        );
        output.textures_delta.clear();
        assert_eq!(height, 240.0);
        assert!(
            short <= 60.0,
            "short content leaves no empty space: {short}"
        );
    }

    #[test]
    fn scroll_bars_look_the_same_in_popups_and_in_both_themes() {
        let context = egui::Context::default();
        apply_style(&context);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            assert_eq!(
                context
                    .style_of(theme)
                    .spacing
                    .scroll
                    .floating_allocated_width,
                scroll_style().floating_allocated_width,
                "{theme:?}"
            );
        }
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
