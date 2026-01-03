use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmEmulation {
    pub enabled: bool,
    pub is_bw: bool, // Whether this is a B&W film (converts color to mono)

    // Tone curve control points (shadows, midtones, highlights) - values 0.0 to 1.0
    pub tone: FilmTone,

    // Film grain simulation
    pub grain: FilmGrain,

    // Halation (light bloom around bright areas, characteristic of film)
    pub halation: FilmHalation,

    // Color channel crossover/crosstalk (film layers interact)
    pub color_crossover: FilmColorCrossover,

    // Color response curves (per-channel gamma/lift)
    pub color_gamma: FilmColorGamma,

    // Black point and white point (film base density and max density)
    pub black_point: f32, // 0.0 to 0.1 (raised blacks = faded look)
    pub white_point: f32, // 0.9 to 1.0 (compressed highlights)

    // Color cast/tint in shadows and highlights
    pub shadow_tint: [f32; 3],    // RGB tint for shadows
    pub highlight_tint: [f32; 3], // RGB tint for highlights

    // Vignette (natural lens falloff)
    pub vignette: FilmVignette,

    // Film latitude (dynamic range compression)
    pub latitude: f32, // 0.0 to 1.0 (higher = more DR recovery)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmTone {
    pub shadows: f32,          // Lift/lower shadows (-1.0 to 1.0)
    pub midtones: f32,         // Adjust midtones (-1.0 to 1.0)
    pub highlights: f32,       // Compress/expand highlights (-1.0 to 1.0)
    pub s_curve_strength: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmGrain {
    pub amount: f32,    // 0.0 to 1.0 (intensity)
    pub size: f32,      // 0.5 to 2.0 (1.0 = normal)
    pub roughness: f32, // 0.0 to 1.0 (organic variation)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmHalation {
    pub amount: f32,     // 0.0 to 1.0
    pub radius: f32,     // Spread of the halation effect
    pub color: [f32; 3], // RGB tint for halation (usually warm/red)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmColorCrossover {
    pub red_in_green: f32,  // -0.2 to 0.2
    pub red_in_blue: f32,   // -0.2 to 0.2
    pub green_in_red: f32,  // -0.2 to 0.2
    pub green_in_blue: f32, // -0.2 to 0.2
    pub blue_in_red: f32,   // -0.2 to 0.2
    pub blue_in_green: f32, // -0.2 to 0.2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmColorGamma {
    pub red: f32,   // 0.8 to 1.2
    pub green: f32, // 0.8 to 1.2
    pub blue: f32,  // 0.8 to 1.2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilmVignette {
    pub amount: f32,   // 0.0 to 1.0
    pub softness: f32, // 0.5 to 2.0
}

impl Default for FilmTone {
    fn default() -> Self {
        Self {
            shadows: 0.0,
            midtones: 0.0,
            highlights: 0.0,
            s_curve_strength: 0.0,
        }
    }
}

impl Default for FilmGrain {
    fn default() -> Self {
        Self {
            amount: 0.0,
            size: 1.0,
            roughness: 0.5,
        }
    }
}

impl Default for FilmHalation {
    fn default() -> Self {
        Self {
            amount: 0.0,
            radius: 1.0,
            color: [1.0, 0.3, 0.1], // Warm red/orange
        }
    }
}

impl Default for FilmColorCrossover {
    fn default() -> Self {
        Self {
            red_in_green: 0.0,
            red_in_blue: 0.0,
            green_in_red: 0.0,
            green_in_blue: 0.0,
            blue_in_red: 0.0,
            blue_in_green: 0.0,
        }
    }
}

impl Default for FilmColorGamma {
    fn default() -> Self {
        Self {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
        }
    }
}

impl Default for FilmVignette {
    fn default() -> Self {
        Self {
            amount: 0.0,
            softness: 1.0,
        }
    }
}

impl Default for FilmEmulation {
    fn default() -> Self {
        Self {
            enabled: false,
            is_bw: false,
            tone: FilmTone::default(),
            grain: FilmGrain::default(),
            halation: FilmHalation::default(),
            color_crossover: FilmColorCrossover::default(),
            color_gamma: FilmColorGamma::default(),
            black_point: 0.0,
            white_point: 1.0,
            shadow_tint: [0.0, 0.0, 0.0],
            highlight_tint: [0.0, 0.0, 0.0],
            vignette: FilmVignette::default(),
            latitude: 0.0,
        }
    }
}

// Apply basic adjustments (non-destructive preview)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAdjustments {
    pub exposure: f32,       // -3.0 to +3.0 (stops)
    pub saturation: f32,     // 0.0 to 2.0 (multiplier)
    pub temperature: f32,    // -1.0 to +1.0 (cool to warm)
    pub film: FilmEmulation, // Film emulation parameters
    pub frame_enabled: bool,
    pub frame_color: [f32; 3], // RGB 0-1
    pub frame_thickness: f32,  // pixels
}

impl Default for ImageAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            saturation: 1.0,
            temperature: 0.0,
            film: FilmEmulation::default(),
            frame_enabled: false,
            frame_color: [1.0, 1.0, 1.0], // white
            frame_thickness: 80.0,
        }
    }
}

/// Film stock characteristics used to build presets
struct FilmCharacteristics {
    // Base adjustments
    saturation: f32,
    temperature: f32,

    // Film-specific parameters
    is_bw: bool,
    tone: FilmTone,
    grain: FilmGrain,
    halation: FilmHalation,
    black_point: f32,
    white_point: f32,
    shadow_tint: [f32; 3],
    highlight_tint: [f32; 3],
    vignette: FilmVignette,
    latitude: f32,
    // Color crossover matrix
    color_crossover: FilmColorCrossover,
    // Per-channel gamma
    color_gamma: FilmColorGamma,
}

impl Default for FilmCharacteristics {
    fn default() -> Self {
        Self {
            saturation: 1.0,
            temperature: 0.0,
            is_bw: false,
            tone: FilmTone::default(),
            grain: FilmGrain::default(),
            halation: FilmHalation::default(),
            black_point: 0.0,
            white_point: 1.0,
            shadow_tint: [0.0, 0.0, 0.0],
            highlight_tint: [0.0, 0.0, 0.0],
            vignette: FilmVignette::default(),
            latitude: 0.0,
            color_crossover: FilmColorCrossover::default(),
            color_gamma: FilmColorGamma::default(),
        }
    }
}

impl FilmCharacteristics {
    fn to_adjustments(&self) -> ImageAdjustments {
        ImageAdjustments {
            exposure: 0.0,
            saturation: self.saturation,
            temperature: self.temperature,
            film: FilmEmulation {
                enabled: true,
                is_bw: self.is_bw,
                tone: self.tone.clone(),
                grain: self.grain.clone(),
                halation: self.halation.clone(),
                color_crossover: self.color_crossover.clone(),
                color_gamma: self.color_gamma.clone(),
                black_point: self.black_point,
                white_point: self.white_point,
                shadow_tint: self.shadow_tint,
                highlight_tint: self.highlight_tint,
                vignette: self.vignette.clone(),
                latitude: self.latitude,
            },
            frame_enabled: false,
            frame_color: [1.0, 1.0, 1.0],
            frame_thickness: 80.0,
        }
    }
}

impl ImageAdjustments {
    pub fn is_default(&self) -> bool {
        self.exposure == 0.0
            && self.saturation == 1.0
            && self.temperature == 0.0
            && !self.film.enabled
            && !self.frame_enabled
    }

    /// Create a lightweight version of the adjustments for fast previews while dragging sliders.
    /// This disables expensive effects like film grain, halation, S-curve and sharpening.
    pub fn preview(&self) -> Self {
        let mut p = self.clone();
        // Disable film emulation for preview to skip heavy multi-pass operations
        p.film.enabled = false;
        // Zero out film-specific heavy features
        p.film.grain.amount = 0.0;
        p.film.halation.amount = 0.0;
        p.film.tone.s_curve_strength = 0.0;
        p.film.vignette.amount = 0.0;
        p.film.latitude = 0.0;
        p
    }

    pub fn apply_preset(&mut self, preset: FilmPreset) {
        // Preserve frame settings across preset changes
        let frame_enabled = self.frame_enabled;
        let frame_color = self.frame_color;
        let frame_thickness = self.frame_thickness;

        if preset == FilmPreset::None {
            // Reset all adjustments to default when None is selected
            *self = ImageAdjustments::default();
        } else {
            // Replace all adjustments with preset values
            *self = preset.characteristics().to_adjustments();
        }

        // Restore frame settings
        self.frame_enabled = frame_enabled;
        self.frame_color = frame_color;
        self.frame_thickness = frame_thickness;
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
                saturation: 0.92,
                temperature: 0.05, // Slightly warm
                tone: FilmTone {
                    shadows: 0.08,
                    midtones: 0.0,
                    highlights: -0.1,
                    s_curve_strength: 0.1,
                },
                grain: FilmGrain {
                    amount: 0.05,
                    size: 0.8,
                    roughness: 0.3,
                },
                halation: FilmHalation {
                    amount: 0.02,
                    radius: 0.8,
                    color: [1.0, 0.6, 0.4],
                },
                black_point: 0.015,
                white_point: 0.99,
                shadow_tint: [0.02, 0.01, -0.01], // Warm shadows
                highlight_tint: [0.01, 0.0, -0.02],
                vignette: FilmVignette {
                    amount: 0.03,
                    softness: 1.5,
                },
                latitude: 0.9, // Excellent latitude
                color_crossover: FilmColorCrossover {
                    red_in_green: 0.02,
                    green_in_red: 0.01,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.02,
                    green: 1.0,
                    blue: 0.98,
                },
                ..Default::default()
            },

            // Portra 400: Versatile portrait film, warm tones
            // Known for: Beautiful skin tones, orange/warm cast, great latitude
            FilmPreset::Portra400 => FilmCharacteristics {
                saturation: 0.95,
                temperature: 0.08, // Warm
                tone: FilmTone {
                    shadows: 0.1,
                    midtones: 0.02,
                    highlights: -0.08,
                    s_curve_strength: 0.15,
                },
                grain: FilmGrain {
                    amount: 0.1,
                    size: 1.0,
                    roughness: 0.4,
                },
                halation: FilmHalation {
                    amount: 0.03,
                    radius: 1.0,
                    color: [1.0, 0.5, 0.3],
                },
                black_point: 0.02,
                white_point: 0.98,
                shadow_tint: [0.03, 0.01, -0.02], // Orange shadows
                highlight_tint: [0.02, 0.01, -0.01],
                vignette: FilmVignette {
                    amount: 0.05,
                    softness: 1.3,
                },
                latitude: 0.85,
                color_crossover: FilmColorCrossover {
                    red_in_green: 0.03,
                    green_in_red: 0.02,
                    blue_in_green: -0.01,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.03,
                    green: 1.0,
                    blue: 0.97,
                },
                ..Default::default()
            },

            // Portra 800: High-speed portrait film
            // Known for: More grain, warmer, slightly less saturation
            FilmPreset::Portra800 => FilmCharacteristics {
                saturation: 0.9,
                temperature: 0.12, // Warmer than 400
                tone: FilmTone {
                    shadows: 0.12,
                    midtones: 0.03,
                    highlights: -0.06,
                    s_curve_strength: 0.18,
                },
                grain: FilmGrain {
                    amount: 0.18,
                    size: 1.3,
                    roughness: 0.5,
                },
                halation: FilmHalation {
                    amount: 0.04,
                    radius: 1.2,
                    color: [1.0, 0.45, 0.25],
                },
                black_point: 0.025,
                white_point: 0.97,
                shadow_tint: [0.04, 0.02, -0.02],
                highlight_tint: [0.02, 0.01, 0.0],
                vignette: FilmVignette {
                    amount: 0.06,
                    softness: 1.2,
                },
                latitude: 0.8,
                color_crossover: FilmColorCrossover {
                    red_in_green: 0.04,
                    green_in_red: 0.02,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.04,
                    green: 1.0,
                    blue: 0.96,
                },
                ..Default::default()
            },

            // Ektar 100: Vivid, saturated landscape film
            // Known for: Punchy colors, deep blues, vivid reds, fine grain
            FilmPreset::Ektar100 => FilmCharacteristics {
                saturation: 1.2,
                temperature: -0.02, // Slightly cool
                tone: FilmTone {
                    shadows: 0.03,
                    midtones: 0.05,
                    highlights: -0.05,
                    s_curve_strength: 0.25,
                },
                grain: FilmGrain {
                    amount: 0.04,
                    size: 0.7,
                    roughness: 0.25,
                },
                halation: FilmHalation {
                    amount: 0.02,
                    radius: 0.6,
                    color: [1.0, 0.4, 0.2],
                },
                black_point: 0.01,
                white_point: 0.995,
                shadow_tint: [0.0, -0.01, 0.02], // Slightly blue shadows
                highlight_tint: [0.02, 0.0, -0.02],
                vignette: FilmVignette {
                    amount: 0.04,
                    softness: 1.4,
                },
                latitude: 0.6, // Less latitude than Portra
                color_crossover: FilmColorCrossover {
                    red_in_blue: 0.02,
                    blue_in_red: 0.01,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.05,
                    green: 1.02,
                    blue: 1.03,
                },
                ..Default::default()
            },

            // Kodak Gold 200: Consumer film, nostalgic
            // Known for: Warm cast, slightly muted, nostalgic feel
            FilmPreset::KodakGold200 => FilmCharacteristics {
                saturation: 1.05,
                temperature: 0.15, // Warm/golden
                tone: FilmTone {
                    shadows: 0.06,
                    midtones: 0.02,
                    highlights: -0.08,
                    s_curve_strength: 0.2,
                },
                grain: FilmGrain {
                    amount: 0.12,
                    size: 1.1,
                    roughness: 0.45,
                },
                halation: FilmHalation {
                    amount: 0.03,
                    radius: 1.0,
                    color: [1.0, 0.6, 0.2], // Golden halation
                },
                black_point: 0.025,
                white_point: 0.975,
                shadow_tint: [0.04, 0.02, -0.03], // Golden shadows
                highlight_tint: [0.03, 0.02, -0.02],
                vignette: FilmVignette {
                    amount: 0.08,
                    softness: 1.1,
                },
                latitude: 0.65,
                color_crossover: FilmColorCrossover {
                    red_in_green: 0.03,
                    green_in_red: 0.02,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.05,
                    green: 1.02,
                    blue: 0.95,
                },
                ..Default::default()
            },

            // ========== FUJIFILM COLOR NEGATIVE ==========

            // Fuji 400H: Soft pastels, slightly cool
            // Known for: Pastel rendering, great for overexposure, soft contrast
            FilmPreset::Fuji400H => FilmCharacteristics {
                saturation: 0.88,
                temperature: -0.03, // Slightly cool
                tone: FilmTone {
                    shadows: 0.12,
                    midtones: -0.02,
                    highlights: -0.12,
                    s_curve_strength: 0.08,
                },
                grain: FilmGrain {
                    amount: 0.08,
                    size: 0.9,
                    roughness: 0.35,
                },
                halation: FilmHalation {
                    amount: 0.02,
                    radius: 0.9,
                    color: [0.9, 0.7, 0.5],
                },
                black_point: 0.02,
                white_point: 0.985,
                shadow_tint: [-0.01, 0.02, 0.02], // Cool/green shadows
                highlight_tint: [0.01, 0.02, 0.0],
                vignette: FilmVignette {
                    amount: 0.04,
                    softness: 1.4,
                },
                latitude: 0.9, // Excellent overexposure latitude
                color_crossover: FilmColorCrossover {
                    green_in_red: 0.02,
                    green_in_blue: 0.02,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 0.98,
                    green: 1.02,
                    blue: 1.0,
                },
                ..Default::default()
            },

            // Fuji Superia 400: Consumer film, green cast
            // Known for: Green-tinted shadows, punchy, everyday look
            FilmPreset::FujiSuperia400 => FilmCharacteristics {
                saturation: 1.08,
                temperature: -0.02,
                tone: FilmTone {
                    shadows: 0.05,
                    midtones: 0.03,
                    highlights: -0.06,
                    s_curve_strength: 0.22,
                },
                grain: FilmGrain {
                    amount: 0.14,
                    size: 1.15,
                    roughness: 0.5,
                },
                halation: FilmHalation {
                    amount: 0.025,
                    radius: 0.85,
                    color: [0.8, 1.0, 0.6],
                },
                black_point: 0.02,
                white_point: 0.98,
                shadow_tint: [-0.02, 0.04, -0.01], // Green shadows
                highlight_tint: [0.0, 0.02, 0.0],
                vignette: FilmVignette {
                    amount: 0.06,
                    softness: 1.2,
                },
                latitude: 0.7,
                color_crossover: FilmColorCrossover {
                    green_in_red: 0.03,
                    green_in_blue: 0.02,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 0.98,
                    green: 1.04,
                    blue: 0.99,
                },
                ..Default::default()
            },

            // ========== FUJIFILM SLIDE (REVERSAL) ==========

            // Provia 100F: Neutral professional slide film
            // Known for: Accurate colors, moderate contrast, versatile
            FilmPreset::Provia100 => FilmCharacteristics {
                saturation: 1.05,
                temperature: 0.0, // Neutral
                tone: FilmTone {
                    shadows: 0.02,
                    midtones: 0.02,
                    highlights: -0.04,
                    s_curve_strength: 0.2,
                },
                grain: FilmGrain {
                    amount: 0.05,
                    size: 0.75,
                    roughness: 0.3,
                },
                halation: FilmHalation {
                    amount: 0.01,
                    radius: 0.5,
                    color: [1.0, 0.8, 0.6],
                },
                black_point: 0.008,
                white_point: 0.995,
                shadow_tint: [0.0, 0.0, 0.01],
                highlight_tint: [0.0, 0.0, 0.0],
                vignette: FilmVignette {
                    amount: 0.02,
                    softness: 1.5,
                },
                latitude: 0.5, // Slide film = less latitude
                color_gamma: FilmColorGamma {
                    red: 1.0,
                    green: 1.0,
                    blue: 1.01,
                },
                ..Default::default()
            },

            // Velvia 50: Ultra-saturated landscape slide film
            // Known for: Intense saturation, deep blacks, vivid greens/blues
            FilmPreset::Velvia50 => FilmCharacteristics {
                saturation: 1.35,
                temperature: 0.02,
                tone: FilmTone {
                    shadows: -0.05,
                    midtones: 0.05,
                    highlights: -0.02,
                    s_curve_strength: 0.35,
                },
                grain: FilmGrain {
                    amount: 0.03,
                    size: 0.6,
                    roughness: 0.2,
                },
                halation: FilmHalation {
                    amount: 0.01,
                    radius: 0.4,
                    color: [1.0, 0.6, 0.3],
                },
                black_point: 0.005,
                white_point: 0.998,
                shadow_tint: [0.0, 0.02, 0.03], // Blue-green shadows
                highlight_tint: [0.02, 0.0, -0.02],
                vignette: FilmVignette {
                    amount: 0.03,
                    softness: 1.3,
                },
                latitude: 0.4, // Very limited latitude
                color_crossover: FilmColorCrossover {
                    blue_in_green: 0.02,
                    green_in_blue: 0.02,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.02,
                    green: 1.05,
                    blue: 1.04,
                },
                ..Default::default()
            },

            // Velvia 100: Saturated but softer than Velvia 50
            // Known for: High saturation, slightly more forgiving than 50
            FilmPreset::Velvia100 => FilmCharacteristics {
                saturation: 1.25,
                temperature: 0.01,
                tone: FilmTone {
                    shadows: -0.02,
                    midtones: 0.04,
                    highlights: -0.03,
                    s_curve_strength: 0.3,
                },
                grain: FilmGrain {
                    amount: 0.04,
                    size: 0.7,
                    roughness: 0.25,
                },
                halation: FilmHalation {
                    amount: 0.015,
                    radius: 0.5,
                    color: [1.0, 0.65, 0.35],
                },
                black_point: 0.008,
                white_point: 0.996,
                shadow_tint: [0.0, 0.015, 0.025],
                highlight_tint: [0.015, 0.0, -0.015],
                vignette: FilmVignette {
                    amount: 0.035,
                    softness: 1.35,
                },
                latitude: 0.45,
                color_crossover: FilmColorCrossover {
                    blue_in_green: 0.015,
                    green_in_blue: 0.015,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.02,
                    green: 1.04,
                    blue: 1.03,
                },
                ..Default::default()
            },

            // Astia 100F: Soft contrast slide film
            // Known for: Lower contrast for slide, good skin tones, natural
            FilmPreset::Astia100 => FilmCharacteristics {
                saturation: 1.0,
                temperature: 0.02,
                tone: FilmTone {
                    shadows: 0.04,
                    midtones: 0.0,
                    highlights: -0.06,
                    s_curve_strength: 0.15,
                },
                grain: FilmGrain {
                    amount: 0.05,
                    size: 0.75,
                    roughness: 0.3,
                },
                halation: FilmHalation {
                    amount: 0.015,
                    radius: 0.6,
                    color: [1.0, 0.7, 0.5],
                },
                black_point: 0.01,
                white_point: 0.992,
                shadow_tint: [0.01, 0.0, 0.0],
                highlight_tint: [0.01, 0.005, -0.01],
                vignette: FilmVignette {
                    amount: 0.025,
                    softness: 1.4,
                },
                latitude: 0.55, // Better latitude than Velvia
                color_gamma: FilmColorGamma {
                    red: 1.01,
                    green: 1.0,
                    blue: 0.99,
                },
                ..Default::default()
            },

            // ========== KODAK B&W ==========

            // T-Max 100: Ultra-fine grain, smooth tones
            // Known for: Extremely fine grain, smooth gradation, modern look
            FilmPreset::TMax100 => FilmCharacteristics {
                is_bw: true,
                saturation: 1.0,
                temperature: 0.0,
                tone: FilmTone {
                    shadows: 0.05,
                    midtones: 0.0,
                    highlights: -0.06,
                    s_curve_strength: 0.18,
                },
                grain: FilmGrain {
                    amount: 0.03,
                    size: 0.6,
                    roughness: 0.2,
                },
                black_point: 0.02,
                white_point: 0.98,
                vignette: FilmVignette {
                    amount: 0.02,
                    softness: 1.5,
                },
                latitude: 0.85,
                ..Default::default()
            },

            // T-Max 400: Modern T-grain, fine grain for speed
            // Known for: Fine grain for ISO 400, broad tonal range
            FilmPreset::TMax400 => FilmCharacteristics {
                is_bw: true,
                saturation: 1.0,
                temperature: 0.0,
                tone: FilmTone {
                    shadows: 0.06,
                    midtones: 0.02,
                    highlights: -0.07,
                    s_curve_strength: 0.2,
                },
                grain: FilmGrain {
                    amount: 0.08,
                    size: 0.85,
                    roughness: 0.3,
                },
                black_point: 0.025,
                white_point: 0.975,
                vignette: FilmVignette {
                    amount: 0.03,
                    softness: 1.4,
                },
                latitude: 0.82,
                ..Default::default()
            },

            // Tri-X 400: Classic gritty B&W
            // Known for: Strong grain, punchy contrast, classic photojournalism look
            FilmPreset::TriX400 => FilmCharacteristics {
                is_bw: true,
                saturation: 1.0,
                temperature: 0.0,
                tone: FilmTone {
                    shadows: 0.04,
                    midtones: 0.05,
                    highlights: -0.04,
                    s_curve_strength: 0.28,
                },
                grain: FilmGrain {
                    amount: 0.2,
                    size: 1.3,
                    roughness: 0.6,
                },
                black_point: 0.04,
                white_point: 0.96,
                vignette: FilmVignette {
                    amount: 0.06,
                    softness: 1.1,
                },
                latitude: 0.75,
                ..Default::default()
            },

            // ========== ILFORD B&W ==========

            // HP5 Plus 400: Versatile, classic grain
            // Known for: Traditional grain, great pushability, versatile
            FilmPreset::Hp5 => FilmCharacteristics {
                is_bw: true,
                saturation: 1.0,
                temperature: 0.0,
                tone: FilmTone {
                    shadows: 0.08,
                    midtones: 0.02,
                    highlights: -0.06,
                    s_curve_strength: 0.22,
                },
                grain: FilmGrain {
                    amount: 0.16,
                    size: 1.2,
                    roughness: 0.55,
                },
                black_point: 0.035,
                white_point: 0.965,
                vignette: FilmVignette {
                    amount: 0.05,
                    softness: 1.2,
                },
                latitude: 0.8,
                ..Default::default()
            },

            // Delta 100: Modern fine grain
            // Known for: Very fine grain, smooth gradation, modern look
            FilmPreset::Delta100 => FilmCharacteristics {
                is_bw: true,
                saturation: 1.0,
                temperature: 0.0,
                tone: FilmTone {
                    shadows: 0.06,
                    midtones: -0.01,
                    highlights: -0.08,
                    s_curve_strength: 0.15,
                },
                grain: FilmGrain {
                    amount: 0.025,
                    size: 0.55,
                    roughness: 0.2,
                },
                black_point: 0.015,
                white_point: 0.985,
                vignette: FilmVignette {
                    amount: 0.02,
                    softness: 1.5,
                },
                latitude: 0.78,
                ..Default::default()
            },

            // Delta 3200: High-speed, pronounced grain
            // Known for: Large grain, moody look, low-light capability
            FilmPreset::Delta3200 => FilmCharacteristics {
                is_bw: true,
                saturation: 1.0,
                temperature: 0.0,
                tone: FilmTone {
                    shadows: 0.1,
                    midtones: 0.04,
                    highlights: -0.04,
                    s_curve_strength: 0.25,
                },
                grain: FilmGrain {
                    amount: 0.3,
                    size: 1.6,
                    roughness: 0.7,
                },
                black_point: 0.05,
                white_point: 0.94,
                vignette: FilmVignette {
                    amount: 0.08,
                    softness: 1.0,
                },
                latitude: 0.7,
                ..Default::default()
            },

            // Pan F Plus 50: Extremely fine grain
            // Known for: Exceptional sharpness, almost grainless, high contrast
            FilmPreset::PanF50 => FilmCharacteristics {
                is_bw: true,
                saturation: 1.0,
                temperature: 0.0,
                tone: FilmTone {
                    shadows: 0.02,
                    midtones: 0.02,
                    highlights: -0.04,
                    s_curve_strength: 0.22,
                },
                grain: FilmGrain {
                    amount: 0.015,
                    size: 0.4,
                    roughness: 0.15,
                },
                black_point: 0.01,
                white_point: 0.99,
                vignette: FilmVignette {
                    amount: 0.02,
                    softness: 1.6,
                },
                latitude: 0.6,
                ..Default::default()
            },

            // ========== CINEMATIC ==========

            // CineStill 800T: Tungsten cinema film with halation
            // Known for: Strong halation, teal/orange look, tungsten balance
            FilmPreset::CineStill800T => FilmCharacteristics {
                saturation: 0.95,
                temperature: -0.15, // Tungsten = cool
                tone: FilmTone {
                    shadows: 0.1,
                    midtones: 0.02,
                    highlights: -0.1,
                    s_curve_strength: 0.18,
                },
                grain: FilmGrain {
                    amount: 0.15,
                    size: 1.2,
                    roughness: 0.45,
                },
                halation: FilmHalation {
                    amount: 0.15, // Strong halation!
                    radius: 2.0,
                    color: [1.0, 0.3, 0.1], // Red/orange halation
                },
                black_point: 0.025,
                white_point: 0.975,
                shadow_tint: [-0.02, 0.02, 0.05],    // Teal shadows
                highlight_tint: [0.05, 0.02, -0.03], // Orange highlights
                vignette: FilmVignette {
                    amount: 0.06,
                    softness: 1.2,
                },
                latitude: 0.75,
                color_crossover: FilmColorCrossover {
                    blue_in_red: 0.02,
                    red_in_blue: -0.02,
                    ..Default::default()
                },
                color_gamma: FilmColorGamma {
                    red: 1.02,
                    green: 1.0,
                    blue: 1.05,
                },
                ..Default::default()
            },

            // CineStill 50D: Daylight cinema film
            // Known for: Rich colors, subtle halation, cinematic look
            FilmPreset::CineStill50D => FilmCharacteristics {
                saturation: 1.05,
                temperature: 0.0, // Daylight balanced
                tone: FilmTone {
                    shadows: 0.06,
                    midtones: 0.02,
                    highlights: -0.06,
                    s_curve_strength: 0.2,
                },
                grain: FilmGrain {
                    amount: 0.06,
                    size: 0.8,
                    roughness: 0.3,
                },
                halation: FilmHalation {
                    amount: 0.08, // Moderate halation
                    radius: 1.5,
                    color: [1.0, 0.35, 0.15],
                },
                black_point: 0.015,
                white_point: 0.985,
                shadow_tint: [0.0, 0.01, 0.02],
                highlight_tint: [0.02, 0.01, -0.02],
                vignette: FilmVignette {
                    amount: 0.04,
                    softness: 1.3,
                },
                latitude: 0.8,
                color_gamma: FilmColorGamma {
                    red: 1.02,
                    green: 1.01,
                    blue: 1.0,
                },
                ..Default::default()
            },
        }
    }
}
