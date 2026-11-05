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

const MARK_WIDTH: f32 = 16.0;
const BALLOON_WIDTH: f32 = 320.0;

/// What a balloon holds: logical text with a paragraph for each line, no wider than the window.
pub fn balloon(ui: &mut egui::Ui, text: &str) {
    ui.set_max_width(BALLOON_WIDTH.min(ui.ctx().content_rect().width() - 32.0));
    crate::i18n::mirrored(ui, |ui| {
        for line in text.lines() {
            crate::i18n::paragraph(ui, line);
        }
    });
}

fn paint_mark(ui: &egui::Ui, response: &egui::Response, text: &str, about: &[&egui::Response]) {
    let rect = response.rect;
    if !ui.is_rect_visible(rect) {
        return;
    }
    let focused = response.has_focus();
    let open =
        response.contains_pointer() || focused || about.iter().any(|control| control.has_focus());
    let visuals = ui.visuals();
    let color = if open {
        visuals.strong_text_color()
    } else {
        visuals.weak_text_color()
    };
    // Drawn rather than typed, so it looks the same whichever font a language uses.
    let center = rect.center();
    let painter = ui.painter();
    // The mark may be the keyboard's stop, and that has to show on the mark itself.
    let ring = if focused { 2.0 } else { 1.0 };
    painter.circle_stroke(center, 6.5, egui::Stroke::new(ring, color));
    painter.circle_filled(center + egui::vec2(0.0, -3.0), 1.0, color);
    painter.line_segment(
        [
            center + egui::vec2(0.0, -1.0),
            center + egui::vec2(0.0, 3.5),
        ],
        egui::Stroke::new(1.5, color),
    );
    if open {
        egui::containers::Tooltip::for_widget(response).show(|ui| balloon(ui, text));
    }
}

/// A mark that stands in for an explanation, and a Tab stop of its own. Its balloon opens while
/// the pointer rests on the mark, and while the keyboard focus is on the mark or on a control the
/// explanation is about.
pub fn info(ui: &mut egui::Ui, text: &str, about: &[&egui::Response]) -> egui::Response {
    let height = ui.spacing().interact_size.y;
    mark_of_height(ui, height, text, about)
}
fn mark_of_height(
    ui: &mut egui::Ui,
    height: f32,
    text: &str,
    about: &[&egui::Response],
) -> egui::Response {
    let size = egui::vec2(MARK_WIDTH, height);
    let (_, response) = ui.allocate_exact_size(size, egui::Sense::focusable_noninteractive());
    paint_mark(ui, &response, text, about);
    response
}
/// The height whose middle is the middle of a control's first line. Controls and labels differ in
/// height, so a mark takes the height of the one it stands beside.
fn first_line_height(ui: &egui::Ui, control: &egui::Response) -> f32 {
    let line = ui.text_style_height(&egui::TextStyle::Body);
    if control.rect.height() < line * 1.5 {
        control.rect.height()
    } else {
        line
    }
}

/// A control or label followed by its mark. It wraps in the width that the mark leaves, and the
/// mark stays beside its first line.
pub fn with_info(
    ui: &mut egui::Ui,
    text: &str,
    add: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    control_and_mark(ui, text, add).0
}
fn control_and_mark(
    ui: &mut egui::Ui,
    text: &str,
    add: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> (egui::Response, egui::Response) {
    ui.horizontal_top(|ui| {
        let width = ui.available_width() - MARK_WIDTH - ui.spacing().item_spacing.x;
        let layout = egui::Layout::top_down(if ui.layout().prefer_right_to_left() {
            egui::Align::Max
        } else {
            egui::Align::Min
        });
        let control = ui
            .allocate_ui_with_layout(egui::vec2(width, 0.0), layout, add)
            .inner;
        let height = first_line_height(ui, &control);
        let mark = mark_of_height(ui, height, text, &[&control]);
        (control, mark)
    })
    .inner
}

/// A mark after a section header, which is only as wide as its text. It lies beside the header
/// without moving the layout, because the section's body has already been added below it.
pub fn info_after_header(ui: &mut egui::Ui, header: &egui::Response, text: &str) {
    let rect = egui::Rect::from_min_size(
        header.rect.right_top() + egui::vec2(ui.spacing().item_spacing.x, 0.0),
        egui::vec2(MARK_WIDTH, header.rect.height()),
    );
    let response = ui.interact(
        rect,
        header.id.with("info"),
        egui::Sense::focusable_noninteractive(),
    );
    paint_mark(ui, &response, text, &[header]);
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
                    let added = add(ui);
                    follow_keyboard_focus(ui);
                    added
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

/// egui moves the keyboard focus without bringing the widget into view. A widget that its clip
/// does not cut is already in view, which also leaves the fixed header out of this.
fn follow_keyboard_focus(ui: &egui::Ui) {
    let ctx = ui.ctx();
    let focused = ctx.memory(|memory| memory.focused());
    if let Some(widget) = focused.and_then(|id| ctx.read_response(id))
        && widget.gained_focus()
        && widget.interact_rect != widget.rect
    {
        widget.scroll_to_me(None);
    }
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

    /// Runs a frame at the given width and returns the text that it painted.
    fn painted(
        context: &egui::Context,
        time: f64,
        width: f32,
        events: Vec<egui::Event>,
        add: impl FnMut(&mut egui::Ui),
    ) -> Vec<(String, egui::Rect)> {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 600.0),
                )),
                ..Default::default()
            },
            add,
        );
        output.textures_delta.clear();
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => {
                    Some((text.galley.job.text.clone(), text.visual_bounding_rect()))
                }
                _ => None,
            })
            .collect()
    }
    fn pointer(position: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    #[test]
    fn the_balloon_opens_on_hover_and_keyboard_focus_but_not_after_a_click() {
        const HINT: &str = "Synthetic explanation";
        let context = egui::Context::default();
        let mark = std::cell::Cell::new(egui::Rect::NOTHING);
        let control = std::cell::Cell::new(egui::Rect::NOTHING);
        let mut time = 0.0;
        // Each step settles over a few frames, as events reach egui before focus moves.
        let mut step = |events: Vec<egui::Event>| {
            let mut open = false;
            let mut events = events;
            for _ in 0..3 {
                time += 1.0;
                let texts = painted(&context, time, 400.0, std::mem::take(&mut events), |ui| {
                    let button = ui.button("Control");
                    control.set(button.rect);
                    mark.set(info(ui, HINT, &[&button]).rect);
                });
                open = texts.iter().any(|(text, _)| text == HINT);
            }
            open
        };
        assert!(!step(vec![]), "closed at rest");
        let on_mark = mark.get().center();
        let on_control = control.get().center();
        let away = egui::pos2(300.0, 500.0);
        assert!(
            step(vec![egui::Event::PointerMoved(on_mark)]),
            "open on hover"
        );
        assert!(
            !step(vec![egui::Event::PointerMoved(away)]),
            "closed after hover"
        );
        let tab = egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        };
        assert!(step(vec![tab]), "open on keyboard focus");
        assert!(
            !step(vec![pointer(away, true), pointer(away, false)]),
            "closed when a click elsewhere takes the focus away"
        );
        assert!(
            !step(vec![
                egui::Event::PointerMoved(on_control),
                pointer(on_control, true),
                pointer(on_control, false),
                egui::Event::PointerMoved(away),
            ]),
            "a click on the control gives it no focus, so no balloon stays open"
        );
    }

    #[test]
    fn every_mark_is_a_tab_stop_so_explanations_of_labels_open_by_keyboard() {
        const LABEL_HINT: &str = "Synthetic label explanation";
        const CONTROL_HINT: &str = "Synthetic control explanation";
        let context = egui::Context::default();
        let label_mark = std::cell::Cell::new(None);
        let control = std::cell::Cell::new(None);
        let control_mark = std::cell::Cell::new(None);
        let mut time = 0.0;
        let mut tab = || {
            let mut events = vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }];
            let mut open = Vec::new();
            for _ in 0..3 {
                time += 1.0;
                let texts = painted(&context, time, 400.0, std::mem::take(&mut events), |ui| {
                    let (_, mark) =
                        control_and_mark(ui, LABEL_HINT, |ui| ui.label("Synthetic label"));
                    label_mark.set(Some(mark.id));
                    let (button, mark) =
                        control_and_mark(ui, CONTROL_HINT, |ui| ui.button("Control"));
                    control.set(Some(button.id));
                    control_mark.set(Some(mark.id));
                });
                open = texts
                    .into_iter()
                    .map(|(text, _)| text)
                    .filter(|text| text == LABEL_HINT || text == CONTROL_HINT)
                    .collect();
            }
            (context.memory(|memory| memory.focused()), open)
        };
        assert_eq!(tab(), (label_mark.get(), vec![LABEL_HINT.to_owned()]));
        assert_eq!(tab(), (control.get(), vec![CONTROL_HINT.to_owned()]));
        assert_eq!(tab(), (control_mark.get(), vec![CONTROL_HINT.to_owned()]));
    }

    #[test]
    fn the_body_scrolls_to_a_widget_that_takes_the_keyboard_focus_and_leaves_the_header() {
        const HEIGHT: f32 = 300.0;
        let context = egui::Context::default();
        apply_style(&context);
        let rows = std::cell::RefCell::new(Vec::new());
        let header = std::cell::Cell::new(egui::Rect::NOTHING);
        let offset = std::cell::Cell::new(0.0);
        let clip = std::cell::Cell::new(egui::Rect::NOTHING);
        let mut time = 0.0;
        let mut tab = |shift: bool| {
            let mut events = vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: if shift {
                    egui::Modifiers::SHIFT
                } else {
                    egui::Modifiers::default()
                },
            }];
            // Backward focus arrives a frame later, and the scroll takes frames of its own.
            for _ in 0..8 {
                time += 1.0;
                let mut output = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events: std::mem::take(&mut events),
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(400.0, HEIGHT),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        header.set(ui.button("Header").rect);
                        let area = body(ui, |ui| {
                            clip.set(ui.clip_rect());
                            let mut shown = Vec::new();
                            for row in 0..40 {
                                let button = ui.button(format!("Row {row}"));
                                shown.push((button.id, button.rect));
                            }
                            *rows.borrow_mut() = shown;
                        });
                        offset.set(area.state.unclamped_offset().y);
                    },
                );
                output.textures_delta.clear();
            }
            context.memory(|memory| memory.focused())
        };
        // The first stop is the header, and with the body at its top nothing moves.
        tab(false);
        assert_eq!(offset.get(), 0.0);
        let mut last = None;
        for _ in 0..30 {
            last = tab(false);
            let rows = rows.borrow();
            let rect = rows
                .iter()
                .find(|row| Some(row.0) == last)
                .expect("a row has focus")
                .1;
            assert!(
                clip.get().contains_rect(rect),
                "forward: {rect:?} outside {:?}",
                clip.get()
            );
        }
        assert!(offset.get() > 0.0, "the body scrolled down");
        let scrolled = offset.get();
        for _ in 0..20 {
            last = tab(true);
            let rows = rows.borrow();
            let rect = rows
                .iter()
                .find(|row| Some(row.0) == last)
                .expect("a row has focus")
                .1;
            assert!(
                clip.get().contains_rect(rect),
                "backward: {rect:?} outside {:?}",
                clip.get()
            );
        }
        assert!(offset.get() < scrolled, "the body scrolled back up");
        let scrolled = offset.get();
        // Back through the remaining rows to the header, which is always in view.
        for _ in 0..10 {
            last = tab(true);
        }
        assert_eq!(last, context.memory(|memory| memory.focused()));
        assert!(offset.get() <= scrolled);
    }

    #[test]
    fn marks_line_up_with_the_first_line_of_what_they_stand_beside() {
        let results = std::cell::RefCell::new(Vec::new());
        let context = egui::Context::default();
        for pass in 0..3 {
            results.borrow_mut().clear();
            painted(&context, f64::from(pass), 380.0, vec![], |ui| {
                let line = ui.text_style_height(&egui::TextStyle::Body);
                let record = |name: &'static str, pair: (egui::Response, egui::Response)| {
                    let (control, mark) = (pair.0.rect, pair.1.rect);
                    let wanted = if control.height() < line * 1.5 {
                        control.center().y
                    } else {
                        control.top() + line / 2.0
                    };
                    results.borrow_mut().push((
                        name,
                        control.height(),
                        mark.center().y - wanted,
                        line,
                    ));
                };
                record(
                    "label",
                    control_and_mark(ui, "Hint", |ui| ui.label("Synthetic label")),
                );
                record(
                    "strong",
                    control_and_mark(ui, "Hint", |ui| ui.strong("Synthetic title")),
                );
                record(
                    "button",
                    control_and_mark(ui, "Hint", |ui| ui.button("Synthetic button")),
                );
                let mut checked = true;
                record(
                    "checkbox",
                    control_and_mark(ui, "Hint", |ui| ui.checkbox(&mut checked, "Synthetic")),
                );
                record(
                    "picker",
                    control_and_mark(ui, "Hint", |ui| {
                        egui::ComboBox::from_id_salt("synthetic")
                            .selected_text("Synthetic")
                            .show_ui(ui, |_| {})
                            .response
                    }),
                );
                let long = "A synthetic label long enough to need several lines ".repeat(3);
                record(
                    "wrapped",
                    control_and_mark(ui, "Hint", |ui| ui.label(long.as_str())),
                );
                let mut wrapped = true;
                record(
                    "wrapped checkbox",
                    control_and_mark(ui, "Hint", |ui| ui.checkbox(&mut wrapped, long.as_str())),
                );
            });
        }
        let results = results.borrow();
        assert_eq!(results.len(), 7);
        for (name, height, off, line) in results.iter() {
            assert!(off.abs() <= 0.5, "{name}: {off} off, height {height}");
            if name.starts_with("wrapped") {
                assert!(*height > line * 1.5, "{name} wraps: {height}");
            }
        }
    }

    #[test]
    fn marks_stay_beside_their_control_at_the_narrowest_width_in_every_language() {
        let problems = std::cell::RefCell::new(Vec::new());
        for language in crate::i18n::Language::ALL {
            crate::i18n::with_language(language, || {
                let context = egui::Context::default();
                crate::fonts::Fallbacks::new(language).apply(&context);
                for pass in 0..3 {
                    painted(&context, f64::from(pass), 380.0, vec![], |ui| {
                        let right = ui.max_rect().right();
                        let left = ui.max_rect().left();
                        crate::i18n::mirrored(ui, |ui| {
                            let mut checked = true;
                            let (control, mark) = control_and_mark(ui, "Hint", |ui| {
                                let label =
                                    crate::i18n::checkbox_label(ui, "settings.split_on_change");
                                ui.checkbox(&mut checked, label)
                            });
                            let (control, mark) = (control.rect, mark.rect);
                            let beside = if language.is_rtl() {
                                mark.right() <= control.left() + 0.5
                            } else {
                                mark.left() >= control.right() - 0.5
                            };
                            if pass == 2
                                && (!beside
                                    || mark.right() > right + 0.5
                                    || mark.left() < left - 0.5
                                    || control.right() > right + 0.5
                                    || control.left() < left - 0.5)
                            {
                                problems.borrow_mut().push((language, control, mark));
                            }
                        });
                    });
                }
            });
        }
        assert!(problems.borrow().is_empty(), "{:?}", problems.borrow());
    }

    #[test]
    fn a_long_balloon_fits_a_narrow_window_in_every_language() {
        const WIDTH: f32 = 380.0;
        for language in crate::i18n::Language::ALL {
            crate::i18n::with_language(language, || {
                let context = egui::Context::default();
                crate::fonts::Fallbacks::new(language).apply(&context);
                let hint = crate::i18n::message("reports.interrupted_hint", &[]);
                let mut mark = egui::Rect::NOTHING;
                let mut texts = Vec::new();
                for pass in 0..5 {
                    let events = if pass == 0 {
                        vec![]
                    } else {
                        vec![egui::Event::PointerMoved(mark.center())]
                    };
                    texts = painted(&context, f64::from(pass), WIDTH, events, |ui| {
                        crate::i18n::mirrored(ui, |ui| {
                            mark = info(ui, &hint, &[]).rect;
                        });
                    });
                }
                assert!(!texts.is_empty(), "{language:?} shows the balloon");
                for (text, rect) in texts {
                    assert!(
                        rect.left() >= -0.5 && rect.right() <= WIDTH + 0.5,
                        "{language:?} {rect:?}: {text}"
                    );
                }
            });
        }
    }

    #[test]
    fn a_header_mark_lies_beside_the_header_and_leaves_the_layout_alone() {
        let context = egui::Context::default();
        let mut body_end = (0.0, 0.0);
        let mut header = egui::Rect::NOTHING;
        for (slot, with_mark) in [false, true].into_iter().enumerate() {
            painted(&context, slot as f64, 400.0, vec![], |ui| {
                let section = egui::CollapsingHeader::new("Synthetic section")
                    .id_salt("section")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label("Synthetic row");
                    });
                if with_mark {
                    info_after_header(ui, &section.header_response, "Hint");
                    header = section.header_response.rect;
                    body_end.1 = ui.cursor().top();
                } else {
                    body_end.0 = ui.cursor().top();
                }
            });
        }
        assert_eq!(body_end.0, body_end.1);
        assert!(
            header.width() < 300.0,
            "the header is as wide as its text: {header:?}"
        );
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
