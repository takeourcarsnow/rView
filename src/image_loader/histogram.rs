use image::DynamicImage;

// Calculate histogram
pub fn calculate_histogram(image: &DynamicImage) -> Vec<Vec<u32>> {
    let rgb = image.to_rgb8();
    let mut histogram = vec![vec![0u32; 256]; 3];

    for pixel in rgb.pixels() {
        histogram[0][pixel[0] as usize] += 1;
        histogram[1][pixel[1] as usize] += 1;
        histogram[2][pixel[2] as usize] += 1;
    }

    histogram
}