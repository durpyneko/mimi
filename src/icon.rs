use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut};
use imageproc::rect::Rect;

pub fn render_icon(battery: Option<u8>) -> Vec<u8> {
    // create transparent canvas
    let mut img = RgbaImage::new(32, 32);
    for px in img.pixels_mut() {
        *px = Rgba([0, 0, 0, 0]);
    }

    let color = match battery {
        Some(level) => battery_color(level),
        None => Rgba([120, 120, 120, 255]), // gray, disconnected
    };

    // battery body outline
    draw_hollow_rect_mut(&mut img, Rect::at(2, 8).of_size(26, 16), color);

    // battery temrinal nub
    draw_filled_rect_mut(&mut img, Rect::at(28, 12).of_size(3, 8), color);

    // battery fill
    if let Some(level) = battery {
        let fill_w = ((level as u32 * 22) / 100).max(1);
        draw_filled_rect_mut(&mut img, Rect::at(4, 10).of_size(fill_w, 12), color);
    }

    // SNI requires ARGB32
    img.pixels()
        .flat_map(|p| {
            let [r, g, b, a] = p.0;
            [a, r, g, b]
        })
        .collect()
}

fn battery_color(level: u8) -> Rgba<u8> {
    match level {
        0..=20 => Rgba([220, 50, 50, 255]),   // red
        21..=50 => Rgba([220, 180, 50, 255]), // yellow
        _ => Rgba([80, 200, 80, 255]),        // green
    }
}
