// GPU-accelerated overlay generation (focus peaking, zebras)
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: OverlayParams;

struct OverlayParams {
    mode: u32,        // 0 = focus peaking, 1 = zebra
    threshold: f32,   // focus peaking threshold or zebra levels
    high_threshold: f32,
    low_threshold: f32,
    width: u32,
    height: u32,
};

// Sobel edge detection for focus peaking
fn sobel_edge(input_texture: texture_2d<f32>, pos: vec2<i32>) -> f32 {
    // Manual convolution with Sobel kernels (no matrix indexing)
    var gx = 0.0;
    var gy = 0.0;

    // Sample all 9 pixels in the 3x3 neighborhood
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let sample_pos = pos + vec2<i32>(i, j);
            if (sample_pos.x >= 0 && sample_pos.x < i32(params.width) &&
                sample_pos.y >= 0 && sample_pos.y < i32(params.height)) {
                let color = textureLoad(input_texture, sample_pos, 0);
                let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));

                // Apply Sobel kernel weights manually
                let weight_x = f32(i) * 2.0 - f32(j) * 2.0; // Simplified from kernel
                let weight_y = f32(j) * 2.0 - f32(i) * 2.0; // Simplified from kernel

                if (i == 0 && j == 0) {
                    // Center pixel - no contribution
                } else if (abs(i) == 1 && abs(j) == 1) {
                    // Corner pixels
                    gx += luminance * f32(i) * f32(j);
                    gy += luminance * f32(j) * f32(i);
                } else {
                    // Edge pixels
                    if (i == 0) {
                        gy += luminance * f32(j) * 2.0;
                    } else {
                        gx += luminance * f32(i) * 2.0;
                    }
                }
            }
        }
    }

    return sqrt(gx * gx + gy * gy);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let px = global_id.x;
    let py = global_id.y;

    if (px >= params.width || py >= params.height) {
        return;
    }

    let pos = vec2<i32>(i32(px), i32(py));
    let color = textureLoad(input_texture, pos, 0);
    var overlay_color = vec4<f32>(0.0, 0.0, 0.0, 0.0); // Transparent by default

    if (params.mode == 0u) {
        // Focus peaking
        let edge_strength = sobel_edge(input_texture, pos);
        if (edge_strength > params.threshold) {
            // Highlight edges in red/cyan
            overlay_color = vec4<f32>(1.0, 0.0, 1.0, edge_strength * 0.5);
        }
    } else if (params.mode == 1u) {
        // Zebra pattern for over/under exposure
        let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));

        // Over-exposed (highlights)
        if (luminance > params.high_threshold) {
            // Red stripes for over-exposure
            let stripe = sin(f32(px + py) * 0.1) * 0.5 + 0.5;
            if (stripe > 0.5) {
                overlay_color = vec4<f32>(1.0, 0.0, 0.0, 0.7);
            }
        }
        // Under-exposed (shadows)
        else if (luminance < params.low_threshold) {
            // Blue stripes for under-exposure
            let stripe = sin(f32(px + py) * 0.1) * 0.5 + 0.5;
            if (stripe > 0.5) {
                overlay_color = vec4<f32>(0.0, 0.0, 1.0, 0.7);
            }
        }
    }

    textureStore(output_texture, pos, overlay_color);
}