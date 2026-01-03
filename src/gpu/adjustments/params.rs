use crate::gpu::types::AdjustmentParams;
use crate::image_loader::ImageAdjustments;

impl crate::gpu::types::GpuProcessor {
    pub fn create_adjustment_params(
        adj: &ImageAdjustments,
        width: u32,
        height: u32,
    ) -> AdjustmentParams {
        let film = &adj.film;
        AdjustmentParams {
            exposure: adj.exposure,
            saturation: adj.saturation,
            temperature: adj.temperature,
            width,
            height,
            film_enabled: if film.enabled { 1 } else { 0 },
            film_is_bw: if film.is_bw { 1 } else { 0 },
            tone_curve_shadows: film.tone.shadows,
            tone_curve_midtones: film.tone.midtones,
            tone_curve_highlights: film.tone.highlights,
            s_curve_strength: film.tone.s_curve_strength,
            grain_amount: film.grain.amount,
            grain_size: film.grain.size,
            grain_roughness: film.grain.roughness,
            halation_amount: film.halation.amount,
            vignette_amount: film.vignette.amount,
            vignette_softness: film.vignette.softness,
            latitude: film.latitude,
            red_gamma: film.color_gamma.red,
            green_gamma: film.color_gamma.green,
            blue_gamma: film.color_gamma.blue,
            black_point: film.black_point,
            white_point: film.white_point,
            // Color crossover matrix
            red_in_green: film.color_crossover.red_in_green,
            red_in_blue: film.color_crossover.red_in_blue,
            green_in_red: film.color_crossover.green_in_red,
            green_in_blue: film.color_crossover.green_in_blue,
            blue_in_red: film.color_crossover.blue_in_red,
            blue_in_green: film.color_crossover.blue_in_green,
            // Shadow/highlight tints
            shadow_tint_r: film.shadow_tint[0],
            shadow_tint_g: film.shadow_tint[1],
            shadow_tint_b: film.shadow_tint[2],
            highlight_tint_r: film.highlight_tint[0],
            highlight_tint_g: film.highlight_tint[1],
            highlight_tint_b: film.highlight_tint[2],
            // Halation color
            halation_color_r: film.halation.color[0],
            halation_color_g: film.halation.color[1],
            halation_color_b: film.halation.color[2],
            halation_radius: film.halation.radius,
        }
    }
}
