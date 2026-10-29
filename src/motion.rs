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

/// Decorative activity marker; fixed bounds keep timer controls stationary.
pub fn paint_activity(
    painter: &eframe::egui::Painter,
    rect: eframe::egui::Rect,
    color: eframe::egui::Color32,
    phase: f32,
) {
    use eframe::egui::{Stroke, pos2};
    let left = pos2(rect.left() + 4.0, rect.center().y);
    let right = pos2(rect.right() - 4.0, rect.center().y);
    painter.line_segment([left, right], Stroke::new(1.0, color.gamma_multiply(0.35)));
    painter.circle_filled(left.lerp(right, phase.clamp(0.0, 1.0)), 2.5, color);
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
    fn activity_marker_moves_visibly_inside_fixed_bounds() {
        use eframe::egui::{self, Color32, Pos2, Rect, vec2};
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(56.0, 16.0));
        let mut centers = Vec::new();
        for phase in [0.0, 0.5, 1.0] {
            let mut output = context.run_ui(Default::default(), |ui| {
                paint_activity(ui.painter(), rect, Color32::GREEN, phase);
            });
            let center = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Circle(circle) = &shape.shape {
                        Some(circle.center)
                    } else {
                        None
                    }
                })
                .expect("activity marker must be painted");
            assert!(rect.shrink(2.5).contains(center));
            centers.push(center);
            output.textures_delta.clear();
        }
        assert_eq!(centers[0].y, centers[2].y);
        assert_eq!(centers[2].x - centers[0].x, 48.0);
        assert_eq!(centers[1], rect.center());
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
