use crate::errors::{Result, ViewerError};
use image::DynamicImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmEmulation {
    pub enabled: bool,
    pub is_bw: bool,                    // Whether this is a B&W film (converts color to mono)

    // Tone curve control points (shadows, midtones, highlights) - values 0.0 to 1.0
    pub tone_curve_shadows: f32,        // Lift/lower shadows (-1.0 to 1.0)
    pub tone_curve_midtones: f32,       // Adjust midtones (-1.0 to 1.0)
    pub tone_curve_highlights: f32,     // Compress/expand highlights (-1.0 to 1.0)

    // S-curve strength for contrast (film characteristic curve)
    pub s_curve_strength: f32,          // 0.0 to 1.0

    // Film grain simulation
    pub grain_amount: f32,              // 0.0 to 1.0 (intensity)
    pub grain_size: f32,                // 0.5 to 2.0 (1.0 = normal)
    pub grain_roughness: f32,           // 0.0 to 1.0 (organic variation)

    // Halation (light bloom around bright areas, characteristic of film)
    pub halation_amount: f32,           // 0.0 to 1.0
    pub halation_radius: f32,           // Spread of the halation effect
    pub halation_color: [f32; 3],       // RGB tint for halation (usually warm/red)

    // Color channel crossover/crosstalk (film layers interact)
    pub red_in_green: f32,              // -0.2 to 0.2
    pub red_in_blue: f32,               // -0.2 to 0.2
    pub green_in_red: f32,              // -0.2 to 0.2
    pub green_in_blue: f32,             // -0.2 to 0.2
    pub blue_in_red: f32,               // -0.2 to 0.2
    pub blue_in_green: f32,             // -0.2 to 0.2

    // Color response curves (per-channel gamma/lift)
    pub red_gamma: f32,                 // 0.8 to 1.2
    pub green_gamma: f32,               // 0.8 to 1.2
    pub blue_gamma: f32,                // 0.8 to 1.2

    // Black point and white point (film base density and max density)
    pub black_point: f32,               // 0.0 to 0.1 (raised blacks = faded look)
    pub white_point: f32,               // 0.9 to 1.0 (compressed highlights)

    // Color cast/tint in shadows and highlights
    pub shadow_tint: [f32; 3],          // RGB tint for shadows
    pub highlight_tint: [f32; 3],       // RGB tint for highlights

    // Vignette (natural lens falloff)
    pub vignette_amount: f32,           // 0.0 to 1.0
    pub vignette_softness: f32,         // 0.5 to 2.0

    // Film latitude (dynamic range compression)
    pub latitude: f32,                  // 0.0 to 1.0 (higher = more DR recovery)
}

impl Default for FilmEmulation {
    fn default() -> Self {
        Self {
            enabled: false,
            is_bw: false,
            tone_curve_shadows: 0.0,
            tone_curve_midtones: 0.0,
            tone_curve_highlights: 0.0,
            s_curve_strength: 0.0,
            grain_amount: 0.0,
            grain_size: 1.0,
            grain_roughness: 0.5,
            halation_amount: 0.0,
            halation_radius: 1.0,
            halation_color: [1.0, 0.3, 0.1], // Warm red/orange
            red_in_green: 0.0,
            red_in_blue: 0.0,
            green_in_red: 0.0,
            green_in_blue: 0.0,
            blue_in_red: 0.0,
            blue_in_green: 0.0,
            red_gamma: 1.0,
            green_gamma: 1.0,
            blue_gamma: 1.0,
            black_point: 0.0,
            white_point: 1.0,
            shadow_tint: [0.0, 0.0, 0.0],
            highlight_tint: [0.0, 0.0, 0.0],
            vignette_amount: 0.0,
            vignette_softness: 1.0,
            latitude: 0.0,
        }
    }
}

// Apply basic adjustments (non-destructive preview)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAdjustments {
    pub exposure: f32,      // -3.0 to +3.0 (stops)
    pub contrast: f32,      // 0.5 to 2.0 (multiplier)
    pub brightness: f32,    // -100 to +100
    pub saturation: f32,    // 0.0 to 2.0 (multiplier)
    pub highlights: f32,    // -1.0 to +1.0
    pub shadows: f32,       // -1.0 to +1.0
    pub temperature: f32,   // -1.0 to +1.0 (cool to warm)
    pub tint: f32,          // -1.0 to +1.0 (green to magenta)
    pub blacks: f32,        // -1.0 to +1.0
    pub whites: f32,        // -1.0 to +1.0
    pub sharpening: f32,    // 0.0 to 2.0
    pub film: FilmEmulation, // Film emulation parameters
    pub frame_enabled: bool,
    pub frame_color: [f32; 3], // RGB 0-1
    pub frame_thickness: f32, // pixels
}

impl Default for ImageAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            brightness: 0.0,
            saturation: 1.0,
            highlights: 0.0,
            shadows: 0.0,
            temperature: 0.0,
            tint: 0.0,
            blacks: 0.0,
            whites: 0.0,
            sharpening: 0.0,
            film: FilmEmulation::default(),
            frame_enabled: false,
            frame_color: [0.0, 0.0, 0.0], // black
            frame_thickness: 10.0,
        }
    }
}

impl ImageAdjustments {
    pub fn is_default(&self) -> bool {
        self.exposure == 0.0 &&
        self.contrast == 1.0 &&
        self.brightness == 0.0 &&
        self.saturation == 1.0 &&
        self.highlights == 0.0 &&
        self.shadows == 0.0 &&
        self.temperature == 0.0 &&
        self.tint == 0.0 &&
        self.blacks == 0.0 &&
        self.whites == 0.0 &&
        self.sharpening == 0.0 &&
        !self.film.enabled &&
        !self.frame_enabled
    }

    pub fn apply_preset(&mut self, preset: FilmPreset) {
        *self = match preset {
            FilmPreset::None => ImageAdjustments::default(),

            // B&W films
            FilmPreset::TMax400 | FilmPreset::TMax100 | FilmPreset::Hp5 | FilmPreset::TriX400 | FilmPreset::Delta3200 => {
                let mut adj = ImageAdjustments::default();
                adj.film.enabled = true;
                adj.film.is_bw = true;
                adj.contrast = 1.1;
                adj.highlights = -0.1;
                adj.shadows = 0.1;
                adj.blacks = 0.1;
                adj.whites = -0.1;
                adj.sharpening = 0.3;
                adj.film.tone_curve_shadows = 0.05;
                adj.film.tone_curve_highlights = -0.05;
                adj.film.s_curve_strength = 0.2;
                adj.film.grain_amount = 0.15;
                adj.film.grain_size = 1.2;
                adj.film.black_point = 0.05;
                adj.film.white_point = 0.95;
                adj.film.vignette_amount = 0.05;
                adj.film.latitude = 0.8;
                adj
            }

            // Color films
            _ => {
                let mut adj = ImageAdjustments::default();
                adj.film.enabled = true;
                adj.film.is_bw = false;
                adj.contrast = 1.05;
                adj.saturation = 0.95;
                adj.highlights = -0.15;
                adj.shadows = 0.15;
                adj.blacks = 0.08;
                adj.whites = -0.08;
                adj.sharpening = 0.2;
                adj.film.tone_curve_shadows = 0.1;
                adj.film.tone_curve_highlights = -0.08;
                adj.film.s_curve_strength = 0.15;
                adj.film.grain_amount = 0.1;
                adj.film.halation_amount = 0.05;
                adj.film.black_point = 0.02;
                adj.film.white_point = 0.98;
                adj.film.vignette_amount = 0.08;
                adj.film.latitude = 0.7;
                adj
            }
        };
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum FilmPreset {
    None,
    Portra400,
    Portra160,
    Portra800,
    TMax400,
    TMax100,
    Provia100,
    Astia100,
    Hp5,
    Velvia50,
    Velvia100,
    KodakGold200,
    Fuji400H,
    TriX400,
    Delta3200,
    Ektar100,
}

impl FilmPreset {
    pub fn name(&self) -> &'static str {
        match self {
            FilmPreset::None => "None",
            FilmPreset::Portra400 => "Portra 400",
            FilmPreset::Portra160 => "Portra 160",
            FilmPreset::Portra800 => "Portra 800",
            FilmPreset::TMax400 => "T-Max 400",
            FilmPreset::TMax100 => "T-Max 100",
            FilmPreset::Provia100 => "Provia 100",
            FilmPreset::Astia100 => "Astia 100",
            FilmPreset::Hp5 => "HP5 Plus",
            FilmPreset::Velvia50 => "Velvia 50",
            FilmPreset::Velvia100 => "Velvia 100",
            FilmPreset::KodakGold200 => "Kodak Gold 200",
            FilmPreset::Fuji400H => "Fuji 400H",
            FilmPreset::TriX400 => "Tri-X 400",
            FilmPreset::Delta3200 => "Delta 3200",
            FilmPreset::Ektar100 => "Ektar 100",
        }
    }

    pub fn all() -> &'static [FilmPreset] {
        &[
            FilmPreset::None,
            FilmPreset::Portra400,
            FilmPreset::Portra160,
            FilmPreset::Portra800,
            FilmPreset::TMax400,
            FilmPreset::TMax100,
            FilmPreset::Provia100,
            FilmPreset::Astia100,
            FilmPreset::Hp5,
            FilmPreset::Velvia50,
            FilmPreset::Velvia100,
            FilmPreset::KodakGold200,
            FilmPreset::Fuji400H,
            FilmPreset::TriX400,
            FilmPreset::Delta3200,
            FilmPreset::Ektar100,
        ]
    }
}