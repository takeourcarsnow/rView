use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Rgba};
use num_cpus;
use rayon::prelude::*;

use super::film_emulation::ImageAdjustments;

// ============ ACES FILMIC TONE MAPPING ============
// Based on the ACES (Academy Color Encoding System) RRT+ODT approximation
// Provides better highlight rolloff and more cinematic look

/// ACES filmic tone mapping curve (approximation of RRT + ODT)
/// This provides natural highlight compression and shadow lift
#[inline]
fn aces_tonemap(x: f32) -> f32 {
    // Attempt to simulate Stephen Hill's fit of ACES
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    let x = x.max(0.0);
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

/// Apply ACES tone mapping with adjustable strength
#[inline]
fn apply_aces_tonemap(r: f32, g: f32, b: f32, strength: f32) -> (f32, f32, f32) {
    if strength <= 0.0 {
        return (r, g, b);
    }
    let r_mapped = aces_tonemap(r);
    let g_mapped = aces_tonemap(g);
    let b_mapped = aces_tonemap(b);
    // Blend between linear and ACES based on strength
    (
        r * (1.0 - strength) + r_mapped * strength,
        g * (1.0 - strength) + g_mapped * strength,
        b * (1.0 - strength) + b_mapped * strength,
    )
}

// ============ OKLAB COLOR SPACE ============
// OKLab is a perceptually uniform color space, much better for saturation/vibrance
// Based on Björn Ottosson's work: https://bottosson.github.io/posts/oklab/

/// Convert linear sRGB to OKLab
#[inline]
fn linear_srgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // sRGB to linear LMS
    let l = 0.41222146 * r + 0.53633255 * g + 0.051445995 * b;
    let m = 0.2119035 * r + 0.6806995 * g + 0.10739696 * b;
    let s = 0.08830246 * r + 0.28171885 * g + 0.6299787 * b;

    // Cube root (approximate pow(x, 1/3))
    let l_ = l.max(0.0).cbrt();
    let m_ = m.max(0.0).cbrt();
    let s_ = s.max(0.0).cbrt();

    // LMS to OKLab
    let lab_l = 0.21045426 * l_ + 0.7936178 * m_ - 0.004072047 * s_;
    let lab_a = 1.9779985 * l_ - 2.4285922 * m_ + 0.4505937 * s_;
    let lab_b = 0.025904037 * l_ + 0.78277177 * m_ - 0.80867577 * s_;

    (lab_l, lab_a, lab_b)
}

/// Convert OKLab to linear sRGB
#[inline]
fn oklab_to_linear_srgb(lab_l: f32, lab_a: f32, lab_b: f32) -> (f32, f32, f32) {
    // OKLab to LMS
    let l_ = lab_l + 0.39633778 * lab_a + 0.21580376 * lab_b;
    let m_ = lab_l - 0.105561346 * lab_a - 0.06385417 * lab_b;
    let s_ = lab_l - 0.08948418 * lab_a - 1.2914855 * lab_b;

    // Cube (inverse of cbrt)
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    // LMS to linear sRGB
    let r = 4.0767417 * l - 3.3077116 * m + 0.23096994 * s;
    let g = -1.268438 * l + 2.6097574 * m - 0.34131938 * s;
    let b = -0.0041960863 * l - 0.7034186 * m + 1.7076147 * s;

    (r, g, b)
}

/// sRGB gamma to linear conversion
#[inline]
fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear to sRGB gamma conversion
#[inline]
fn linear_to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// Apply saturation in OKLab color space (perceptually uniform)
#[inline]
fn apply_oklab_saturation(r: f32, g: f32, b: f32, saturation: f32) -> (f32, f32, f32) {
    // Convert to linear sRGB first
    let r_lin = srgb_to_linear(r);
    let g_lin = srgb_to_linear(g);
    let b_lin = srgb_to_linear(b);

    // Convert to OKLab
    let (lab_l, lab_a, lab_b) = linear_srgb_to_oklab(r_lin, g_lin, b_lin);

    // Scale chroma (a and b channels) by saturation factor
    let lab_a = lab_a * saturation;
    let lab_b = lab_b * saturation;

    // Convert back to linear sRGB
    let (r_out, g_out, b_out) = oklab_to_linear_srgb(lab_l, lab_a, lab_b);

    // Convert back to sRGB gamma
    (
        linear_to_srgb(r_out.clamp(0.0, 1.0)),
        linear_to_srgb(g_out.clamp(0.0, 1.0)),
        linear_to_srgb(b_out.clamp(0.0, 1.0)),
    )
}

/// Apply vibrance in OKLab (protects already-saturated colors and skin tones)
#[inline]
#[allow(dead_code)]
fn apply_oklab_vibrance(r: f32, g: f32, b: f32, vibrance: f32) -> (f32, f32, f32) {
    // Convert to linear sRGB
    let r_lin = srgb_to_linear(r);
    let g_lin = srgb_to_linear(g);
    let b_lin = srgb_to_linear(b);

    // Convert to OKLab
    let (lab_l, lab_a, lab_b) = linear_srgb_to_oklab(r_lin, g_lin, b_lin);

    // Calculate current chroma
    let chroma = (lab_a * lab_a + lab_b * lab_b).sqrt();

    // Vibrance effect: less saturated colors get more boost
    // Also protect skin tones (orange-ish hues)
    let hue = lab_b.atan2(lab_a);
    let skin_hue_center = 0.7; // approximate skin tone hue in OKLab
    let skin_protection =
        1.0 - (((hue - skin_hue_center).abs() / std::f32::consts::PI).min(1.0) * 0.5);

    // Less saturated colors get more boost (inverse relationship)
    let saturation_factor = (1.0 - chroma.min(0.5) * 2.0).max(0.0);

    // Combined vibrance factor
    let effective_vibrance = vibrance * saturation_factor * (1.0 - skin_protection * 0.3);
    let factor = 1.0 + effective_vibrance;

    let lab_a = lab_a * factor;
    let lab_b = lab_b * factor;

    // Convert back to linear sRGB
    let (r_out, g_out, b_out) = oklab_to_linear_srgb(lab_l, lab_a, lab_b);

    // Convert back to sRGB gamma
    (
        linear_to_srgb(r_out.clamp(0.0, 1.0)),
        linear_to_srgb(g_out.clamp(0.0, 1.0)),
        linear_to_srgb(b_out.clamp(0.0, 1.0)),
    )
}

pub fn apply_adjustments(image: &DynamicImage, adj: &ImageAdjustments) -> DynamicImage {
    if adj.is_default() {
        return image.clone();
    }

    let mut img = image.to_rgba8();
    let (width, height) = img.dimensions();

    // Pre-calculate common values
    let exposure_mult = 2.0_f32.powf(adj.exposure);
    let sat_factor = adj.saturation;
    let temp_r_add = if adj.temperature > 0.0 {
        adj.temperature * 25.5
    } else {
        adj.temperature * 15.3
    };
    let temp_b_sub = if adj.temperature > 0.0 {
        adj.temperature * 15.3
    } else {
        adj.temperature * 25.5
    };

    // Pre-generate grain texture for consistent grain pattern
    let grain_seed = 12345u64;

    // Calculate center for vignette
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_dist = (center_x * center_x + center_y * center_y).sqrt();

    // Process pixels in parallel
    process_pixels_parallel(
        &mut img,
        adj,
        exposure_mult,
        sat_factor,
        temp_r_add,
        temp_b_sub,
        grain_seed,
        center_x,
        center_y,
        max_dist,
    );

    // Apply frame if enabled
    apply_frame_if_needed(img, adj)
}

fn process_pixels_parallel(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    adj: &ImageAdjustments,
    exposure_mult: f32,
    sat_factor: f32,
    temp_r_add: f32,
    temp_b_sub: f32,
    grain_seed: u64,
    center_x: f32,
    center_y: f32,
    max_dist: f32,
) {
    let (width, height) = img.dimensions();
    let mut samples = img.as_flat_samples_mut();
    let raw_pixels = samples.as_mut_slice();

    // Calculate bytes per chunk, ensuring it's aligned to 4-byte pixel boundaries
    let pixel_count = (width * height) as usize;
    let pixels_per_thread = (pixel_count / num_cpus::get()).max(1);
    let bytes_per_chunk = pixels_per_thread * 4; // 4 bytes per RGBA pixel

    raw_pixels
        .par_chunks_mut(bytes_per_chunk)
        .enumerate()
        .for_each(|(chunk_idx, chunk)| {
            let chunk_start_pixel = chunk_idx * pixels_per_thread;

            for (local_idx, pixel) in chunk.chunks_mut(4).enumerate() {
                if pixel.len() < 4 {
                    continue;
                } // Skip incomplete pixels

                let pixel_idx = chunk_start_pixel + local_idx;
                let px = (pixel_idx % width as usize) as f32;
                let py = (pixel_idx / width as usize) as f32;

                process_single_pixel(
                    pixel,
                    px,
                    py,
                    adj,
                    exposure_mult,
                    sat_factor,
                    temp_r_add,
                    temp_b_sub,
                    grain_seed,
                    center_x,
                    center_y,
                    max_dist,
                );
            }
        });
}

fn process_single_pixel(
    pixel: &mut [u8],
    px: f32,
    py: f32,
    adj: &ImageAdjustments,
    exposure_mult: f32,
    sat_factor: f32,
    temp_r_add: f32,
    temp_b_sub: f32,
    grain_seed: u64,
    center_x: f32,
    center_y: f32,
    max_dist: f32,
) {
    let mut r = pixel[0] as f32 / 255.0;
    let mut g = pixel[1] as f32 / 255.0;
    let mut b = pixel[2] as f32 / 255.0;
    let a = pixel[3] as f32;

    // Apply film emulation first
    if adj.film.enabled {
        apply_film_emulation(&mut r, &mut g, &mut b, &adj.film);
    }

    // Convert to 0-255 range for standard adjustments
    r *= 255.0;
    g *= 255.0;
    b *= 255.0;

    // Apply standard adjustments
    apply_standard_adjustments(
        &mut r,
        &mut g,
        &mut b,
        exposure_mult,
        sat_factor,
        temp_r_add,
        temp_b_sub,
        adj,
    );

    // Apply film post-processing
    if adj.film.enabled {
        apply_film_post_processing(
            &mut r,
            &mut g,
            &mut b,
            px,
            py,
            &adj.film,
            grain_seed,
            center_x,
            center_y,
            max_dist,
        );
    }

    // Clamp values
    pixel[0] = r.clamp(0.0, 255.0) as u8;
    pixel[1] = g.clamp(0.0, 255.0) as u8;
    pixel[2] = b.clamp(0.0, 255.0) as u8;
    pixel[3] = a as u8; // Alpha unchanged
}

fn apply_film_emulation(r: &mut f32, g: &mut f32, b: &mut f32, film: &super::film_emulation::FilmEmulation) {
    // B&W conversion
    if film.is_bw {
        let luminance = 0.30 * *r + 0.59 * *g + 0.11 * *b;
        *r = luminance;
        *g = luminance;
        *b = luminance;
    }

    // Color channel crossover
    if !film.is_bw {
        let orig_r = *r;
        let orig_g = *g;
        let orig_b = *b;
        *r = orig_r
            + orig_g * film.color_crossover.green_in_red
            + orig_b * film.color_crossover.blue_in_red;
        *g = orig_g
            + orig_r * film.color_crossover.red_in_green
            + orig_b * film.color_crossover.blue_in_green;
        *b = orig_b
            + orig_r * film.color_crossover.red_in_blue
            + orig_g * film.color_crossover.green_in_blue;
    }

    // Per-channel gamma
    *r = r.max(0.0).powf(film.color_gamma.red);
    *g = g.max(0.0).powf(film.color_gamma.green);
    *b = b.max(0.0).powf(film.color_gamma.blue);

    // Film latitude
    if film.latitude > 0.0 {
        let latitude_factor = film.latitude * 0.5;
        *r = *r / (1.0 + *r * latitude_factor);
        *g = *g / (1.0 + *g * latitude_factor);
        *b = *b / (1.0 + *b * latitude_factor);
        let comp = 1.0 + latitude_factor * 0.5;
        *r *= comp;
        *g *= comp;
        *b *= comp;
    }

    // Tone curve
    if film.tone.s_curve_strength > 0.0 {
        let s = film.tone.s_curve_strength;
        *r = apply_s_curve(*r, s);
        *g = apply_s_curve(*g, s);
        *b = apply_s_curve(*b, s);
    }

    // Tone curve control points
    *r = apply_tone_curve(*r, film.tone.shadows, film.tone.midtones, film.tone.highlights);
    *g = apply_tone_curve(*g, film.tone.shadows, film.tone.midtones, film.tone.highlights);
    *b = apply_tone_curve(*b, film.tone.shadows, film.tone.midtones, film.tone.highlights);

    // Black point and white point
    let bp = film.black_point;
    let wp = film.white_point;
    let range = wp - bp;
    if range > 0.01 {
        *r = bp + *r * range;
        *g = bp + *g * range;
        *b = bp + *b * range;
    }

    // Shadow and highlight tinting
    let luminance = 0.299 * *r + 0.587 * *g + 0.114 * *b;
    let shadow_amount = (1.0 - luminance * 2.0).clamp(0.0, 1.0);
    let highlight_amount = ((luminance - 0.5) * 2.0).clamp(0.0, 1.0);

    *r += film.shadow_tint[0] * shadow_amount + film.highlight_tint[0] * highlight_amount;
    *g += film.shadow_tint[1] * shadow_amount + film.highlight_tint[1] * highlight_amount;
    *b += film.shadow_tint[2] * shadow_amount + film.highlight_tint[2] * highlight_amount;
}

fn apply_standard_adjustments(
    r: &mut f32,
    g: &mut f32,
    b: &mut f32,
    exposure_mult: f32,
    sat_factor: f32,
    temp_r_add: f32,
    temp_b_sub: f32,
    adj: &ImageAdjustments,
) {
    // Apply exposure
    *r *= exposure_mult;
    *g *= exposure_mult;
    *b *= exposure_mult;

    // Temperature
    *r += temp_r_add;
    *b -= temp_b_sub;

    // ACES tone mapping
    apply_aces_tone_mapping(r, g, b, exposure_mult);

    // Saturation (skip for B&W film)
    if (!adj.film.enabled || !adj.film.is_bw) && (sat_factor - 1.0).abs() > 0.001 {
        apply_saturation(r, g, b, sat_factor);
    }
}

fn apply_aces_tone_mapping(r: &mut f32, g: &mut f32, b: &mut f32, exposure_mult: f32) {
    let r_norm = (*r / 255.0).max(0.0);
    let g_norm = (*g / 255.0).max(0.0);
    let b_norm = (*b / 255.0).max(0.0);

    let aces_strength = 0.5 + (exposure_mult - 1.0).abs() * 0.3;
    let aces_strength = aces_strength.clamp(0.3, 0.9);

    let (r_tm, g_tm, b_tm) = apply_aces_tonemap(r_norm, g_norm, b_norm, aces_strength);
    *r = r_tm * 255.0;
    *g = g_tm * 255.0;
    *b = b_tm * 255.0;
}

fn apply_saturation(r: &mut f32, g: &mut f32, b: &mut f32, sat_factor: f32) {
    let r_norm = (*r / 255.0).clamp(0.0, 1.0);
    let g_norm = (*g / 255.0).clamp(0.0, 1.0);
    let b_norm = (*b / 255.0).clamp(0.0, 1.0);

    let (r_sat, g_sat, b_sat) = apply_oklab_saturation(r_norm, g_norm, b_norm, sat_factor);

    *r = r_sat * 255.0;
    *g = g_sat * 255.0;
    *b = b_sat * 255.0;
}

fn apply_film_post_processing(
    r: &mut f32,
    g: &mut f32,
    b: &mut f32,
    px: f32,
    py: f32,
    film: &super::film_emulation::FilmEmulation,
    grain_seed: u64,
    center_x: f32,
    center_y: f32,
    max_dist: f32,
) {
    // Vignette
    if film.vignette.amount > 0.0 {
        let dx = px - center_x;
        let dy = py - center_y;
        let dist = (dx * dx + dy * dy).sqrt() / max_dist;
        let vignette = 1.0 - film.vignette.amount * (dist / film.vignette.softness).powf(2.0);
        let vignette = vignette.clamp(0.0, 1.0);
        *r *= vignette;
        *g *= vignette;
        *b *= vignette;
    }

    // Film grain
    if film.grain.amount > 0.0 {
        let grain = generate_film_grain(px as u32, py as u32, grain_seed, film.grain.size, film.grain.roughness);
        let lum = (0.299 * *r + 0.587 * *g + 0.114 * *b) / 255.0;
        let grain_mask = 4.0 * lum * (1.0 - lum);
        let grain_strength = film.grain.amount * 255.0 * 0.15 * grain_mask;

        *r += grain * grain_strength;
        *g += grain * grain_strength;
        *b += grain * grain_strength;
    }

    // Halation
    if film.halation.amount > 0.0 {
        let luminance = (0.299 * *r + 0.587 * *g + 0.114 * *b) / 255.0;
        let halation_mask = ((luminance - 0.7) / 0.3).clamp(0.0, 1.0);
        let halation_strength = film.halation.amount * halation_mask * 30.0;
        *r += film.halation.color[0] * halation_strength;
        *g += film.halation.color[1] * halation_strength;
        *b += film.halation.color[2] * halation_strength;
    }
}

fn apply_frame_if_needed(img: ImageBuffer<Rgba<u8>, Vec<u8>>, adj: &ImageAdjustments) -> DynamicImage {
    if adj.frame_enabled && adj.frame_thickness > 0.0 {
        let (width, height) = img.dimensions();
        let thickness = adj.frame_thickness as u32;
        let new_width = width + 2 * thickness;
        let new_height = height + 2 * thickness;

        let mut framed = ImageBuffer::new(new_width, new_height);

        // Fill with frame color
        let frame_r = (adj.frame_color[0] * 255.0) as u8;
        let frame_g = (adj.frame_color[1] * 255.0) as u8;
        let frame_b = (adj.frame_color[2] * 255.0) as u8;

        for pixel in framed.pixels_mut() {
            *pixel = Rgba([frame_r, frame_g, frame_b, 255]);
        }

        // Copy original image to center
        imageops::overlay(&mut framed, &img, thickness as i64, thickness as i64);

        DynamicImage::ImageRgba8(framed)
    } else {
        DynamicImage::ImageRgba8(img)
    }
}

/// Apply S-curve contrast enhancement (film characteristic curve)
#[inline]
fn apply_s_curve(x: f32, strength: f32) -> f32 {
    // Attempt to simulate Hurter-Driffield (H&D) curve
    let x = x.clamp(0.0, 1.0);
    let midpoint = 0.5;
    let steepness = 1.0 + strength * 3.0;

    // Sigmoid function centered at midpoint
    let sigmoid = 1.0 / (1.0 + (-steepness * (x - midpoint)).exp());
    // Normalize to 0-1 range
    let min_sig = 1.0 / (1.0 + (steepness * midpoint).exp());
    let max_sig = 1.0 / (1.0 + (-steepness * (1.0 - midpoint)).exp());

    let normalized = (sigmoid - min_sig) / (max_sig - min_sig);
    // Blend between linear and S-curve based on strength
    x * (1.0 - strength) + normalized * strength
}

/// Apply tone curve adjustments for shadows, midtones, and highlights
#[inline]
fn apply_tone_curve(x: f32, shadows: f32, midtones: f32, highlights: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);

    // Shadow region (0-0.33)
    // Midtone region (0.33-0.66)
    // Highlight region (0.66-1.0)

    let shadow_weight = (1.0 - x * 3.0).clamp(0.0, 1.0);
    let highlight_weight = ((x - 0.66) * 3.0).clamp(0.0, 1.0);
    let midtone_weight = 1.0 - shadow_weight - highlight_weight;

    // Apply adjustments weighted by region
    let adjustment = shadows * shadow_weight * 0.15
        + midtones * midtone_weight * 0.1
        + highlights * highlight_weight * 0.15;

    (x + adjustment).clamp(0.0, 1.0)
}

/// Generate film grain using pseudo-random noise
#[inline]
fn generate_film_grain(x: u32, y: u32, seed: u64, size: f32, roughness: f32) -> f32 {
    // Scale coordinates by grain size
    let scale = 1.0 / size;
    let sx = (x as f32 * scale) as u32;
    let sy = (y as f32 * scale) as u32;

    // Simple hash function for pseudo-random values
    let mut hash = seed;
    hash ^= sx as u64;
    hash = hash.wrapping_mul(0x517cc1b727220a95);
    hash ^= sy as u64;
    hash = hash.wrapping_mul(0x517cc1b727220a95);
    hash ^= hash >> 32;

    // Convert to -1 to 1 range
    let noise = (hash as f32 / u64::MAX as f32) * 2.0 - 1.0;

    // Add roughness variation (multi-octave noise approximation)
    let mut rough_noise = noise;
    if roughness > 0.0 {
        hash = hash.wrapping_mul(0x517cc1b727220a95);
        let noise2 = (hash as f32 / u64::MAX as f32) * 2.0 - 1.0;
        rough_noise = noise * (1.0 - roughness * 0.5) + noise2 * roughness * 0.5;
    }

    rough_noise
}

/// Apply basic adjustments to a thumbnail (simplified version without film emulation or parallel processing)
/// This avoids glitches that can occur with parallel chunk processing on small images
pub fn apply_adjustments_thumbnail(image: &DynamicImage, adj: &ImageAdjustments) -> DynamicImage {
    if adj.is_default() {
        return image.clone();
    }

    let mut img = image.to_rgba8();
    let (width, height) = img.dimensions();

    // Pre-calculate adjustment factors
    let exposure_mult = 2.0_f32.powf(adj.exposure);
    let sat_factor = adj.saturation;
    let temp_r_add = if adj.temperature > 0.0 {
        adj.temperature * 25.5
    } else {
        adj.temperature * 15.3
    };
    let temp_b_sub = if adj.temperature > 0.0 {
        adj.temperature * 15.3
    } else {
        adj.temperature * 25.5
    };

    // Process each pixel sequentially (safe for small images)
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let mut r = pixel[0] as f32;
            let mut g = pixel[1] as f32;
            let mut b = pixel[2] as f32;
            let a = pixel[3];

            // Apply exposure
            r *= exposure_mult;
            g *= exposure_mult;
            b *= exposure_mult;

            // Temperature
            r += temp_r_add;
            b -= temp_b_sub;

            // Saturation
            let gray = 0.299 * r + 0.587 * g + 0.114 * b;
            r = gray + (r - gray) * sat_factor;
            g = gray + (g - gray) * sat_factor;
            b = gray + (b - gray) * sat_factor;

            // Film B&W conversion if enabled
            if adj.film.enabled && adj.film.is_bw {
                let luminance = 0.30 * r + 0.59 * g + 0.11 * b;
                r = luminance;
                g = luminance;
                b = luminance;
            }

            // Clamp values
            let r = r.clamp(0.0, 255.0) as u8;
            let g = g.clamp(0.0, 255.0) as u8;
            let b = b.clamp(0.0, 255.0) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, a]));
        }
    }

    DynamicImage::ImageRgba8(img)
}

// Rotate image losslessly (for JPEG, just update EXIF, for others, actually rotate)
pub fn rotate_image(image: &DynamicImage, degrees: i32) -> DynamicImage {
    match degrees {
        90 | -270 => image.rotate90(),
        180 | -180 => image.rotate180(),
        270 | -90 => image.rotate270(),
        _ => image.clone(),
    }
}

// Crop image to specified rectangle (x, y, width, height)
pub fn crop_image(image: &DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
    let (img_width, img_height) = image.dimensions();

    // Ensure crop rectangle is within image bounds
    let crop_x = if x < img_width {
        x
    } else {
        img_width.saturating_sub(1)
    };
    let crop_y = if y < img_height {
        y
    } else {
        img_height.saturating_sub(1)
    };
    let crop_width = if crop_x + width <= img_width {
        width
    } else {
        img_width.saturating_sub(crop_x)
    };
    let crop_height = if crop_y + height <= img_height {
        height
    } else {
        img_height.saturating_sub(crop_y)
    };

    if crop_width == 0 || crop_height == 0 {
        return image.clone();
    }

    image.crop_imm(crop_x, crop_y, crop_width, crop_height)
}
