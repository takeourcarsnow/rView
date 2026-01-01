// RAW image demosaicing compute shader
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

// Bilinear demosaicing algorithm
fn demosaic_bilinear(x: u32, y: u32) -> vec3<f32> {
    let w = params.width;
    let h = params.height;

    // Get raw values (16-bit stored in u32)
    let raw = get_raw_value(x, y);

    // Determine Bayer pattern position
    let pattern_x = x % 2u;
    let pattern_y = y % 2u;
    let bayer_idx = pattern_y * 2u + pattern_x;

    // Adjust for different Bayer patterns
    let pos = (bayer_idx + params.bayer_pattern) % 4u;

    var r: f32;
    var g: f32;
    var b: f32;

    if (pos == 0u) {
        // Red pixel (RGGB top-left)
        r = raw;
        g = (get_raw_value(x-1u, y) + get_raw_value(x+1u, y) + get_raw_value(x, y-1u) + get_raw_value(x, y+1u)) * 0.25;
        b = (get_raw_value(x-1u, y-1u) + get_raw_value(x-1u, y+1u) + get_raw_value(x+1u, y-1u) + get_raw_value(x+1u, y+1u)) * 0.25;
    } else if (pos == 1u) {
        // Green pixel (RGGB top-right)
        r = (get_raw_value(x-1u, y) + get_raw_value(x+1u, y)) * 0.5;
        g = raw;
        b = (get_raw_value(x, y-1u) + get_raw_value(x, y+1u)) * 0.5;
    } else if (pos == 2u) {
        // Green pixel (RGGB bottom-left)
        r = (get_raw_value(x, y-1u) + get_raw_value(x, y+1u)) * 0.5;
        g = raw;
        b = (get_raw_value(x-1u, y) + get_raw_value(x+1u, y)) * 0.5;
    } else {
        // Blue pixel (RGGB bottom-right)
        r = (get_raw_value(x-1u, y-1u) + get_raw_value(x-1u, y+1u) + get_raw_value(x+1u, y-1u) + get_raw_value(x+1u, y+1u)) * 0.25;
        g = (get_raw_value(x-1u, y) + get_raw_value(x+1u, y) + get_raw_value(x, y-1u) + get_raw_value(x, y+1u)) * 0.25;
        b = raw;
    }

    return vec3<f32>(r, g, b);
}

// Helper function to get raw pixel value
fn get_raw_value(px: u32, py: u32) -> f32 {
    if (px >= params.width || py >= params.height) { return 0.0; }
    let idx = py * params.width + px;
    return f32(raw_data[idx] & 0xFFFFu);
}

// Apply color correction and gamma
fn apply_color_correction(rgb: vec3<f32>) -> vec3<f32> {
    // Normalize to 0-1 range
    var color = (rgb - params.black_level.xyz) / (params.white_level.xyz - params.black_level.xyz);
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // Apply color matrix
    color = params.color_matrix * color;

    // Apply gamma correction
    color = pow(color, vec3<f32>(1.0 / params.gamma));

    return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= params.width || y >= params.height) {
        return;
    }

    // Demosaic
    let raw_rgb = demosaic_bilinear(x, y);

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