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

/// Film stock characteristics used to build presets
struct FilmCharacteristics {
    // Base adjustments
    contrast: f32,
    saturation: f32,
    highlights: f32,
    shadows: f32,
    blacks: f32,
    whites: f32,
    sharpening: f32,
    temperature: f32,
    tint: f32,
    
    // Film-specific parameters
    is_bw: bool,
    tone_curve_shadows: f32,
    tone_curve_midtones: f32,
    tone_curve_highlights: f32,
    s_curve_strength: f32,
    grain_amount: f32,
    grain_size: f32,
    grain_roughness: f32,
    halation_amount: f32,
    halation_radius: f32,
    halation_color: [f32; 3],
    black_point: f32,
    white_point: f32,
    shadow_tint: [f32; 3],
    highlight_tint: [f32; 3],
    vignette_amount: f32,
    vignette_softness: f32,
    latitude: f32,
    // Color crossover matrix
    red_in_green: f32,
    red_in_blue: f32,
    green_in_red: f32,
    green_in_blue: f32,
    blue_in_red: f32,
    blue_in_green: f32,
    // Per-channel gamma
    red_gamma: f32,
    green_gamma: f32,
    blue_gamma: f32,
}

impl Default for FilmCharacteristics {
    fn default() -> Self {
        Self {
            contrast: 1.0,
            saturation: 1.0,
            highlights: 0.0,
            shadows: 0.0,
            blacks: 0.0,
            whites: 0.0,
            sharpening: 0.0,
            temperature: 0.0,
            tint: 0.0,
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
            halation_color: [1.0, 0.3, 0.1],
            black_point: 0.0,
            white_point: 1.0,
            shadow_tint: [0.0, 0.0, 0.0],
            highlight_tint: [0.0, 0.0, 0.0],
            vignette_amount: 0.0,
            vignette_softness: 1.0,
            latitude: 0.0,
            red_in_green: 0.0,
            red_in_blue: 0.0,
            green_in_red: 0.0,
            green_in_blue: 0.0,
            blue_in_red: 0.0,
            blue_in_green: 0.0,
            red_gamma: 1.0,
            green_gamma: 1.0,
            blue_gamma: 1.0,
        }
    }
}

impl FilmCharacteristics {
    fn to_adjustments(&self) -> ImageAdjustments {
        ImageAdjustments {
            exposure: 0.0,
            contrast: self.contrast,
            brightness: 0.0,
            saturation: self.saturation,
            highlights: self.highlights,
            shadows: self.shadows,
            temperature: self.temperature,
            tint: self.tint,
            blacks: self.blacks,
            whites: self.whites,
            sharpening: self.sharpening,
            film: FilmEmulation {
                enabled: true,
                is_bw: self.is_bw,
                tone_curve_shadows: self.tone_curve_shadows,
                tone_curve_midtones: self.tone_curve_midtones,
                tone_curve_highlights: self.tone_curve_highlights,
                s_curve_strength: self.s_curve_strength,
                grain_amount: self.grain_amount,
                grain_size: self.grain_size,
                grain_roughness: self.grain_roughness,
                halation_amount: self.halation_amount,
                halation_radius: self.halation_radius,
                halation_color: self.halation_color,
                red_in_green: self.red_in_green,
                red_in_blue: self.red_in_blue,
                green_in_red: self.green_in_red,
                green_in_blue: self.green_in_blue,
                blue_in_red: self.blue_in_red,
                blue_in_green: self.blue_in_green,
                red_gamma: self.red_gamma,
                green_gamma: self.green_gamma,
                blue_gamma: self.blue_gamma,
                black_point: self.black_point,
                white_point: self.white_point,
                shadow_tint: self.shadow_tint,
                highlight_tint: self.highlight_tint,
                vignette_amount: self.vignette_amount,
                vignette_softness: self.vignette_softness,
                latitude: self.latitude,
            },
            frame_enabled: false,
            frame_color: [0.0, 0.0, 0.0],
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
        if preset == FilmPreset::None {
            self.film.enabled = false;
        } else {
            let frame_enabled = self.frame_enabled;
            let frame_color = self.frame_color;
            let frame_thickness = self.frame_thickness;
            *self = preset.characteristics().to_adjustments();
            self.frame_enabled = frame_enabled;
            self.frame_color = frame_color;
            self.frame_thickness = frame_thickness;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum FilmPreset {
    None,
    // Kodak Color Negative
    Portra160,
    Portra400,
    Portra800,
    Ektar100,
    KodakGold200,
    // Fujifilm Color Negative
    Fuji400H,
    FujiSuperia400,
    // Fujifilm Slide (Reversal)
    Provia100,
    Velvia50,
    Velvia100,
    Astia100,
    // Kodak B&W
    TMax100,
    TMax400,
    TriX400,
    // Ilford B&W
    Hp5,
    Delta100,
    Delta3200,
    PanF50,
    // Cinematic
    CineStill800T,
    CineStill50D,
}

impl FilmPreset {
    pub fn name(&self) -> &'static str {
        match self {
            FilmPreset::None => "None",
            FilmPreset::Portra160 => "Portra 160",
            FilmPreset::Portra400 => "Portra 400",
            FilmPreset::Portra800 => "Portra 800",
            FilmPreset::Ektar100 => "Ektar 100",
            FilmPreset::KodakGold200 => "Kodak Gold 200",
            FilmPreset::Fuji400H => "Fuji 400H",
            FilmPreset::FujiSuperia400 => "Fuji Superia 400",
            FilmPreset::Provia100 => "Provia 100F",
            FilmPreset::Velvia50 => "Velvia 50",
            FilmPreset::Velvia100 => "Velvia 100",
            FilmPreset::Astia100 => "Astia 100F",
            FilmPreset::TMax100 => "T-Max 100",
            FilmPreset::TMax400 => "T-Max 400",
            FilmPreset::TriX400 => "Tri-X 400",
            FilmPreset::Hp5 => "HP5 Plus 400",
            FilmPreset::Delta100 => "Delta 100",
            FilmPreset::Delta3200 => "Delta 3200",
            FilmPreset::PanF50 => "Pan F Plus 50",
            FilmPreset::CineStill800T => "CineStill 800T",
            FilmPreset::CineStill50D => "CineStill 50D",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            FilmPreset::None => "No film emulation",
            FilmPreset::Portra160 => "Fine grain, natural skin tones, low contrast",
            FilmPreset::Portra400 => "Versatile portrait film, warm tones, excellent latitude",
            FilmPreset::Portra800 => "High-speed portrait film, slightly warmer, more grain",
            FilmPreset::Ektar100 => "Vivid colors, high saturation, fine grain, punchy contrast",
            FilmPreset::KodakGold200 => "Consumer film, warm cast, nostalgic look",
            FilmPreset::Fuji400H => "Soft pastels, slightly cool, great for overexposure",
            FilmPreset::FujiSuperia400 => "Consumer film, green-tinted shadows, punchy",
            FilmPreset::Provia100 => "Neutral slide film, accurate colors, moderate contrast",
            FilmPreset::Velvia50 => "Hyper-saturated, deep blacks, landscape film",
            FilmPreset::Velvia100 => "Saturated but slightly softer than Velvia 50",
            FilmPreset::Astia100 => "Soft contrast slide film, natural skin tones",
            FilmPreset::TMax100 => "Ultra-fine grain B&W, smooth tones, scientific clarity",
            FilmPreset::TMax400 => "Modern T-grain B&W, fine grain for speed, broad tones",
            FilmPreset::TriX400 => "Classic gritty B&W, strong grain, punchy contrast",
            FilmPreset::Hp5 => "Versatile B&W, classic grain, pushable",
            FilmPreset::Delta100 => "Modern fine-grain B&W, smooth gradation",
            FilmPreset::Delta3200 => "High-speed B&W, pronounced grain, moody",
            FilmPreset::PanF50 => "Extremely fine grain B&W, exceptional sharpness",
            FilmPreset::CineStill800T => "Tungsten-balanced cinema film, halation, teal/orange",
            FilmPreset::CineStill50D => "Daylight cinema film, rich colors, subtle halation",
        }
    }

    pub fn all() -> &'static [FilmPreset] {
        &[
            FilmPreset::None,
            // Kodak Color Negative
            FilmPreset::Portra160,
            FilmPreset::Portra400,
            FilmPreset::Portra800,
            FilmPreset::Ektar100,
            FilmPreset::KodakGold200,
            // Fujifilm Color Negative
            FilmPreset::Fuji400H,
            FilmPreset::FujiSuperia400,
            // Fujifilm Slide
            FilmPreset::Provia100,
            FilmPreset::Velvia50,
            FilmPreset::Velvia100,
            FilmPreset::Astia100,
            // Kodak B&W
            FilmPreset::TMax100,
            FilmPreset::TMax400,
            FilmPreset::TriX400,
            // Ilford B&W
            FilmPreset::Hp5,
            FilmPreset::Delta100,
            FilmPreset::Delta3200,
            FilmPreset::PanF50,
            // Cinematic
            FilmPreset::CineStill800T,
            FilmPreset::CineStill50D,
        ]
    }

    /// Returns authentic film characteristics for each stock
    fn characteristics(&self) -> FilmCharacteristics {
        match self {
            FilmPreset::None => FilmCharacteristics::default(),

            // ========== KODAK COLOR NEGATIVE ==========
            
            // Portra 160: Fine grain professional film, excellent skin tones
            // Known for: Natural colors, low contrast, superb latitude
            FilmPreset::Portra160 => FilmCharacteristics {
                contrast: 0.95,
                saturation: 0.92,
                highlights: -0.15,
                shadows: 0.12,
                blacks: 0.03,
                whites: -0.05,
                sharpening: 0.15,
                temperature: 0.05,  // Slightly warm
                tint: 0.02,
                tone_curve_shadows: 0.08,
                tone_curve_midtones: 0.0,
                tone_curve_highlights: -0.1,
                s_curve_strength: 0.1,
                grain_amount: 0.05,
                grain_size: 0.8,
                grain_roughness: 0.3,
                halation_amount: 0.02,
                halation_radius: 0.8,
                halation_color: [1.0, 0.6, 0.4],
                black_point: 0.015,
                white_point: 0.99,
                shadow_tint: [0.02, 0.01, -0.01],  // Warm shadows
                highlight_tint: [0.01, 0.0, -0.02],
                vignette_amount: 0.03,
                vignette_softness: 1.5,
                latitude: 0.9,  // Excellent latitude
                red_in_green: 0.02,
                green_in_red: 0.01,
                red_gamma: 1.02,
                green_gamma: 1.0,
                blue_gamma: 0.98,
                ..Default::default()
            },

            // Portra 400: Versatile portrait film, warm tones
            // Known for: Beautiful skin tones, orange/warm cast, great latitude
            FilmPreset::Portra400 => FilmCharacteristics {
                contrast: 1.0,
                saturation: 0.95,
                highlights: -0.12,
                shadows: 0.15,
                blacks: 0.05,
                whites: -0.08,
                sharpening: 0.2,
                temperature: 0.08,  // Warm
                tint: 0.0,
                tone_curve_shadows: 0.1,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.08,
                s_curve_strength: 0.15,
                grain_amount: 0.1,
                grain_size: 1.0,
                grain_roughness: 0.4,
                halation_amount: 0.03,
                halation_radius: 1.0,
                halation_color: [1.0, 0.5, 0.3],
                black_point: 0.02,
                white_point: 0.98,
                shadow_tint: [0.03, 0.01, -0.02],  // Orange shadows
                highlight_tint: [0.02, 0.01, -0.01],
                vignette_amount: 0.05,
                vignette_softness: 1.3,
                latitude: 0.85,
                red_in_green: 0.03,
                green_in_red: 0.02,
                blue_in_green: -0.01,
                red_gamma: 1.03,
                green_gamma: 1.0,
                blue_gamma: 0.97,
                ..Default::default()
            },

            // Portra 800: High-speed portrait film
            // Known for: More grain, warmer, slightly less saturation
            FilmPreset::Portra800 => FilmCharacteristics {
                contrast: 1.02,
                saturation: 0.9,
                highlights: -0.1,
                shadows: 0.18,
                blacks: 0.06,
                whites: -0.1,
                sharpening: 0.18,
                temperature: 0.12,  // Warmer than 400
                tint: 0.02,
                tone_curve_shadows: 0.12,
                tone_curve_midtones: 0.03,
                tone_curve_highlights: -0.06,
                s_curve_strength: 0.18,
                grain_amount: 0.18,
                grain_size: 1.3,
                grain_roughness: 0.5,
                halation_amount: 0.04,
                halation_radius: 1.2,
                halation_color: [1.0, 0.45, 0.25],
                black_point: 0.025,
                white_point: 0.97,
                shadow_tint: [0.04, 0.02, -0.02],
                highlight_tint: [0.02, 0.01, 0.0],
                vignette_amount: 0.06,
                vignette_softness: 1.2,
                latitude: 0.8,
                red_in_green: 0.04,
                green_in_red: 0.02,
                red_gamma: 1.04,
                green_gamma: 1.0,
                blue_gamma: 0.96,
                ..Default::default()
            },

            // Ektar 100: Vivid, saturated landscape film
            // Known for: Punchy colors, deep blues, vivid reds, fine grain
            FilmPreset::Ektar100 => FilmCharacteristics {
                contrast: 1.15,
                saturation: 1.2,
                highlights: -0.08,
                shadows: 0.05,
                blacks: 0.02,
                whites: -0.03,
                sharpening: 0.25,
                temperature: -0.02,  // Slightly cool
                tint: -0.02,
                tone_curve_shadows: 0.03,
                tone_curve_midtones: 0.05,
                tone_curve_highlights: -0.05,
                s_curve_strength: 0.25,
                grain_amount: 0.04,
                grain_size: 0.7,
                grain_roughness: 0.25,
                halation_amount: 0.02,
                halation_radius: 0.6,
                halation_color: [1.0, 0.4, 0.2],
                black_point: 0.01,
                white_point: 0.995,
                shadow_tint: [0.0, -0.01, 0.02],  // Slightly blue shadows
                highlight_tint: [0.02, 0.0, -0.02],
                vignette_amount: 0.04,
                vignette_softness: 1.4,
                latitude: 0.6,  // Less latitude than Portra
                red_in_blue: 0.02,
                blue_in_red: 0.01,
                red_gamma: 1.05,
                green_gamma: 1.02,
                blue_gamma: 1.03,
                ..Default::default()
            },

            // Kodak Gold 200: Consumer film, nostalgic
            // Known for: Warm cast, slightly muted, nostalgic feel
            FilmPreset::KodakGold200 => FilmCharacteristics {
                contrast: 1.08,
                saturation: 1.05,
                highlights: -0.1,
                shadows: 0.08,
                blacks: 0.04,
                whites: -0.06,
                sharpening: 0.15,
                temperature: 0.15,  // Warm/golden
                tint: 0.03,
                tone_curve_shadows: 0.06,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.08,
                s_curve_strength: 0.2,
                grain_amount: 0.12,
                grain_size: 1.1,
                grain_roughness: 0.45,
                halation_amount: 0.03,
                halation_radius: 1.0,
                halation_color: [1.0, 0.6, 0.2],  // Golden halation
                black_point: 0.025,
                white_point: 0.975,
                shadow_tint: [0.04, 0.02, -0.03],  // Golden shadows
                highlight_tint: [0.03, 0.02, -0.02],
                vignette_amount: 0.08,
                vignette_softness: 1.1,
                latitude: 0.65,
                red_in_green: 0.03,
                green_in_red: 0.02,
                red_gamma: 1.05,
                green_gamma: 1.02,
                blue_gamma: 0.95,
                ..Default::default()
            },

            // ========== FUJIFILM COLOR NEGATIVE ==========

            // Fuji 400H: Soft pastels, slightly cool
            // Known for: Pastel rendering, great for overexposure, soft contrast
            FilmPreset::Fuji400H => FilmCharacteristics {
                contrast: 0.92,
                saturation: 0.88,
                highlights: -0.18,
                shadows: 0.15,
                blacks: 0.04,
                whites: -0.1,
                sharpening: 0.18,
                temperature: -0.03,  // Slightly cool
                tint: 0.02,
                tone_curve_shadows: 0.12,
                tone_curve_midtones: -0.02,
                tone_curve_highlights: -0.12,
                s_curve_strength: 0.08,
                grain_amount: 0.08,
                grain_size: 0.9,
                grain_roughness: 0.35,
                halation_amount: 0.02,
                halation_radius: 0.9,
                halation_color: [0.9, 0.7, 0.5],
                black_point: 0.02,
                white_point: 0.985,
                shadow_tint: [-0.01, 0.02, 0.02],  // Cool/green shadows
                highlight_tint: [0.01, 0.02, 0.0],
                vignette_amount: 0.04,
                vignette_softness: 1.4,
                latitude: 0.9,  // Excellent overexposure latitude
                green_in_red: 0.02,
                green_in_blue: 0.02,
                red_gamma: 0.98,
                green_gamma: 1.02,
                blue_gamma: 1.0,
                ..Default::default()
            },

            // Fuji Superia 400: Consumer film, green cast
            // Known for: Green-tinted shadows, punchy, everyday look
            FilmPreset::FujiSuperia400 => FilmCharacteristics {
                contrast: 1.1,
                saturation: 1.08,
                highlights: -0.08,
                shadows: 0.1,
                blacks: 0.03,
                whites: -0.05,
                sharpening: 0.2,
                temperature: -0.02,
                tint: -0.05,  // Green tint
                tone_curve_shadows: 0.05,
                tone_curve_midtones: 0.03,
                tone_curve_highlights: -0.06,
                s_curve_strength: 0.22,
                grain_amount: 0.14,
                grain_size: 1.15,
                grain_roughness: 0.5,
                halation_amount: 0.025,
                halation_radius: 0.85,
                halation_color: [0.8, 1.0, 0.6],
                black_point: 0.02,
                white_point: 0.98,
                shadow_tint: [-0.02, 0.04, -0.01],  // Green shadows
                highlight_tint: [0.0, 0.02, 0.0],
                vignette_amount: 0.06,
                vignette_softness: 1.2,
                latitude: 0.7,
                green_in_red: 0.03,
                green_in_blue: 0.02,
                red_gamma: 0.98,
                green_gamma: 1.04,
                blue_gamma: 0.99,
                ..Default::default()
            },

            // ========== FUJIFILM SLIDE (REVERSAL) ==========

            // Provia 100F: Neutral professional slide film
            // Known for: Accurate colors, moderate contrast, versatile
            FilmPreset::Provia100 => FilmCharacteristics {
                contrast: 1.12,
                saturation: 1.05,
                highlights: -0.05,
                shadows: 0.02,
                blacks: 0.01,
                whites: -0.02,
                sharpening: 0.22,
                temperature: 0.0,  // Neutral
                tint: 0.0,
                tone_curve_shadows: 0.02,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.04,
                s_curve_strength: 0.2,
                grain_amount: 0.05,
                grain_size: 0.75,
                grain_roughness: 0.3,
                halation_amount: 0.01,
                halation_radius: 0.5,
                halation_color: [1.0, 0.8, 0.6],
                black_point: 0.008,
                white_point: 0.995,
                shadow_tint: [0.0, 0.0, 0.01],
                highlight_tint: [0.0, 0.0, 0.0],
                vignette_amount: 0.02,
                vignette_softness: 1.5,
                latitude: 0.5,  // Slide film = less latitude
                red_gamma: 1.0,
                green_gamma: 1.0,
                blue_gamma: 1.01,
                ..Default::default()
            },

            // Velvia 50: Ultra-saturated landscape slide film
            // Known for: Intense saturation, deep blacks, vivid greens/blues
            FilmPreset::Velvia50 => FilmCharacteristics {
                contrast: 1.25,
                saturation: 1.35,
                highlights: -0.03,
                shadows: -0.05,  // Deeper shadows
                blacks: 0.0,
                whites: 0.0,
                sharpening: 0.3,
                temperature: 0.02,
                tint: -0.02,
                tone_curve_shadows: -0.05,
                tone_curve_midtones: 0.05,
                tone_curve_highlights: -0.02,
                s_curve_strength: 0.35,
                grain_amount: 0.03,
                grain_size: 0.6,
                grain_roughness: 0.2,
                halation_amount: 0.01,
                halation_radius: 0.4,
                halation_color: [1.0, 0.6, 0.3],
                black_point: 0.005,
                white_point: 0.998,
                shadow_tint: [0.0, 0.02, 0.03],  // Blue-green shadows
                highlight_tint: [0.02, 0.0, -0.02],
                vignette_amount: 0.03,
                vignette_softness: 1.3,
                latitude: 0.4,  // Very limited latitude
                blue_in_green: 0.02,
                green_in_blue: 0.02,
                red_gamma: 1.02,
                green_gamma: 1.05,
                blue_gamma: 1.04,
                ..Default::default()
            },

            // Velvia 100: Saturated but softer than Velvia 50
            // Known for: High saturation, slightly more forgiving than 50
            FilmPreset::Velvia100 => FilmCharacteristics {
                contrast: 1.2,
                saturation: 1.25,
                highlights: -0.05,
                shadows: -0.02,
                blacks: 0.01,
                whites: -0.01,
                sharpening: 0.28,
                temperature: 0.01,
                tint: -0.01,
                tone_curve_shadows: -0.02,
                tone_curve_midtones: 0.04,
                tone_curve_highlights: -0.03,
                s_curve_strength: 0.3,
                grain_amount: 0.04,
                grain_size: 0.7,
                grain_roughness: 0.25,
                halation_amount: 0.015,
                halation_radius: 0.5,
                halation_color: [1.0, 0.65, 0.35],
                black_point: 0.008,
                white_point: 0.996,
                shadow_tint: [0.0, 0.015, 0.025],
                highlight_tint: [0.015, 0.0, -0.015],
                vignette_amount: 0.035,
                vignette_softness: 1.35,
                latitude: 0.45,
                blue_in_green: 0.015,
                green_in_blue: 0.015,
                red_gamma: 1.02,
                green_gamma: 1.04,
                blue_gamma: 1.03,
                ..Default::default()
            },

            // Astia 100F: Soft contrast slide film
            // Known for: Lower contrast for slide, good skin tones, natural
            FilmPreset::Astia100 => FilmCharacteristics {
                contrast: 1.08,
                saturation: 1.0,
                highlights: -0.08,
                shadows: 0.05,
                blacks: 0.015,
                whites: -0.03,
                sharpening: 0.2,
                temperature: 0.02,
                tint: 0.01,
                tone_curve_shadows: 0.04,
                tone_curve_midtones: 0.0,
                tone_curve_highlights: -0.06,
                s_curve_strength: 0.15,
                grain_amount: 0.05,
                grain_size: 0.75,
                grain_roughness: 0.3,
                halation_amount: 0.015,
                halation_radius: 0.6,
                halation_color: [1.0, 0.7, 0.5],
                black_point: 0.01,
                white_point: 0.992,
                shadow_tint: [0.01, 0.0, 0.0],
                highlight_tint: [0.01, 0.005, -0.01],
                vignette_amount: 0.025,
                vignette_softness: 1.4,
                latitude: 0.55,  // Better latitude than Velvia
                red_gamma: 1.01,
                green_gamma: 1.0,
                blue_gamma: 0.99,
                ..Default::default()
            },

            // ========== KODAK B&W ==========

            // T-Max 100: Ultra-fine grain, smooth tones
            // Known for: Extremely fine grain, smooth gradation, modern look
            FilmPreset::TMax100 => FilmCharacteristics {
                is_bw: true,
                contrast: 1.08,
                saturation: 1.0,
                highlights: -0.08,
                shadows: 0.08,
                blacks: 0.02,
                whites: -0.04,
                sharpening: 0.35,
                tone_curve_shadows: 0.05,
                tone_curve_midtones: 0.0,
                tone_curve_highlights: -0.06,
                s_curve_strength: 0.18,
                grain_amount: 0.03,
                grain_size: 0.6,
                grain_roughness: 0.2,
                black_point: 0.02,
                white_point: 0.98,
                vignette_amount: 0.02,
                vignette_softness: 1.5,
                latitude: 0.85,
                ..Default::default()
            },

            // T-Max 400: Modern T-grain, fine grain for speed
            // Known for: Fine grain for ISO 400, broad tonal range
            FilmPreset::TMax400 => FilmCharacteristics {
                is_bw: true,
                contrast: 1.1,
                saturation: 1.0,
                highlights: -0.1,
                shadows: 0.1,
                blacks: 0.03,
                whites: -0.05,
                sharpening: 0.32,
                tone_curve_shadows: 0.06,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.07,
                s_curve_strength: 0.2,
                grain_amount: 0.08,
                grain_size: 0.85,
                grain_roughness: 0.3,
                black_point: 0.025,
                white_point: 0.975,
                vignette_amount: 0.03,
                vignette_softness: 1.4,
                latitude: 0.82,
                ..Default::default()
            },

            // Tri-X 400: Classic gritty B&W
            // Known for: Strong grain, punchy contrast, classic photojournalism look
            FilmPreset::TriX400 => FilmCharacteristics {
                is_bw: true,
                contrast: 1.18,
                saturation: 1.0,
                highlights: -0.05,
                shadows: 0.08,
                blacks: 0.05,
                whites: -0.08,
                sharpening: 0.28,
                tone_curve_shadows: 0.04,
                tone_curve_midtones: 0.05,
                tone_curve_highlights: -0.04,
                s_curve_strength: 0.28,
                grain_amount: 0.2,
                grain_size: 1.3,
                grain_roughness: 0.6,
                black_point: 0.04,
                white_point: 0.96,
                vignette_amount: 0.06,
                vignette_softness: 1.1,
                latitude: 0.75,
                ..Default::default()
            },

            // ========== ILFORD B&W ==========

            // HP5 Plus 400: Versatile, classic grain
            // Known for: Traditional grain, great pushability, versatile
            FilmPreset::Hp5 => FilmCharacteristics {
                is_bw: true,
                contrast: 1.12,
                saturation: 1.0,
                highlights: -0.08,
                shadows: 0.12,
                blacks: 0.04,
                whites: -0.06,
                sharpening: 0.25,
                tone_curve_shadows: 0.08,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.06,
                s_curve_strength: 0.22,
                grain_amount: 0.16,
                grain_size: 1.2,
                grain_roughness: 0.55,
                black_point: 0.035,
                white_point: 0.965,
                vignette_amount: 0.05,
                vignette_softness: 1.2,
                latitude: 0.8,
                ..Default::default()
            },

            // Delta 100: Modern fine grain
            // Known for: Very fine grain, smooth gradation, modern look
            FilmPreset::Delta100 => FilmCharacteristics {
                is_bw: true,
                contrast: 1.06,
                saturation: 1.0,
                highlights: -0.1,
                shadows: 0.08,
                blacks: 0.02,
                whites: -0.04,
                sharpening: 0.38,
                tone_curve_shadows: 0.06,
                tone_curve_midtones: -0.01,
                tone_curve_highlights: -0.08,
                s_curve_strength: 0.15,
                grain_amount: 0.025,
                grain_size: 0.55,
                grain_roughness: 0.2,
                black_point: 0.015,
                white_point: 0.985,
                vignette_amount: 0.02,
                vignette_softness: 1.5,
                latitude: 0.78,
                ..Default::default()
            },

            // Delta 3200: High-speed, pronounced grain
            // Known for: Large grain, moody look, low-light capability
            FilmPreset::Delta3200 => FilmCharacteristics {
                is_bw: true,
                contrast: 1.15,
                saturation: 1.0,
                highlights: -0.05,
                shadows: 0.15,
                blacks: 0.06,
                whites: -0.1,
                sharpening: 0.2,
                tone_curve_shadows: 0.1,
                tone_curve_midtones: 0.04,
                tone_curve_highlights: -0.04,
                s_curve_strength: 0.25,
                grain_amount: 0.3,
                grain_size: 1.6,
                grain_roughness: 0.7,
                black_point: 0.05,
                white_point: 0.94,
                vignette_amount: 0.08,
                vignette_softness: 1.0,
                latitude: 0.7,
                ..Default::default()
            },

            // Pan F Plus 50: Extremely fine grain
            // Known for: Exceptional sharpness, almost grainless, high contrast
            FilmPreset::PanF50 => FilmCharacteristics {
                is_bw: true,
                contrast: 1.15,
                saturation: 1.0,
                highlights: -0.06,
                shadows: 0.04,
                blacks: 0.01,
                whites: -0.02,
                sharpening: 0.45,
                tone_curve_shadows: 0.02,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.04,
                s_curve_strength: 0.22,
                grain_amount: 0.015,
                grain_size: 0.4,
                grain_roughness: 0.15,
                black_point: 0.01,
                white_point: 0.99,
                vignette_amount: 0.02,
                vignette_softness: 1.6,
                latitude: 0.6,
                ..Default::default()
            },

            // ========== CINEMATIC ==========

            // CineStill 800T: Tungsten cinema film with halation
            // Known for: Strong halation, teal/orange look, tungsten balance
            FilmPreset::CineStill800T => FilmCharacteristics {
                contrast: 1.05,
                saturation: 0.95,
                highlights: -0.12,
                shadows: 0.15,
                blacks: 0.04,
                whites: -0.08,
                sharpening: 0.18,
                temperature: -0.15,  // Tungsten = cool
                tint: -0.02,
                tone_curve_shadows: 0.1,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.1,
                s_curve_strength: 0.18,
                grain_amount: 0.15,
                grain_size: 1.2,
                grain_roughness: 0.45,
                halation_amount: 0.15,  // Strong halation!
                halation_radius: 2.0,
                halation_color: [1.0, 0.3, 0.1],  // Red/orange halation
                black_point: 0.025,
                white_point: 0.975,
                shadow_tint: [-0.02, 0.02, 0.05],  // Teal shadows
                highlight_tint: [0.05, 0.02, -0.03],  // Orange highlights
                vignette_amount: 0.06,
                vignette_softness: 1.2,
                latitude: 0.75,
                blue_in_red: 0.02,
                red_in_blue: -0.02,
                red_gamma: 1.02,
                green_gamma: 1.0,
                blue_gamma: 1.05,
                ..Default::default()
            },

            // CineStill 50D: Daylight cinema film
            // Known for: Rich colors, subtle halation, cinematic look
            FilmPreset::CineStill50D => FilmCharacteristics {
                contrast: 1.1,
                saturation: 1.05,
                highlights: -0.08,
                shadows: 0.1,
                blacks: 0.02,
                whites: -0.05,
                sharpening: 0.25,
                temperature: 0.0,  // Daylight balanced
                tint: 0.0,
                tone_curve_shadows: 0.06,
                tone_curve_midtones: 0.02,
                tone_curve_highlights: -0.06,
                s_curve_strength: 0.2,
                grain_amount: 0.06,
                grain_size: 0.8,
                grain_roughness: 0.3,
                halation_amount: 0.08,  // Moderate halation
                halation_radius: 1.5,
                halation_color: [1.0, 0.35, 0.15],
                black_point: 0.015,
                white_point: 0.985,
                shadow_tint: [0.0, 0.01, 0.02],
                highlight_tint: [0.02, 0.01, -0.02],
                vignette_amount: 0.04,
                vignette_softness: 1.3,
                latitude: 0.8,
                red_gamma: 1.02,
                green_gamma: 1.01,
                blue_gamma: 1.0,
                ..Default::default()
            },
        }
    }
}