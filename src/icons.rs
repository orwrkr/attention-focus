//! One scalable geometric mark shared by Windows resources and runtime icons.

#[derive(Clone, Copy)]
pub enum State {
    App,
    AppDevelopment,
    Running,
    Stopped,
    RunningDevelopment,
    StoppedDevelopment,
}

pub fn rgba(size: u32, state: State) -> Vec<u8> {
    tracking_rgba(size, state, 4)
}

/// Brightness varies only for running tray lights; identity and stopped marks stay static.
pub fn tracking_rgba(size: u32, state: State, level: u8) -> Vec<u8> {
    let accent = match state {
        State::App => [112.0, 160.0, 255.0],
        State::AppDevelopment => [194.0, 145.0, 255.0],
        State::Running | State::RunningDevelopment => [42.0, 212.0, 160.0],
        State::Stopped | State::StoppedDevelopment => [169.0, 181.0, 198.0],
    };
    let accent = if matches!(state, State::Running | State::RunningDevelopment) {
        accent.map(|channel| channel * (0.88 + 0.03 * level.min(4) as f32))
    } else {
        accent
    };
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0.0_f32; 4];
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = (x as f32 + (sx as f32 + 0.5) / 4.0) / size as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / 4.0) / size as f32;
                    let dx = px - 0.5;
                    let dy = py - 0.5;
                    let radius = dx.hypot(dy);
                    let badge =
                        matches!(state, State::RunningDevelopment | State::StoppedDevelopment)
                            && (0.66..0.96).contains(&px)
                            && (0.66..0.96).contains(&py);
                    if radius > 0.475 && !badge {
                        continue;
                    }
                    let ring = (0.33..=0.40).contains(&radius);
                    let center = match state {
                        State::App | State::AppDevelopment => dx.abs() + dy.abs() < 0.17,
                        State::Running | State::RunningDevelopment => radius < 0.19,
                        State::Stopped | State::StoppedDevelopment => false,
                    };
                    let mut color = if ring || center {
                        accent
                    } else {
                        [19.0, 29.0, 45.0]
                    };
                    // A purple D badge identifies development without changing status color.
                    if badge {
                        color = [155.0, 93.0, 225.0];
                        let bx = (px - 0.66) / 0.30;
                        let by = (py - 0.66) / 0.30;
                        if (0.20..0.35).contains(&bx) && (0.20..0.80).contains(&by)
                            || (0.35..0.62).contains(&bx)
                                && ((0.20..0.34).contains(&by) || (0.66..0.80).contains(&by))
                            || (0.57..0.76).contains(&bx) && (0.31..0.69).contains(&by)
                        {
                            color = [255.0, 255.0, 255.0];
                        }
                    }
                    for i in 0..3 {
                        sum[i] += color[i];
                    }
                    sum[3] += 255.0;
                }
            }
            let coverage = sum[3] / 255.0;
            for value in &sum[..3] {
                pixels.push(if coverage > 0.0 {
                    (value / coverage).round() as u8
                } else {
                    0
                });
            }
            pixels.push((sum[3] / 16.0).round() as u8);
        }
    }
    pixels
}

/// Multi-resolution 32-bit DIB ICO, with transparency masks for Windows shell use.
pub fn ico(state: State) -> Vec<u8> {
    let sizes = [16_u32, 20, 24, 32, 48, 64, 128, 256];
    let mut output = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut images = Vec::new();
    let mut offset = 6 + sizes.len() * 16;
    for size in sizes {
        let pixels = rgba(size, state);
        let mask_stride = size.div_ceil(32) * 4;
        let data_size = size * size * 4 + mask_stride * size;
        let mut image = Vec::new();
        for value in [40_u32, size, size * 2] {
            image.extend(value.to_le_bytes());
        }
        image.extend(1_u16.to_le_bytes());
        image.extend(32_u16.to_le_bytes());
        for value in [0, data_size, 0, 0, 0, 0] {
            image.extend(value.to_le_bytes());
        }
        for y in (0..size).rev() {
            for x in 0..size {
                let i = ((y * size + x) * 4) as usize;
                image.extend([pixels[i + 2], pixels[i + 1], pixels[i], pixels[i + 3]]);
            }
        }
        for y in (0..size).rev() {
            let mut mask = vec![0_u8; mask_stride as usize];
            for x in 0..size {
                if pixels[((y * size + x) * 4 + 3) as usize] == 0 {
                    mask[(x / 8) as usize] |= 0x80 >> (x % 8);
                }
            }
            image.extend(mask);
        }
        output.extend([size as u8, size as u8, 0, 0]);
        output.extend(1_u16.to_le_bytes());
        output.extend(32_u16.to_le_bytes());
        output.extend((image.len() as u32).to_le_bytes());
        output.extend((offset as u32).to_le_bytes());
        offset += image.len();
        images.extend(image);
    }
    output.extend(images);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtle_tray_levels_preserve_status_shape_alpha_and_development_badge() {
        for state in [State::Running, State::RunningDevelopment] {
            let bright = tracking_rgba(32, state, 4);
            let dim = tracking_rgba(32, state, 0);
            assert_ne!(bright, dim);
            for (a, b) in bright.as_chunks::<4>().0.iter().zip(dim.as_chunks::<4>().0) {
                assert_eq!(a[3], b[3]);
                for channel in 0..3 {
                    assert!((a[channel] as i16 - b[channel] as i16).abs() <= 26);
                }
            }
            let center = (16 * 32 + 16) * 4;
            assert!(dim[center + 1] > 180);
            assert!(dim[center + 1] > dim[center + 2]);
            if matches!(state, State::RunningDevelopment) {
                for y in 23..29 {
                    for x in 23..29 {
                        let offset = (y * 32 + x) * 4;
                        assert_eq!(&bright[offset..offset + 4], &dim[offset..offset + 4]);
                    }
                }
            }
        }
        for state in [
            State::App,
            State::AppDevelopment,
            State::Stopped,
            State::StoppedDevelopment,
        ] {
            assert_eq!(tracking_rgba(32, state, 0), tracking_rgba(32, state, 4));
        }
    }

    #[test]
    fn states_differ_and_keep_transparent_corners() {
        let running = rgba(32, State::Running);
        let stopped = rgba(32, State::Stopped);
        assert_ne!(running, stopped);
        assert_eq!(running.len(), 32 * 32 * 4);
        assert_eq!(running[3], 0);
        assert_eq!(running[(16 * 32 + 16) * 4 + 3], 255);
        for size in [16, 32, 64] {
            let states = [
                State::App,
                State::AppDevelopment,
                State::Running,
                State::Stopped,
                State::RunningDevelopment,
                State::StoppedDevelopment,
            ];
            let images: Vec<_> = states.iter().map(|s| rgba(size, *s)).collect();
            for i in 0..images.len() {
                for j in i + 1..images.len() {
                    assert_ne!(images[i], images[j]);
                }
            }
            let center = ((size / 2 * size + size / 2) * 4) as usize;
            assert!(images[0][center + 2] > images[0][center + 1]);
            assert!(images[2][center + 1] > images[2][center + 2]);
            assert_eq!(&images[3][center..center + 3], &[19, 29, 45]);
        }
    }

    #[test]
    fn ico_directory_entries_cover_the_file() {
        let bytes = ico(State::App);
        assert_eq!(&bytes[..6], &[0, 0, 1, 0, 8, 0]);
        let mut end = 6 + 8 * 16;
        for entry in bytes[6..end].as_chunks::<16>().0 {
            let length = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            assert_eq!(offset, end);
            assert_eq!(
                u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
                40
            );
            end = offset + length;
        }
        assert_eq!(end, bytes.len());
    }
}
