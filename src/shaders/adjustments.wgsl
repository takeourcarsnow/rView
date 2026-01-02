// Image adjustments compute shader with film emulation
// Enhanced with ACES tone mapping and OKLab color space
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: AdjustmentParams;

struct AdjustmentParams {
    exposure: f32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    highlights: f32,
    shadows: f32,
    temperature: f32,
    tint: f32,
    blacks: f32,
    whites: f32,
    sharpening: f32,
    width: u32,
    height: u32,
    film_enabled: u32,
    film_is_bw: u32,
    tone_curve_shadows: f32,
    tone_curve_midtones: f32,
    tone_curve_highlights: f32,
    s_curve_strength: f32,
    grain_amount: f32,
    grain_size: f32,
    grain_roughness: f32,
    halation_amount: f32,
    vignette_amount: f32,
    vignette_softness: f32,
    latitude: f32,
    red_gamma: f32,
    green_gamma: f32,
    blue_gamma: f32,
    black_point: f32,
    white_point: f32,
};

// Hash function for pseudo-random noise
fn hash(p: vec2<u32>, seed: u32) -> f32 {
    var h = seed;
    h = h ^ p.x;
    h = h * 0x517cc1b7u;
    h = h ^ p.y;
    h = h * 0x517cc1b7u;
    h = h ^ (h >> 16u);
    return f32(h) / f32(0xFFFFFFFFu) * 2.0 - 1.0;
}

// ============ ACES FILMIC TONE MAPPING ============
// Based on Stephen Hill's fit of ACES RRT+ODT
fn aces_tonemap_single(x: f32) -> f32 {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    let xp = max(x, 0.0);
    return clamp((xp * (a * xp + b)) / (xp * (c * xp + d) + e), 0.0, 1.0);
}

fn apply_aces_tonemap(rgb: vec3<f32>, strength: f32) -> vec3<f32> {
    if (strength <= 0.0) {
        return rgb;
    }
    let mapped = vec3<f32>(
        aces_tonemap_single(rgb.x),
        aces_tonemap_single(rgb.y),
        aces_tonemap_single(rgb.z)
    );
    return mix(rgb, mapped, strength);
}

// ============ OKLAB COLOR SPACE ============
// Perceptually uniform color space for better saturation/vibrance

fn srgb_to_linear(x: f32) -> f32 {
    if (x <= 0.04045) {
        return x / 12.92;
    } else {
        return pow((x + 0.055) / 1.055, 2.4);
    }
}

fn linear_to_srgb(x: f32) -> f32 {
    if (x <= 0.0031308) {
        return x * 12.92;
    } else {
        return 1.055 * pow(x, 1.0 / 2.4) - 0.055;
    }
}

fn linear_srgb_to_oklab(rgb: vec3<f32>) -> vec3<f32> {
    let l = 0.4122214708 * rgb.x + 0.5363325363 * rgb.y + 0.0514459929 * rgb.z;
    let m = 0.2119034982 * rgb.x + 0.6806995451 * rgb.y + 0.1073969566 * rgb.z;
    let s = 0.0883024619 * rgb.x + 0.2817188376 * rgb.y + 0.6299787005 * rgb.z;
    
    let l_ = pow(max(l, 0.0), 1.0 / 3.0);
    let m_ = pow(max(m, 0.0), 1.0 / 3.0);
    let s_ = pow(max(s, 0.0), 1.0 / 3.0);
    
    return vec3<f32>(
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_
    );
}

fn oklab_to_linear_srgb(lab: vec3<f32>) -> vec3<f32> {
    let l_ = lab.x + 0.3963377774 * lab.y + 0.2158037573 * lab.z;
    let m_ = lab.x - 0.1055613458 * lab.y - 0.0638541728 * lab.z;
    let s_ = lab.x - 0.0894841775 * lab.y - 1.2914855480 * lab.z;
    
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    
    return vec3<f32>(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    );
}

fn apply_oklab_saturation(rgb: vec3<f32>, saturation: f32) -> vec3<f32> {
    // Convert to linear sRGB
    let r_lin = srgb_to_linear(rgb.x);
    let g_lin = srgb_to_linear(rgb.y);
    let b_lin = srgb_to_linear(rgb.z);
    
    // Convert to OKLab
    let lab = linear_srgb_to_oklab(vec3<f32>(r_lin, g_lin, b_lin));
    
    // Scale chroma (a and b channels) by saturation factor
    let lab_adjusted = vec3<f32>(lab.x, lab.y * saturation, lab.z * saturation);
    
    // Convert back to linear sRGB
    let rgb_out = oklab_to_linear_srgb(lab_adjusted);
    
    // Convert back to sRGB gamma
    return vec3<f32>(
        linear_to_srgb(clamp(rgb_out.x, 0.0, 1.0)),
        linear_to_srgb(clamp(rgb_out.y, 0.0, 1.0)),
        linear_to_srgb(clamp(rgb_out.z, 0.0, 1.0))
    );
}

// S-curve for film characteristic curve
fn apply_s_curve(x: f32, strength: f32) -> f32 {
    let xc = clamp(x, 0.0, 1.0);
    let midpoint = 0.5;
    let steepness = 1.0 + strength * 3.0;

    let sigmoid = 1.0 / (1.0 + exp(-steepness * (xc - midpoint)));
    let min_sig = 1.0 / (1.0 + exp(steepness * midpoint));
    let max_sig = 1.0 / (1.0 + exp(-steepness * (1.0 - midpoint)));

    let normalized = (sigmoid - min_sig) / (max_sig - min_sig);
    return xc * (1.0 - strength) + normalized * strength;
}

// Tone curve for shadows/midtones/highlights
fn apply_tone_curve(x: f32, shadows: f32, midtones: f32, highlights: f32) -> f32 {
    let xc = clamp(x, 0.0, 1.0);

    let shadow_weight = clamp(1.0 - xc * 3.0, 0.0, 1.0);
    let highlight_weight = clamp((xc - 0.66) * 3.0, 0.0, 1.0);
    let midtone_weight = 1.0 - shadow_weight - highlight_weight;

    let adjustment = shadows * shadow_weight * 0.15
                   + midtones * midtone_weight * 0.1
                   + highlights * highlight_weight * 0.15;

    return clamp(xc + adjustment, 0.0, 1.0);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let px = global_id.x;
    let py = global_id.y;

    if (px >= params.width || py >= params.height) {
        return;
    }

    var color = textureLoad(input_texture, vec2<i32>(i32(px), i32(py)), 0).rgb;
    let alpha = textureLoad(input_texture, vec2<i32>(i32(px), i32(py)), 0).a;

    // ============ FILM EMULATION ============
    if (params.film_enabled != 0u) {
        // B&W conversion for monochrome films
        if (params.film_is_bw != 0u) {
            let luminance = 0.30 * color.r + 0.59 * color.g + 0.11 * color.b;
            color = vec3<f32>(luminance);
        }

        // Per-channel gamma
        color.r = pow(max(color.r, 0.0001), params.red_gamma);
        color.g = pow(max(color.g, 0.0001), params.green_gamma);
        color.b = pow(max(color.b, 0.0001), params.blue_gamma);

        // Film latitude (dynamic range compression)
        if (params.latitude > 0.0) {
            let lat = params.latitude * 0.5;
            color = color / (vec3<f32>(1.0) + color * lat);
            let comp = 1.0 + lat * 0.5;
            color = color * comp;
        }

        // S-curve
        if (params.s_curve_strength > 0.0) {
            color.r = apply_s_curve(color.r, params.s_curve_strength);
            color.g = apply_s_curve(color.g, params.s_curve_strength);
            color.b = apply_s_curve(color.b, params.s_curve_strength);
        }

        // Tone curve
        color.r = apply_tone_curve(color.r, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        color.g = apply_tone_curve(color.g, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        color.b = apply_tone_curve(color.b, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);

        // Black/white point
        let bp = params.black_point;
        let wp = params.white_point;
        let range = wp - bp;
        if (range > 0.01) {
            color = vec3<f32>(bp) + color * range;
        }
    }

    // ============ STANDARD ADJUSTMENTS ============

    // Exposure (stops): multiply
    let exposure_mult = pow(2.0, params.exposure);
    color = color * exposure_mult;

    // Blacks adjustment (lift shadows)
    color = color + vec3<f32>(params.blacks * 0.1);

    // Whites adjustment (reduce highlights)
    let white_factor = 1.0 - params.whites * 0.1;
    color = color * white_factor;

    // Shadows adjustment (gamma-like curve for shadows)
    let shadow_lift = params.shadows * 0.2;
    color = mix(color, pow(color, vec3<f32>(1.0 - shadow_lift)), step(0.0, -shadow_lift));

    // Highlights adjustment (compress highlights)
    let highlight_compress = params.highlights * 0.3;
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    let highlight_mask = smoothstep(0.5, 1.0, luminance);
    color = mix(color, color * (1.0 - highlight_compress * highlight_mask), step(0.0, highlight_compress));

    // Brightness
    color = color + vec3<f32>(params.brightness / 100.0);

    // Contrast
    color = ((color - vec3<f32>(0.5)) * params.contrast + vec3<f32>(0.5));

    // Temperature adjustment
    if (params.temperature > 0.0) {
        color.r = color.r + params.temperature * 0.1;
        color.g = color.g + params.temperature * 0.05;
        color.b = color.b - params.temperature * 0.08;
    } else {
        color.r = color.r + params.temperature * 0.08;
        color.g = color.g + params.temperature * 0.05;
        color.b = color.b - params.temperature * 0.1;
    }

    // Tint adjustment
    if (params.tint > 0.0) {
        color.r = color.r + params.tint * 0.05;
        color.b = color.b + params.tint * 0.05;
    } else {
        color.g = color.g - params.tint * 0.08;
    }

    // ============ ACES FILMIC TONE MAPPING ============
    // Apply ACES for better highlight handling and cinematic look
    {
        // ACES strength increases with exposure to handle highlight clipping
        let exposure_mult = pow(2.0, params.exposure);
        let aces_strength = clamp(0.5 + abs(exposure_mult - 1.0) * 0.3, 0.3, 0.9);
        color = apply_aces_tonemap(max(color, vec3<f32>(0.0)), aces_strength);
    }

    // ============ OKLAB SATURATION (perceptually uniform) ============
    // Saturation (skip for B&W film) - now using OKLab for better perceptual uniformity
    if (params.film_enabled == 0u || params.film_is_bw == 0u) {
        if (abs(params.saturation - 1.0) > 0.001) {
            // Apply OKLab saturation for perceptually uniform results
            color = apply_oklab_saturation(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), params.saturation);
        }
    }

    // Sharpening using local contrast enhancement
    // Since we can't easily sample neighbors, we enhance mid-tone contrast for perceived sharpness
    if (params.sharpening > 0.0) {
        // Calculate luminance
        let lum = dot(color, vec3<f32>(0.299, 0.587, 0.114));
        
        // Local contrast enhancement - boost difference from mid-gray
        let mid = 0.5;
        let contrast_boost = 1.0 + params.sharpening * 0.5;
        
        // Apply contrast enhancement per channel
        let enhanced = (color - mid) * contrast_boost + mid;
        
        // Blend based on sharpening amount, with edge emphasis
        let detail_factor = abs(lum - 0.5) * 2.0;
        let blend = params.sharpening * (0.3 + 0.7 * (1.0 - detail_factor));
        
        color = mix(color, enhanced, blend);
    }

    // ============ FILM POST-PROCESSING ============
    if (params.film_enabled != 0u) {
        // Vignette
        if (params.vignette_amount > 0.0) {
            let center = vec2<f32>(f32(params.width) / 2.0, f32(params.height) / 2.0);
            let max_dist = length(center);
            let pos = vec2<f32>(f32(px), f32(py));
            let dist = length(pos - center) / max_dist;
            let vignette = 1.0 - params.vignette_amount * pow(dist / params.vignette_softness, 2.0);
            color = color * clamp(vignette, 0.0, 1.0);
        }

        // Film grain
        if (params.grain_amount > 0.0) {
            let scale = 1.0 / params.grain_size;
            let sx = u32(f32(px) * scale);
            let sy = u32(f32(py) * scale);

            var grain = hash(vec2<u32>(sx, sy), 12345u);
            if (params.grain_roughness > 0.0) {
                let grain2 = hash(vec2<u32>(sx + 1u, sy + 1u), 54321u);
                grain = grain * (1.0 - params.grain_roughness * 0.5) + grain2 * params.grain_roughness * 0.5;
            }

            let lum = dot(color, vec3<f32>(0.299, 0.587, 0.114));
            let grain_mask = 4.0 * lum * (1.0 - lum);
            let grain_strength = params.grain_amount * 0.15 * grain_mask;

            color = color + vec3<f32>(grain * grain_strength);
        }

        // Halation
        if (params.halation_amount > 0.0) {
            let lum = dot(color, vec3<f32>(0.299, 0.587, 0.114));
            let halation_mask = clamp((lum - 0.7) / 0.3, 0.0, 1.0);
            let halation_strength = params.halation_amount * halation_mask * 0.12;
            color.r = color.r + halation_strength;
            color.g = color.g + halation_strength;
            color.b = color.b + halation_strength;
        }
    }

    // Clamp final result
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    textureStore(output_texture, vec2<i32>(i32(px), i32(py)), vec4<f32>(color, alpha));
}