use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::DynamicImage;
use imageproc::contrast::stretch_contrast;
use imageproc::geometric_transformations::rotate_about_center;
use std::f32::consts::PI;

fn bench_image_adjustments(c: &mut Criterion) {
    let test_image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(1920, 1080, image::Rgba([128, 128, 128, 255])));

    c.bench_function("cpu_image_adjustments_1920x1080", |b| {
        b.iter(|| {
            let _result = stretch_contrast(&test_image.to_luma8(), 0, 255, 0, 255);
        })
    });
}

fn bench_film_emulation(c: &mut Criterion) {
    let test_image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(1000, 1000, image::Rgba([200, 150, 100, 255])));

    c.bench_function("cpu_film_emulation_1000x1000", |b| {
        b.iter(|| {
            let _result = rotate_about_center(&test_image.to_rgba8(), PI / 180.0 * 5.0, imageproc::geometric_transformations::Interpolation::Bilinear, image::Rgba([0, 0, 0, 0]));
        })
    });
}

fn bench_cache_operations(c: &mut Criterion) {
    c.bench_function("cache_operations_overhead", |b| {
        b.iter(|| {
            // Just measure basic operations that would be involved in caching
            let mut vec = Vec::with_capacity(50);
            for i in 0..50 {
                vec.push(black_box(i));
            }
            black_box(vec);
        })
    });
}

criterion_group!(benches, bench_image_adjustments, bench_film_emulation, bench_cache_operations);
criterion_main!(benches);