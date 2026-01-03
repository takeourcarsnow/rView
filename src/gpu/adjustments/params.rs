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
            brightness: adj.brightness,
            contrast: adj.contrast,
            saturation: adj.saturation,
            highlights: adj.highlights,
            shadows: adj.shadows,
            temperature: adj.temperature,
            tint: adj.tint,
            blacks: adj.blacks,
            whites: adj.whites,
            sharpening: adj.sharpening,
            width,
            height,
            film_enabled: if film.enabled { 1 } else { 0 },
            film_is_bw: if film.is_bw { 1 } else { 0 },
            tone_curve_shadows: film.tone_curve_shadows,
            tone_curve_midtones: film.tone_curve_midtones,
            tone_curve_highlights: film.tone_curve_highlights,
            s_curve_strength: film.s_curve_strength,
            grain_amount: film.grain_amount,
            grain_size: film.grain_size,
            grain_roughness: film.grain_roughness,
            halation_amount: film.halation_amount,
            vignette_amount: film.vignette_amount,
            vignette_softness: film.vignette_softness,
            latitude: film.latitude,
            red_gamma: film.red_gamma,
            green_gamma: film.green_gamma,
            blue_gamma: film.blue_gamma,
            black_point: film.black_point,
            white_point: film.white_point,
        }
    }
}
