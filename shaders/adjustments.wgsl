struct Params {
    exposure : f32,
    brightness : f32,
    contrast : f32,
    saturation : f32,
    highlights : f32,
    shadows : f32,
    temperature : f32,
    tint : f32,
    blacks : f32,
    whites : f32,
    sharpening : f32,
    width : u32,
    height : u32,
    // Film emulation parameters
    film_enabled : u32,
    film_is_bw : u32,
    tone_curve_shadows : f32,
    tone_curve_midtones : f32,
    tone_curve_highlights : f32,
    s_curve_strength : f32,
    grain_amount : f32,
    grain_size : f32,
    grain_roughness : f32,
    halation_amount : f32,
    halation_radius : f32,
    halation_color_r : f32,
    halation_color_g : f32,
    halation_color_b : f32,
    red_in_green : f32,
    red_in_blue : f32,
    green_in_red : f32,
    green_in_blue : f32,
    blue_in_red : f32,
    blue_in_green : f32,
    red_gamma : f32,
    green_gamma : f32,
    blue_gamma : f32,
    black_point : f32,
    white_point : f32,
    shadow_tint_r : f32,
    shadow_tint_g : f32,
    shadow_tint_b : f32,
    highlight_tint_r : f32,
    highlight_tint_g : f32,
    highlight_tint_b : f32,
    vignette_amount : f32,
    vignette_softness : f32,
    latitude : f32,
    _padding : f32,
};

@group(0) @binding(0) var<storage, read> input_pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_pixels: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<uniform> offset: u32;

fn unpack_u32(px: u32) -> vec4<f32> {
    let r = f32(px & 0xFFu) / 255.0;
    let g = f32((px >> 8) & 0xFFu) / 255.0;
    let b = f32((px >> 16) & 0xFFu) / 255.0;
    let a = f32((px >> 24) & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

fn pack_u32(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.x * 255.0, 0.0, 255.0));
    let g = u32(clamp(c.y * 255.0, 0.0, 255.0));
    let b = u32(clamp(c.z * 255.0, 0.0, 255.0));
    let a = u32(clamp(c.w * 255.0, 0.0, 255.0));
    return (a << 24) | (b << 16) | (g << 8) | r;
}

// Hash function for pseudo-random noise
fn hash(x: u32, y: u32, seed: u32) -> f32 {
    var h = seed;
    h = h ^ x;
    h = h * 0x517cc1b7u;
    h = h ^ y;
    h = h * 0x517cc1b7u;
    h = h ^ (h >> 16u);
    return f32(h) / f32(0xFFFFFFFFu) * 2.0 - 1.0;
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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let idx = GlobalInvocationID.x + offset;
    if (idx >= params.width * params.height) {
        return;
    }

    let px = idx % params.width;
    let py = idx / params.width;
    
    var c = unpack_u32(input_pixels[idx]); // rgba in [0,1]
    var rgb = c.xyz;
    
    let film_enabled = params.film_enabled != 0u;
    let film_is_bw = params.film_is_bw != 0u;

    // ============ FILM EMULATION ============
    if (film_enabled) {
        // B&W conversion for monochrome films
        if (film_is_bw) {
            let luminance = 0.30 * rgb.x + 0.59 * rgb.y + 0.11 * rgb.z;
            rgb = vec3<f32>(luminance);
        }
        
        // Color channel crossover/crosstalk
        if (!film_is_bw) {
            let orig = rgb;
            rgb.x = orig.x + orig.y * params.green_in_red + orig.z * params.blue_in_red;
            rgb.y = orig.y + orig.x * params.red_in_green + orig.z * params.blue_in_green;
            rgb.z = orig.z + orig.x * params.red_in_blue + orig.y * params.green_in_blue;
        }
        
        // Per-channel gamma
        rgb.x = pow(max(rgb.x, 0.0), params.red_gamma);
        rgb.y = pow(max(rgb.y, 0.0), params.green_gamma);
        rgb.z = pow(max(rgb.z, 0.0), params.blue_gamma);
        
        // Film latitude (dynamic range compression)
        if (params.latitude > 0.0) {
            let lat = params.latitude * 0.5;
            rgb = rgb / (vec3<f32>(1.0) + rgb * lat);
            let comp = 1.0 + lat * 0.5;
            rgb = rgb * comp;
        }
        
        // S-curve
        if (params.s_curve_strength > 0.0) {
            rgb.x = apply_s_curve(rgb.x, params.s_curve_strength);
            rgb.y = apply_s_curve(rgb.y, params.s_curve_strength);
            rgb.z = apply_s_curve(rgb.z, params.s_curve_strength);
        }
        
        // Tone curve
        rgb.x = apply_tone_curve(rgb.x, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        rgb.y = apply_tone_curve(rgb.y, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        rgb.z = apply_tone_curve(rgb.z, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        
        // Black/white point
        let bp = params.black_point;
        let wp = params.white_point;
        let range = wp - bp;
        if (range > 0.01) {
            rgb = vec3<f32>(bp) + rgb * range;
        }
        
        // Shadow/highlight tinting
        let luminance = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        let shadow_amount = clamp(1.0 - luminance * 2.0, 0.0, 1.0);
        let highlight_amount = clamp((luminance - 0.5) * 2.0, 0.0, 1.0);
        
        rgb.x = rgb.x + params.shadow_tint_r * shadow_amount + params.highlight_tint_r * highlight_amount;
        rgb.y = rgb.y + params.shadow_tint_g * shadow_amount + params.highlight_tint_g * highlight_amount;
        rgb.z = rgb.z + params.shadow_tint_b * shadow_amount + params.highlight_tint_b * highlight_amount;
    }

    // Convert to 0-255 range for standard adjustments
    rgb = rgb * 255.0;
    
    // ============ STANDARD ADJUSTMENTS ============
    
    // Apply exposure
    let exposure_mult = pow(2.0, params.exposure);
    rgb = rgb * exposure_mult;
    
    // Blacks adjustment (lift shadows)
    rgb = rgb + vec3<f32>(params.blacks * 25.5);
    
    // Whites adjustment (reduce highlights)
    rgb = rgb * (1.0 - params.whites * 0.1);
    
    // Shadows adjustment (gamma-like curve for shadows)
    if (params.shadows < 0.0) {
        let gamma = 1.0 - params.shadows;
        rgb = pow(max(rgb / 255.0, vec3<f32>(0.0)), vec3<f32>(gamma)) * 255.0;
    }
    
    // Highlights adjustment (compress highlights)
    if (params.highlights > 0.0) {
        let luminance = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        let highlight_mask = clamp((luminance - 127.5) / 127.5, 0.0, 1.0);
        let compress = 1.0 - params.highlights * highlight_mask;
        rgb = rgb * compress;
    }
    
    // Brightness
    rgb = rgb + vec3<f32>(params.brightness * 2.55);
    
    // Contrast
    rgb = ((rgb / 255.0 - vec3<f32>(0.5)) * params.contrast + vec3<f32>(0.5)) * 255.0;
    
    // Temperature
    rgb.x = rgb.x + params.temperature * 25.5;
    rgb.z = rgb.z - params.temperature * 15.3;
    
    // Tint
    rgb.x = rgb.x + params.tint * 12.75;
    rgb.y = rgb.y - params.tint * 20.4;
    rgb.z = rgb.z + params.tint * 12.75;
    
    // Saturation (skip for B&W film)
    if (!film_enabled || !film_is_bw) {
        let gray = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        rgb = vec3<f32>(gray) + (rgb - vec3<f32>(gray)) * params.saturation;
    }
    
    // Basic sharpening
    if (params.sharpening > 0.0) {
        let gray = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        let sharpened = rgb + (rgb - vec3<f32>(gray)) * params.sharpening;
        rgb = rgb + (sharpened - rgb) * params.sharpening;
    }
    
    // Convert back to 0-1 range
    rgb = rgb / 255.0;
    
    // ============ FILM POST-PROCESSING ============
    if (film_enabled) {
        // Vignette
        if (params.vignette_amount > 0.0) {
            let center = vec2<f32>(f32(params.width) / 2.0, f32(params.height) / 2.0);
            let max_dist = length(center);
            let pos = vec2<f32>(f32(px), f32(py));
            let dist = length(pos - center) / max_dist;
            let vignette = 1.0 - params.vignette_amount * pow(dist / params.vignette_softness, 2.0);
            rgb = rgb * clamp(vignette, 0.0, 1.0);
        }
        
        // Film grain
        if (params.grain_amount > 0.0) {
            let scale = 1.0 / params.grain_size;
            let sx = u32(f32(px) * scale);
            let sy = u32(f32(py) * scale);
            var grain = hash(sx, sy, 12345u);
            if (params.grain_roughness > 0.0) {
                let grain2 = hash(sx + 1u, sy + 1u, 54321u);
                grain = grain * (1.0 - params.grain_roughness * 0.5) + grain2 * params.grain_roughness * 0.5;
            }
            
            let lum = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
            let grain_mask = 4.0 * lum * (1.0 - lum);
            let grain_strength = params.grain_amount * 255.0 * 0.15 * grain_mask;
            
            rgb.x = rgb.x + grain * grain_strength / 255.0;
            rgb.y = rgb.y + grain * grain_strength / 255.0;
            rgb.z = rgb.z + grain * grain_strength / 255.0;
        }
        
        // Halation
        if (params.halation_amount > 0.0) {
            let luminance = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
            let halation_mask = clamp((luminance - 0.7) / 0.3, 0.0, 1.0);
            let halation_strength = params.halation_amount * halation_mask * 30.0 / 255.0;
            rgb.x = rgb.x + params.halation_color_r * halation_strength;
            rgb.y = rgb.y + params.halation_color_g * halation_strength;
            rgb.z = rgb.z + params.halation_color_b * halation_strength;
        }
    }
    
    // Clamp
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    c = vec4<f32>(rgb, c.w);

    output_pixels[idx] = pack_u32(vec4<f32>(c.xyz, c.w));
}