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
