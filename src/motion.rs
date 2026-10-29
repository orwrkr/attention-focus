//! Subtle, status-preserving motion; never used as a tracking clock.
pub fn level(seconds: f64) -> f32 {
    ((1.0 - (seconds.rem_euclid(8.0) * std::f64::consts::TAU / 8.0).cos()) * 0.5) as f32
}

/// Five cached green brightness levels, advancing at most once per second.
pub fn tray_level(seconds: f64, animate: bool) -> u8 {
    if !animate {
        return 4;
    }
    [0, 1, 2, 3, 4, 3, 2, 1][seconds.rem_euclid(8.0) as usize]
}

/// Point on a rounded rectangle, clockwise from the top edge's left tangent.
fn border_point(rect: eframe::egui::Rect, radius: f32, distance: f32) -> eframe::egui::Pos2 {
    use eframe::egui::pos2;
    let r = radius.max(0.1).min(rect.width().min(rect.height()) * 0.5);
    let horizontal = (rect.width() - 2.0 * r).max(0.0);
    let vertical = (rect.height() - 2.0 * r).max(0.0);
    let arc = std::f32::consts::FRAC_PI_2 * r;
    let perimeter = 2.0 * (horizontal + vertical) + 4.0 * arc;
    let mut d = distance.rem_euclid(perimeter);
    for side in 0..4 {
        let length = if side % 2 == 0 { horizontal } else { vertical };
        if d <= length {
            return match side {
                0 => pos2(rect.left() + r + d, rect.top()),
                1 => pos2(rect.right(), rect.top() + r + d),
                2 => pos2(rect.right() - r - d, rect.bottom()),
                _ => pos2(rect.left(), rect.bottom() - r - d),
            };
        }
        d -= length;
        if d <= arc {
            let center = match side {
                0 => pos2(rect.right() - r, rect.top() + r),
                1 => pos2(rect.right() - r, rect.bottom() - r),
                2 => pos2(rect.left() + r, rect.bottom() - r),
                _ => pos2(rect.left() + r, rect.top() + r),
            };
            let angle = (side as f32 - 1.0) * std::f32::consts::FRAC_PI_2 + d / r;
            return center + eframe::egui::vec2(angle.cos(), angle.sin()) * r;
        }
        d -= arc;
    }
    pos2(rect.left() + r, rect.top())
}

/// A fading quarter-perimeter trail; paint only, with no layout or storage effects.
pub fn paint_border(
    painter: &eframe::egui::Painter,
    bounds: eframe::egui::Rect,
    radius: f32,
    color: eframe::egui::Color32,
    seconds: f64,
) {
    use eframe::egui::Stroke;
    let rect = bounds.shrink(1.5);
    if rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }
    let r = (radius - 1.5)
        .max(0.1)
        .min(rect.width().min(rect.height()) * 0.5);
    let perimeter = 2.0 * (rect.width() + rect.height() - 4.0 * r) + std::f32::consts::TAU * r;
    let head = (seconds.rem_euclid(8.0) / 8.0) as f32 * perimeter;
    let tail = perimeter * 0.25;
    for step in 0..96 {
        let t = step as f32 / 96.0;
        let next = (step + 1) as f32 / 96.0;
        let from = border_point(rect, r, head - tail + tail * t);
        let to = border_point(rect, r, head - tail + tail * next);
        painter.line_segment(
            [from, to],
            Stroke::new(2.5, color.gamma_multiply(next * next)),
        );
    }
}

/// Do not generate trail geometry or schedule animation when it cannot be shown.
pub fn animate_border(
    ui: &eframe::egui::Ui,
    bounds: eframe::egui::Rect,
    radius: f32,
    color: eframe::egui::Color32,
    enabled: bool,
) {
    if enabled
        && ui.is_rect_visible(bounds)
        && ui.input(|input| input.viewport().visible().unwrap_or(true))
    {
        paint_border(
            ui.painter(),
            bounds,
            radius,
            color,
            ui.input(|input| input.time),
        );
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
}

pub fn allowed() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
        };
        let mut enabled = 0_i32;
        // Respect Windows accessibility preferences; fail closed to static status.
        unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                (&mut enabled as *mut i32).cast(),
                0,
            ) != 0
                && enabled != 0
        }
    }
    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invisible_animation_emits_no_geometry_or_repaint_loop() {
        use eframe::egui::{self, Color32, Pos2, Rect, vec2};
        for (enabled, clipped, minimized, occluded) in [
            (true, false, false, false),
            (false, false, false, false),
            (true, true, false, false),
            (true, false, true, false),
            (true, false, false, true),
        ] {
            let context = egui::Context::default();
            for pass in 0..4 {
                let mut input = egui::RawInput {
                    time: Some(pass as f64),
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
                    ..Default::default()
                };
                let viewport = input.viewports.entry(egui::ViewportId::ROOT).or_default();
                viewport.minimized = Some(minimized);
                viewport.occluded = Some(occluded);
                let mut output = context.run_ui(input, |ui| {
                    let rect = Rect::from_min_size(
                        Pos2::new(10.0, if clipped { 500.0 } else { 10.0 }),
                        vec2(220.0, 140.0),
                    );
                    animate_border(ui, rect, 6.0, Color32::GREEN, enabled);
                });
                if pass == 3 {
                    let active = enabled && !clipped && !minimized && !occluded;
                    let lines = output
                        .shapes
                        .iter()
                        .filter(|s| matches!(s.shape, egui::Shape::LineSegment { .. }))
                        .count();
                    assert_eq!(lines, if active { 96 } else { 0 });
                    let delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
                    assert_eq!(delay <= std::time::Duration::from_millis(50), active);
                }
                output.textures_delta.clear();
            }
        }
    }

    #[test]
    fn border_trail_is_periodic_graded_and_inside_the_card() {
        use eframe::egui::{self, Color32, Pos2, Rect, vec2};
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(220.0, 140.0));
        let context = egui::Context::default();
        let mut frames = Vec::new();
        for time in [0.0, 2.0, 4.0, 6.0, 8.0] {
            let mut output = context.run_ui(Default::default(), |ui| {
                paint_border(ui.painter(), rect, 6.0, Color32::GREEN, time);
            });
            let lines: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::LineSegment { points, stroke } = &shape.shape {
                        for point in points {
                            assert!(rect.shrink(1.0).contains(*point));
                        }
                        Some((*points, stroke.color))
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(lines.len(), 96);
            assert!(lines.first().unwrap().1.a() < lines.last().unwrap().1.a());
            for pair in lines.windows(2) {
                assert!(pair[0].0[1].distance(pair[1].0[0]) < 0.01);
            }
            frames.push(lines);
            output.textures_delta.clear();
        }
        assert_eq!(frames[0], frames[4]);
        assert_ne!(frames[0], frames[1]);
    }

    #[test]
    fn motion_is_bounded_periodic_and_reduced_motion_is_static() {
        for step in 0..800 {
            let t = step as f64 / 100.0;
            assert!((0.0..=1.0).contains(&level(t)));
            assert!((level(t) - level(t + 8.0)).abs() < 0.00001);
            assert!((level(t + 0.01) - level(t)).abs() < 0.004);
            assert!(tray_level(t, true) <= 4);
            assert_eq!(tray_level(t, false), 4);
        }
        assert_eq!(level(0.0), 0.0);
        assert_eq!(level(4.0), 1.0);
        assert_eq!(tray_level(0.9, true), tray_level(0.0, true));
    }
}
