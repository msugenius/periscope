use crate::settings::CrosshairSettings;

pub(crate) const OVERLAY_SIZE: i32 = 256;

pub(crate) fn rasterize(pixels: &mut [u32], settings: &CrosshairSettings) {
    let center = OVERLAY_SIZE / 2;
    let color = parse_color(&settings.color);
    let outline_color = parse_color(&settings.outline_color);
    let alpha = ((settings.opacity as u16 * 255) / 100) as u8;
    let half = settings.thickness / 2;

    let mut arms = vec![
        (
            center - settings.gap - settings.length,
            center - half,
            center - settings.gap,
            center - half + settings.thickness,
        ),
        (
            center + settings.gap,
            center - half,
            center + settings.gap + settings.length,
            center - half + settings.thickness,
        ),
        (
            center - half,
            center + settings.gap,
            center - half + settings.thickness,
            center + settings.gap + settings.length,
        ),
    ];
    if !settings.t_style {
        arms.push((
            center - half,
            center - settings.gap - settings.length,
            center - half + settings.thickness,
            center - settings.gap,
        ));
    }

    if settings.outline {
        for &(left, top, right, bottom) in &arms {
            fill_rect(
                pixels,
                left - settings.outline_thickness,
                top - settings.outline_thickness,
                right + settings.outline_thickness,
                bottom + settings.outline_thickness,
                outline_color,
                alpha,
            );
        }
    }
    for &(left, top, right, bottom) in &arms {
        fill_rect(pixels, left, top, right, bottom, color, alpha);
    }

    if settings.center_dot {
        if settings.outline {
            fill_circle(
                pixels,
                center,
                settings.dot_size + settings.outline_thickness * 2,
                outline_color,
                alpha,
            );
        }
        fill_circle(pixels, center, settings.dot_size, color, alpha);
    }
}

fn fill_circle(pixels: &mut [u32], center: i32, diameter: i32, rgb: (u8, u8, u8), alpha: u8) {
    const SAMPLES_PER_AXIS: i32 = 4;
    const SAMPLE_COUNT: i32 = SAMPLES_PER_AXIS * SAMPLES_PER_AXIS;

    let left = center - diameter / 2;
    let right = left + diameter;
    let center_scaled = (left * 2 + diameter) * SAMPLES_PER_AXIS;
    let radius_scaled = diameter * SAMPLES_PER_AXIS;
    let radius_squared = radius_scaled * radius_scaled;

    for y in left.clamp(0, OVERLAY_SIZE)..right.clamp(0, OVERLAY_SIZE) {
        let row = y as usize * OVERLAY_SIZE as usize;
        for x in left.clamp(0, OVERLAY_SIZE)..right.clamp(0, OVERLAY_SIZE) {
            let mut covered_samples = 0;
            for sample_y in 0..SAMPLES_PER_AXIS {
                let y_scaled = (y * 2 * SAMPLES_PER_AXIS) + sample_y * 2 + 1;
                let dy = y_scaled - center_scaled;
                for sample_x in 0..SAMPLES_PER_AXIS {
                    let x_scaled = (x * 2 * SAMPLES_PER_AXIS) + sample_x * 2 + 1;
                    let dx = x_scaled - center_scaled;
                    if dx * dx + dy * dy <= radius_squared {
                        covered_samples += 1;
                    }
                }
            }

            if covered_samples > 0 {
                blend_coverage(
                    &mut pixels[row + x as usize],
                    rgb,
                    alpha,
                    covered_samples,
                    SAMPLE_COUNT,
                );
            }
        }
    }
}

fn blend_coverage(
    destination: &mut u32,
    rgb: (u8, u8, u8),
    alpha: u8,
    covered_samples: i32,
    sample_count: i32,
) {
    let inverse_coverage = sample_count - covered_samples;
    let source_channel = |channel: u8| u32::from(channel) * u32::from(alpha) / 255;
    let mix = |source: u32, existing: u32| {
        (source * covered_samples as u32 + existing * inverse_coverage as u32) / sample_count as u32
    };
    let existing = *destination;
    let output_alpha = mix(u32::from(alpha), existing >> 24);
    let output_red = mix(source_channel(rgb.0), (existing >> 16) & 0xff);
    let output_green = mix(source_channel(rgb.1), (existing >> 8) & 0xff);
    let output_blue = mix(source_channel(rgb.2), existing & 0xff);
    *destination = (output_alpha << 24) | (output_red << 16) | (output_green << 8) | output_blue;
}

fn fill_rect(
    pixels: &mut [u32],
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    rgb: (u8, u8, u8),
    alpha: u8,
) {
    let left = left.clamp(0, OVERLAY_SIZE);
    let right = right.clamp(0, OVERLAY_SIZE);
    let top = top.clamp(0, OVERLAY_SIZE);
    let bottom = bottom.clamp(0, OVERLAY_SIZE);
    let premultiply = |channel: u8| ((channel as u16 * alpha as u16) / 255) as u32;
    let pixel = ((alpha as u32) << 24)
        | (premultiply(rgb.0) << 16)
        | (premultiply(rgb.1) << 8)
        | premultiply(rgb.2);
    for y in top..bottom {
        let row = y as usize * OVERLAY_SIZE as usize;
        for x in left..right {
            pixels[row + x as usize] = pixel;
        }
    }
}

fn parse_color(value: &str) -> (u8, u8, u8) {
    let Some(hex) = value.strip_prefix('#').filter(|hex| hex.len() == 6) else {
        return (255, 255, 255);
    };
    let channel = |start| u8::from_str_radix(&hex[start..start + 2], 16);
    match (channel(0), channel(2), channel(4)) {
        (Ok(red), Ok(green), Ok(blue)) => (red, green, blue),
        _ => (255, 255, 255),
    }
}
