// RAW image demosaicing compute shader
// Implements Adaptive Homogeneity-Directed (AHD) interpolation for high-quality demosaicing
@group(0) @binding(0) var<storage, read> raw_data: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_pixels: array<u32>;
@group(0) @binding(2) var<uniform> params: DemosaicParams;

struct DemosaicParams {
    width: u32,
    height: u32,
    bayer_pattern: u32, // 0=RGGB, 1=GRBG, 2=GBRG, 3=BGGR
    black_level: vec4<f32>,
    white_level: vec4<f32>,
    color_matrix: mat3x3<f32>,
    gamma: f32,
};

// Helper function to get raw pixel value with bounds checking
fn get_raw_value(px: u32, py: u32) -> f32 {
    let cx = clamp(px, 0u, params.width - 1u);
    let cy = clamp(py, 0u, params.height - 1u);
    let idx = cy * params.width + cx;
    return f32(raw_data[idx] & 0xFFFFu);
}

// Get the color at a Bayer position (0=R, 1=G on R row, 2=G on B row, 3=B)
fn get_bayer_color(x: u32, y: u32) -> u32 {
    let pattern_x = x % 2u;
    let pattern_y = y % 2u;
    let bayer_idx = pattern_y * 2u + pattern_x;
    return (bayer_idx + params.bayer_pattern) % 4u;
}

// ============ AHD (Adaptive Homogeneity-Directed) DEMOSAICING ============
// This algorithm provides much better edge handling than bilinear interpolation
// by choosing between horizontal and vertical interpolation based on local homogeneity

// Horizontal interpolation for green channel
fn interpolate_green_h(x: u32, y: u32) -> f32 {
    let color = get_bayer_color(x, y);
    if (color == 1u || color == 2u) {
        // Already a green pixel
        return get_raw_value(x, y);
    }
    
    // Interpolate horizontally using gradient-weighted average
    let g_left = get_raw_value(x - 1u, y);
    let g_right = get_raw_value(x + 1u, y);
    let center = get_raw_value(x, y);
    let left2 = get_raw_value(x - 2u, y);
    let right2 = get_raw_value(x + 2u, y);
    
    // Second-order gradient correction
    let gradient_h = abs(left2 - center) + abs(center - right2);
    let base = (g_left + g_right) * 0.5;
    let correction = (2.0 * center - left2 - right2) * 0.25;
    
    return base + correction * (1.0 / (1.0 + gradient_h * 0.001));
}

// Vertical interpolation for green channel
fn interpolate_green_v(x: u32, y: u32) -> f32 {
    let color = get_bayer_color(x, y);
    if (color == 1u || color == 2u) {
        // Already a green pixel
        return get_raw_value(x, y);
    }
    
    // Interpolate vertically using gradient-weighted average
    let g_top = get_raw_value(x, y - 1u);
    let g_bottom = get_raw_value(x, y + 1u);
    let center = get_raw_value(x, y);
    let top2 = get_raw_value(x, y - 2u);
    let bottom2 = get_raw_value(x, y + 2u);
    
    // Second-order gradient correction
    let gradient_v = abs(top2 - center) + abs(center - bottom2);
    let base = (g_top + g_bottom) * 0.5;
    let correction = (2.0 * center - top2 - bottom2) * 0.25;
    
    return base + correction * (1.0 / (1.0 + gradient_v * 0.001));
}

// Calculate homogeneity in a direction (lower = more homogeneous = better)
fn calc_homogeneity_h(x: u32, y: u32, g: f32) -> f32 {
    var sum = 0.0;
    for (var dx: i32 = -1; dx <= 1; dx += 1) {
        for (var dy: i32 = -1; dy <= 1; dy += 1) {
            let nx = u32(max(0, min(i32(params.width) - 1, i32(x) + dx)));
            let ny = u32(max(0, min(i32(params.height) - 1, i32(y) + dy)));
            let ng = interpolate_green_h(nx, ny);
            sum += abs(g - ng);
        }
    }
    return sum;
}

fn calc_homogeneity_v(x: u32, y: u32, g: f32) -> f32 {
    var sum = 0.0;
    for (var dx: i32 = -1; dx <= 1; dx += 1) {
        for (var dy: i32 = -1; dy <= 1; dy += 1) {
            let nx = u32(max(0, min(i32(params.width) - 1, i32(x) + dx)));
            let ny = u32(max(0, min(i32(params.height) - 1, i32(y) + dy)));
            let ng = interpolate_green_v(nx, ny);
            sum += abs(g - ng);
        }
    }
    return sum;
}

// AHD demosaicing - adaptively choose between horizontal and vertical interpolation
fn demosaic_ahd(x: u32, y: u32) -> vec3<f32> {
    let color = get_bayer_color(x, y);
    let raw = get_raw_value(x, y);
    
    var r: f32;
    var g: f32;
    var b: f32;
    
    // First, interpolate green using AHD
    let g_h = interpolate_green_h(x, y);
    let g_v = interpolate_green_v(x, y);
    
    // Calculate homogeneity for both directions
    let h_h = calc_homogeneity_h(x, y, g_h);
    let h_v = calc_homogeneity_v(x, y, g_v);
    
    // Choose the direction with lower homogeneity (more uniform = better)
    // Blend slightly to avoid harsh transitions
    let blend = clamp((h_v - h_h) / (h_h + h_v + 0.001), -1.0, 1.0) * 0.5 + 0.5;
    g = g_h * blend + g_v * (1.0 - blend);
    
    // Now interpolate R and B using color difference method (more robust than direct interpolation)
    if (color == 0u) {
        // Red pixel
        r = raw;
        // Blue: use color difference interpolation
        let b_nw = get_raw_value(x - 1u, y - 1u);
        let b_ne = get_raw_value(x + 1u, y - 1u);
        let b_sw = get_raw_value(x - 1u, y + 1u);
        let b_se = get_raw_value(x + 1u, y + 1u);
        let g_nw = interpolate_green_h(x - 1u, y - 1u) * 0.5 + interpolate_green_v(x - 1u, y - 1u) * 0.5;
        let g_ne = interpolate_green_h(x + 1u, y - 1u) * 0.5 + interpolate_green_v(x + 1u, y - 1u) * 0.5;
        let g_sw = interpolate_green_h(x - 1u, y + 1u) * 0.5 + interpolate_green_v(x - 1u, y + 1u) * 0.5;
        let g_se = interpolate_green_h(x + 1u, y + 1u) * 0.5 + interpolate_green_v(x + 1u, y + 1u) * 0.5;
        let diff_avg = ((b_nw - g_nw) + (b_ne - g_ne) + (b_sw - g_sw) + (b_se - g_se)) * 0.25;
        b = g + diff_avg;
    } else if (color == 1u) {
        // Green pixel on red row
        g = raw;
        let r_left = get_raw_value(x - 1u, y);
        let r_right = get_raw_value(x + 1u, y);
        let g_left = interpolate_green_h(x - 1u, y);
        let g_right = interpolate_green_h(x + 1u, y);
        r = g + ((r_left - g_left) + (r_right - g_right)) * 0.5;
        
        let b_top = get_raw_value(x, y - 1u);
        let b_bottom = get_raw_value(x, y + 1u);
        let g_top = interpolate_green_v(x, y - 1u);
        let g_bottom = interpolate_green_v(x, y + 1u);
        b = g + ((b_top - g_top) + (b_bottom - g_bottom)) * 0.5;
    } else if (color == 2u) {
        // Green pixel on blue row
        g = raw;
        let b_left = get_raw_value(x - 1u, y);
        let b_right = get_raw_value(x + 1u, y);
        let g_left = interpolate_green_h(x - 1u, y);
        let g_right = interpolate_green_h(x + 1u, y);
        b = g + ((b_left - g_left) + (b_right - g_right)) * 0.5;
        
        let r_top = get_raw_value(x, y - 1u);
        let r_bottom = get_raw_value(x, y + 1u);
        let g_top = interpolate_green_v(x, y - 1u);
        let g_bottom = interpolate_green_v(x, y + 1u);
        r = g + ((r_top - g_top) + (r_bottom - g_bottom)) * 0.5;
    } else {
        // Blue pixel
        b = raw;
        // Red: use color difference interpolation
        let r_nw = get_raw_value(x - 1u, y - 1u);
        let r_ne = get_raw_value(x + 1u, y - 1u);
        let r_sw = get_raw_value(x - 1u, y + 1u);
        let r_se = get_raw_value(x + 1u, y + 1u);
        let g_nw = interpolate_green_h(x - 1u, y - 1u) * 0.5 + interpolate_green_v(x - 1u, y - 1u) * 0.5;
        let g_ne = interpolate_green_h(x + 1u, y - 1u) * 0.5 + interpolate_green_v(x + 1u, y - 1u) * 0.5;
        let g_sw = interpolate_green_h(x - 1u, y + 1u) * 0.5 + interpolate_green_v(x - 1u, y + 1u) * 0.5;
        let g_se = interpolate_green_h(x + 1u, y + 1u) * 0.5 + interpolate_green_v(x + 1u, y + 1u) * 0.5;
        let diff_avg = ((r_nw - g_nw) + (r_ne - g_ne) + (r_sw - g_sw) + (r_se - g_se)) * 0.25;
        r = g + diff_avg;
    }
    
    return vec3<f32>(max(r, 0.0), max(g, 0.0), max(b, 0.0));
}

// Bilinear demosaicing (fallback for edges or performance)
fn demosaic_bilinear(x: u32, y: u32) -> vec3<f32> {
    let color = get_bayer_color(x, y);
    let raw = get_raw_value(x, y);
    
    var r: f32;
    var g: f32;
    var b: f32;

    if (color == 0u) {
        // Red pixel
        r = raw;
        g = (get_raw_value(x - 1u, y) + get_raw_value(x + 1u, y) + get_raw_value(x, y - 1u) + get_raw_value(x, y + 1u)) * 0.25;
        b = (get_raw_value(x - 1u, y - 1u) + get_raw_value(x - 1u, y + 1u) + get_raw_value(x + 1u, y - 1u) + get_raw_value(x + 1u, y + 1u)) * 0.25;
    } else if (color == 1u) {
        // Green pixel (R row)
        r = (get_raw_value(x - 1u, y) + get_raw_value(x + 1u, y)) * 0.5;
        g = raw;
        b = (get_raw_value(x, y - 1u) + get_raw_value(x, y + 1u)) * 0.5;
    } else if (color == 2u) {
        // Green pixel (B row)
        r = (get_raw_value(x, y - 1u) + get_raw_value(x, y + 1u)) * 0.5;
        g = raw;
        b = (get_raw_value(x - 1u, y) + get_raw_value(x + 1u, y)) * 0.5;
    } else {
        // Blue pixel
        r = (get_raw_value(x - 1u, y - 1u) + get_raw_value(x - 1u, y + 1u) + get_raw_value(x + 1u, y - 1u) + get_raw_value(x + 1u, y + 1u)) * 0.25;
        g = (get_raw_value(x - 1u, y) + get_raw_value(x + 1u, y) + get_raw_value(x, y - 1u) + get_raw_value(x, y + 1u)) * 0.25;
        b = raw;
    }

    return vec3<f32>(r, g, b);
}

// Apply color correction and gamma
fn apply_color_correction(rgb: vec3<f32>) -> vec3<f32> {
    // Normalize to 0-1 range
    var color = (rgb - params.black_level.xyz) / (params.white_level.xyz - params.black_level.xyz);
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // Apply color matrix
    color = params.color_matrix * color;

    // Apply gamma correction
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / params.gamma));

    return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= params.width || y >= params.height) {
        return;
    }

    // Use AHD demosaicing for interior pixels, bilinear for edges (where AHD needs more context)
    var raw_rgb: vec3<f32>;
    if (x >= 3u && x < params.width - 3u && y >= 3u && y < params.height - 3u) {
        raw_rgb = demosaic_ahd(x, y);
    } else {
        raw_rgb = demosaic_bilinear(x, y);
    }

    // Apply color correction
    let corrected_rgb = apply_color_correction(raw_rgb);

    // Convert to 8-bit RGBA
    let r = u32(clamp(corrected_rgb.r * 255.0, 0.0, 255.0));
    let g = u32(clamp(corrected_rgb.g * 255.0, 0.0, 255.0));
    let b = u32(clamp(corrected_rgb.b * 255.0, 0.0, 255.0));
    let a = 255u;

    let pixel = (a << 24) | (b << 16) | (g << 8) | r;
    let idx = y * params.width + x;
    output_pixels[idx] = pixel;
}