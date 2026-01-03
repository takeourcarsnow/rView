use image::{DynamicImage, Rgba, RgbaImage};

// Focus peaking - detect edges/sharp areas
pub fn generate_focus_peaking_overlay(image: &DynamicImage, threshold: f32) -> RgbaImage {
    let gray = image.to_luma8();
    let (width, height) = gray.dimensions();
    let mut overlay = RgbaImage::new(width, height);

    // Sobel edge detection
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let gx = -(gray.get_pixel(x - 1, y - 1).0[0] as f32)
                + 1.0 * gray.get_pixel(x + 1, y - 1).0[0] as f32
                + -2.0 * gray.get_pixel(x - 1, y).0[0] as f32
                + 2.0 * gray.get_pixel(x + 1, y).0[0] as f32
                + -(gray.get_pixel(x - 1, y + 1).0[0] as f32)
                + 1.0 * gray.get_pixel(x + 1, y + 1).0[0] as f32;

            let gy = -(gray.get_pixel(x - 1, y - 1).0[0] as f32)
                + -2.0 * gray.get_pixel(x, y - 1).0[0] as f32
                + -(gray.get_pixel(x + 1, y - 1).0[0] as f32)
                + 1.0 * gray.get_pixel(x - 1, y + 1).0[0] as f32
                + 2.0 * gray.get_pixel(x, y + 1).0[0] as f32
                + 1.0 * gray.get_pixel(x + 1, y + 1).0[0] as f32;

            let magnitude = (gx * gx + gy * gy).sqrt();

            if magnitude > threshold {
                overlay.put_pixel(x, y, Rgba([255, 0, 0, 200]));
            } else {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    overlay
}

// Zebra pattern for overexposure
pub fn generate_zebra_overlay(
    image: &DynamicImage,
    high_threshold: u8,
    low_threshold: u8,
) -> RgbaImage {
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let mut overlay = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x, y);
            let max_val = pixel.0[0].max(pixel.0[1]).max(pixel.0[2]);
            let min_val = pixel.0[0].min(pixel.0[1]).min(pixel.0[2]);

            // Zebra stripes pattern
            let stripe = ((x + y) / 4) % 2 == 0;

            if max_val >= high_threshold && stripe {
                // Overexposed - red stripes
                overlay.put_pixel(x, y, Rgba([255, 0, 0, 180]));
            } else if min_val <= low_threshold && stripe {
                // Underexposed - blue stripes
                overlay.put_pixel(x, y, Rgba([0, 0, 255, 180]));
            } else {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    overlay
}
