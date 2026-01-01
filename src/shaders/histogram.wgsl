// GPU-accelerated histogram computation
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> histogram: array<atomic<u32>>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let px = global_id.x;
    let py = global_id.y;

    if (px >= dims.x || py >= dims.y) {
        return;
    }

    let color = textureLoad(input_texture, vec2<i32>(i32(px), i32(py)), 0);

    // Convert to 0-255 range and atomically increment histogram bins
    let r_bin = u32(clamp(color.r * 255.0, 0.0, 255.0));
    let g_bin = u32(clamp(color.g * 255.0, 0.0, 255.0));
    let b_bin = u32(clamp(color.b * 255.0, 0.0, 255.0));
    let a_bin = u32(clamp(color.a * 255.0, 0.0, 255.0));

    // Luminance for RGB histogram
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let l_bin = u32(clamp(luminance * 255.0, 0.0, 255.0));

    // Atomic increments (4 channels × 256 bins each = 1024 total)
    atomicAdd(&histogram[r_bin], 1u);
    atomicAdd(&histogram[256u + g_bin], 1u);
    atomicAdd(&histogram[512u + b_bin], 1u);
    atomicAdd(&histogram[768u + l_bin], 1u);
}