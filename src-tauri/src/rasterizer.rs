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
        let dot_half = settings.dot_size / 2;
        if settings.outline {
            let radius = dot_half + settings.outline_thickness;
            fill_rect(
                pixels,
                center - radius,
                center - radius,
                center + radius + 1,
                center + radius + 1,
                outline_color,
                alpha,
            );
        }
        fill_rect(
            pixels,
            center - dot_half,
            center - dot_half,
            center - dot_half + settings.dot_size,
            center - dot_half + settings.dot_size,
            color,
            alpha,
        );
    }
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
