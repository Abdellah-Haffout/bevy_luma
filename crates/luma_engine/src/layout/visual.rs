//! Advanced visual styling components for Univis UI nodes.
//!
//! Provides [`UGradient`](crate::layout::visual::UGradient) for linear and radial gradient fills and
//! [`UShadow`](crate::layout::visual::UShadow) for outer glows, drop shadows, and inner holographic glows.

use bevy::prelude::*;

/// Direction or focal mapping for a [`UGradient`].
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum UGradientKind {
    /// Linear gradient along an angle in radians (0.0 = left-to-right, PI/2 = top-to-bottom).
    Linear {
        /// Angle in radians.
        angle: f32,
    },
    /// Radial gradient centered at normalized coordinates (0.5, 0.5 = center of node).
    Radial {
        /// Center in normalized [0.0, 1.0] UV coordinates.
        center: Vec2,
        /// Relative radius (e.g. 0.5 reaches the edge).
        radius: f32,
    },
    /// Conic / angular sweep gradient centered at normalized coordinates (0.5, 0.5 = center of node).
    Angular {
        /// Center in normalized [0.0, 1.0] UV coordinates.
        center: Vec2,
    },
}

/// A single color stop in a [`UGradient`].
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct UGradientStop {
    /// Color at this stop position.
    pub color: Color,
    /// Normalized position along the gradient line in the range `[0.0, 1.0]`.
    pub position: f32,
}

impl UGradientStop {
    /// Creates a new gradient stop.
    pub fn new(position: f32, color: Color) -> Self {
        Self {
            position: position.clamp(0.0, 1.0),
            color,
        }
    }
}

impl From<(f32, Color)> for UGradientStop {
    fn from((position, color): (f32, Color)) -> Self {
        Self::new(position, color)
    }
}

/// Maximum number of color stops supported in a single [`UGradient`].
pub const MAX_GRADIENT_STOPS: usize = 8;

/// Interpolation mode between gradient color stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Reflect)]
#[reflect(Default)]
pub enum UGradientInterpolation {
    /// Smooth linear interpolation between adjacent stops (default).
    #[default]
    Smooth,
    /// Discrete, hard-edged color bands without interpolation (stepped stops).
    Stepped,
}

/// Linear or radial gradient fill with multi-stop color blending.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct UGradient {
    /// Ordered list of color stops (up to 8 stops).
    pub stops: Vec<UGradientStop>,
    /// Shape and orientation of the gradient.
    pub kind: UGradientKind,
    /// Interpolation mode (smooth blending vs sharp, stepped color bands).
    pub interpolation: UGradientInterpolation,
    /// Offset / phase shift along the gradient direction (useful for sweeping/passing animations).
    pub offset: f32,
}

impl Default for UGradient {
    fn default() -> Self {
        Self::horizontal(Color::WHITE, Color::WHITE)
    }
}

impl UGradient {
    /// Creates a linear horizontal gradient (left to right).
    pub fn horizontal(start_color: Color, end_color: Color) -> Self {
        Self {
            stops: vec![
                UGradientStop::new(0.0, start_color),
                UGradientStop::new(1.0, end_color),
            ],
            kind: UGradientKind::Linear { angle: 0.0 },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a linear vertical gradient (top to bottom).
    pub fn vertical(start_color: Color, end_color: Color) -> Self {
        Self {
            stops: vec![
                UGradientStop::new(0.0, start_color),
                UGradientStop::new(1.0, end_color),
            ],
            kind: UGradientKind::Linear {
                angle: core::f32::consts::FRAC_PI_2,
            },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a linear gradient with an explicit angle in radians.
    pub fn linear(start_color: Color, end_color: Color, angle: f32) -> Self {
        Self {
            stops: vec![
                UGradientStop::new(0.0, start_color),
                UGradientStop::new(1.0, end_color),
            ],
            kind: UGradientKind::Linear { angle },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a multi-stop linear gradient along an explicit angle in radians.
    pub fn linear_stops<I, S>(angle: f32, stops: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<UGradientStop>,
    {
        let mut stops_vec: Vec<UGradientStop> = stops
            .into_iter()
            .map(Into::into)
            .take(MAX_GRADIENT_STOPS)
            .collect();
        stops_vec.sort_by(|a, b| a.position.total_cmp(&b.position));
        Self {
            stops: stops_vec,
            kind: UGradientKind::Linear { angle },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a radial gradient centered in the node.
    pub fn radial(start_color: Color, end_color: Color) -> Self {
        Self {
            stops: vec![
                UGradientStop::new(0.0, start_color),
                UGradientStop::new(1.0, end_color),
            ],
            kind: UGradientKind::Radial {
                center: Vec2::splat(0.5),
                radius: 0.5,
            },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a radial gradient with explicit center and radius.
    pub fn radial_custom(start_color: Color, end_color: Color, center: Vec2, radius: f32) -> Self {
        Self {
            stops: vec![
                UGradientStop::new(0.0, start_color),
                UGradientStop::new(1.0, end_color),
            ],
            kind: UGradientKind::Radial { center, radius },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a multi-stop radial gradient centered in the node.
    pub fn radial_stops<I, S>(stops: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<UGradientStop>,
    {
        let mut stops_vec: Vec<UGradientStop> = stops
            .into_iter()
            .map(Into::into)
            .take(MAX_GRADIENT_STOPS)
            .collect();
        stops_vec.sort_by(|a, b| a.position.total_cmp(&b.position));
        Self {
            stops: stops_vec,
            kind: UGradientKind::Radial {
                center: Vec2::splat(0.5),
                radius: 0.5,
            },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a multi-stop radial gradient with custom center and radius.
    pub fn radial_stops_custom<I, S>(center: Vec2, radius: f32, stops: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<UGradientStop>,
    {
        let mut stops_vec: Vec<UGradientStop> = stops
            .into_iter()
            .map(Into::into)
            .take(MAX_GRADIENT_STOPS)
            .collect();
        stops_vec.sort_by(|a, b| a.position.total_cmp(&b.position));
        Self {
            stops: stops_vec,
            kind: UGradientKind::Radial { center, radius },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates an angular / conic sweep gradient centered in the node.
    pub fn angular(start_color: Color, end_color: Color) -> Self {
        Self {
            stops: vec![
                UGradientStop::new(0.0, start_color),
                UGradientStop::new(1.0, end_color),
            ],
            kind: UGradientKind::Angular {
                center: Vec2::splat(0.5),
            },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Creates a multi-stop angular / conic sweep gradient centered in the node.
    pub fn angular_stops<I, S>(stops: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<UGradientStop>,
    {
        let mut stops_vec: Vec<UGradientStop> = stops
            .into_iter()
            .map(Into::into)
            .take(MAX_GRADIENT_STOPS)
            .collect();
        stops_vec.sort_by(|a, b| a.position.total_cmp(&b.position));
        Self {
            stops: stops_vec,
            kind: UGradientKind::Angular {
                center: Vec2::splat(0.5),
            },
            interpolation: UGradientInterpolation::Smooth,
            offset: 0.0,
        }
    }

    /// Alias for [`UGradient::angular`] (conic sweep gradient).
    pub fn conic(start_color: Color, end_color: Color) -> Self {
        Self::angular(start_color, end_color)
    }

    /// Alias for [`UGradient::angular_stops`] (conic sweep gradient with multiple stops).
    pub fn conic_stops<I, S>(stops: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<UGradientStop>,
    {
        Self::angular_stops(stops)
    }

    /// Switches the gradient to stepped interpolation (hard edges, discrete color bands).
    pub fn stepped(mut self) -> Self {
        self.interpolation = UGradientInterpolation::Stepped;
        self
    }

    /// Sets the gradient interpolation mode explicitly.
    pub fn with_interpolation(mut self, interpolation: UGradientInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Sets the offset / phase translation along the gradient direction.
    pub fn with_offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    /// Appends or inserts a color stop, maintaining ascending position order.
    pub fn with_stop(mut self, position: f32, color: Color) -> Self {
        if self.stops.len() < MAX_GRADIENT_STOPS {
            self.stops.push(UGradientStop::new(position, color));
            self.stops.sort_by(|a, b| a.position.total_cmp(&b.position));
        }
        self
    }

    /// Returns the color of the first stop, or white if empty.
    pub fn start_color(&self) -> Color {
        self.stops.first().map_or(Color::WHITE, |s| s.color)
    }

    /// Returns the color of the last stop, or white if empty.
    pub fn end_color(&self) -> Color {
        self.stops.last().map_or(Color::WHITE, |s| s.color)
    }
}

/// Outer glow, drop shadow, or inner holographic glow for a [`crate::layout::univis_node::UNode`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct UShadow {
    /// Color and opacity of the shadow or glow.
    pub color: Color,
    /// Offset of the shadow in logical pixels (X, Y). Set to Vec2::ZERO for a symmetrical glow.
    pub offset: Vec2,
    /// Blur softness radius in logical pixels.
    pub blur: f32,
    /// Spread distance in logical pixels (expands the base shape before blur).
    pub spread: f32,
    /// If true, the glow renders as an inner glow from the border toward the center.
    pub is_inner: bool,
}

impl Default for UShadow {
    fn default() -> Self {
        Self {
            color: Color::srgba(0.0, 0.9, 1.0, 0.4),
            offset: Vec2::ZERO,
            blur: 12.0,
            spread: 0.0,
            is_inner: false,
        }
    }
}

impl UShadow {
    /// Creates a symmetrical outer glow (e.g. cyber neon glow).
    pub fn glow(color: Color, blur: f32) -> Self {
        Self {
            color,
            offset: Vec2::ZERO,
            blur,
            spread: 0.0,
            is_inner: false,
        }
    }

    /// Creates an inner holographic glow radiating inward from the border.
    pub fn inner(color: Color, blur: f32) -> Self {
        Self {
            color,
            offset: Vec2::ZERO,
            blur,
            spread: 0.0,
            is_inner: true,
        }
    }

    /// Creates a directional drop shadow.
    pub fn drop(color: Color, offset: Vec2, blur: f32) -> Self {
        Self {
            color,
            offset,
            blur,
            spread: 0.0,
            is_inner: false,
        }
    }
}

/// Holographic inner edge glow radiating inward from the border.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct UInnerGlow {
    /// Color and opacity of the inner glow.
    pub color: Color,
    /// Blur softness radius in logical pixels.
    pub blur: f32,
    /// Spread distance in logical pixels.
    pub spread: f32,
}

impl Default for UInnerGlow {
    fn default() -> Self {
        Self {
            color: Color::srgba(0.0, 0.9, 1.0, 0.5),
            blur: 10.0,
            spread: 0.0,
        }
    }
}

impl UInnerGlow {
    /// Creates a holographic inner edge glow.
    pub fn new(color: Color, blur: f32) -> Self {
        Self {
            color,
            blur,
            spread: 0.0,
        }
    }

    /// Creates an inner glow with explicit spread.
    pub fn with_spread(color: Color, blur: f32, spread: f32) -> Self {
        Self {
            color,
            blur,
            spread,
        }
    }
}

/// Source definition for a custom node background shader (`UShader`).
#[derive(Reflect, Clone, Debug, PartialEq, Eq, Hash)]
pub enum UShaderSource {
    /// Inline WGSL code snippet injected into the host `UNode` SDF shader.
    ///
    /// You can either provide the body of `sample_custom_shader` directly (with
    /// `uv: vec2<f32>`, `size: vec2<f32>`, `time: f32`, `params: array<vec4<f32>, 4>` in scope),
    /// or define helper functions followed by a full
    /// `fn sample_custom_shader(uv: vec2<f32>, size: vec2<f32>, time: f32, params: array<vec4<f32>, 4>) -> vec4<f32>`.
    Wgsl(String),
    /// Path to a custom fragment shader asset loaded via Bevy's `AssetServer`.
    AssetPath(String),
    /// Pre-loaded Bevy `Handle<Shader>` overriding the fragment shader.
    Handle(Handle<Shader>),
}

impl Default for UShaderSource {
    fn default() -> Self {
        Self::Wgsl("return params[0];".to_string())
    }
}

impl Unpin for UShaderSource {}

/// Compositing blend mode determining how [`UShader`] combines with the node's base fill
/// (`background_color`, `UGradient`, and `UImage`).
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UShaderBlendMode {
    /// Completely replaces the node's base background fill while preserving SDF borders,
    /// inner glows, outer shadows, and rounded clipping.
    #[default]
    Replace,
    /// Alpha-blends the custom shader over the node's existing background or gradient.
    AlphaBlend,
    /// Multiplies the custom shader color with the node's base fill.
    Multiply,
    /// Additively blends the custom shader color (ideal for light beams, flares, and holograms).
    Add,
}

impl UShaderBlendMode {
    /// Numeric encoding passed to the shader uniform `custom_meta.y`.
    pub fn as_f32(self) -> f32 {
        match self {
            Self::Replace => 0.0,
            Self::AlphaBlend => 1.0,
            Self::Multiply => 2.0,
            Self::Add => 3.0,
        }
    }
}

/// Injects or replaces a `UNode`'s background with a custom GPU fragment shader while
/// preserving all Univis SDF geometry (rounded/cut corners, borders, inner glow, shadows, and clipping).
///
/// # Example
/// ```rust,no_run
/// use bevy::prelude::*;
/// use luma_engine::prelude::*;
///
/// let shader = UShader::from_wgsl("
///     let wave = 0.5 + 0.5 * sin(uv.x * 8.0 + time * 2.0);
///     return mix(params[0], params[1], wave);
/// ")
/// .with_colors([Color::srgb(0.05, 0.2, 0.95), Color::srgb(0.95, 0.1, 0.5)]);
/// ```
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct UShader {
    /// Shader code or asset source.
    pub source: UShaderSource,
    /// Four general-purpose 16-byte `Vec4` uniform slots (`params[0]..params[3]` in WGSL).
    /// Colors passed via [`Self::with_color`] / [`Self::with_colors`] are converted to linear RGBA here.
    pub params: [Vec4; 4],
    /// How the shader output combines with the node's underlying color/gradient.
    pub blend_mode: UShaderBlendMode,
    /// Overall opacity multiplier (`0.0..=1.0`) applied to the shader output.
    pub opacity: f32,
    /// Whether `time` should automatically advance each frame without dirtying UI layout.
    pub animate: bool,
    /// Multiplier applied to elapsed time when `animate` is true.
    pub speed: f32,
    /// Static phase/time offset added to the shader's `time` input.
    pub time_offset: f32,
}

impl Unpin for UShader {}

impl Default for UShader {
    fn default() -> Self {
        Self {
            source: UShaderSource::default(),
            params: [
                Vec4::new(0.1, 0.4, 1.0, 1.0),
                Vec4::new(0.9, 0.1, 0.5, 1.0),
                Vec4::new(0.0, 0.9, 0.8, 1.0),
                Vec4::new(0.02, 0.02, 0.05, 1.0),
            ],
            blend_mode: UShaderBlendMode::Replace,
            opacity: 1.0,
            animate: true,
            speed: 1.0,
            time_offset: 0.0,
        }
    }
}

impl UShader {
    /// Creates a custom shader from an inline WGSL snippet.
    ///
    /// Built-in variables available in scope:
    /// - `uv: vec2<f32>` (`0.0..1.0` normalized coordinates across the node body)
    /// - `size: vec2<f32>` (logical pixel size of the node)
    /// - `time: f32` (animated time in seconds)
    /// - `params: array<vec4<f32>, 4>` (custom uniform vectors/colors)
    /// - Built-in noise functions: `luma_hash21(p)`, `luma_noise(p)`, `luma_fbm(p)`
    pub fn from_wgsl(wgsl: impl Into<String>) -> Self {
        Self {
            source: UShaderSource::Wgsl(wgsl.into()),
            ..Default::default()
        }
    }

    /// Creates a custom shader loaded from an asset path via `AssetServer`.
    pub fn from_asset(path: impl Into<String>) -> Self {
        Self {
            source: UShaderSource::AssetPath(path.into()),
            ..Default::default()
        }
    }

    /// Creates a custom shader from an existing `Handle<Shader>`.
    pub fn from_handle(handle: Handle<Shader>) -> Self {
        Self {
            source: UShaderSource::Handle(handle),
            ..Default::default()
        }
    }

    /// Sets one of the 4 uniform parameter vectors (`index` in `0..4`).
    pub fn with_param(mut self, index: usize, value: Vec4) -> Self {
        if index < 4 {
            self.params[index] = value;
        }
        self
    }

    /// Sets all 4 uniform parameter vectors at once.
    pub fn with_params(mut self, params: [Vec4; 4]) -> Self {
        self.params = params;
        self
    }

    /// Sets one of the 4 parameter slots from a Bevy `Color` (converted to `LinearRgba`).
    pub fn with_color(mut self, index: usize, color: Color) -> Self {
        if index < 4 {
            self.params[index] = LinearRgba::from(color).to_vec4();
        }
        self
    }

    /// Populates up to 4 parameter slots from an iterator/array of Bevy `Color`s.
    pub fn with_colors(mut self, colors: impl IntoIterator<Item = Color>) -> Self {
        for (i, c) in colors.into_iter().take(4).enumerate() {
            self.params[i] = LinearRgba::from(c).to_vec4();
        }
        self
    }

    /// Sets the compositing blend mode (`Replace`, `AlphaBlend`, `Multiply`, `Add`).
    pub fn with_blend_mode(mut self, mode: UShaderBlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Sets the opacity multiplier (`0.0..=1.0`).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Enables or disables automatic frame-by-frame `time` animation.
    pub fn with_animation(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }

    /// Sets the animation speed multiplier.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Sets a static phase/time offset (useful for desynchronizing a grid of shader cards).
    pub fn with_time_offset(mut self, offset: f32) -> Self {
        self.time_offset = offset;
        self
    }

    // -------------------------------------------------------------------------
    // Built-in Procedural GPU Presets
    // -------------------------------------------------------------------------

    /// Organic 4-point animated Mesh / Fluid Gradient on a single node using domain-warped FBM.
    pub fn mesh_gradient(c0: Color, c1: Color, c2: Color, c3: Color) -> Self {
        Self::from_wgsl(
            r#"
            let t = time * 0.45;
            let warp = vec2<f32>(
                luma_fbm(uv * 2.4 + vec2<f32>(t * 0.7, -t * 0.5)),
                luma_fbm(uv * 2.4 + vec2<f32>(-t * 0.6, t * 0.8) + vec2<f32>(4.2, 1.7))
            ) - 0.5;
            let p = clamp(uv + warp * 0.42, vec2<f32>(0.0), vec2<f32>(1.0));

            let p0 = vec2<f32>(0.18 + 0.15 * sin(t), 0.22 + 0.15 * cos(t * 0.8));
            let p1 = vec2<f32>(0.82 + 0.14 * cos(t * 0.9), 0.25 + 0.14 * sin(t * 1.1));
            let p2 = vec2<f32>(0.24 + 0.16 * cos(t * 0.7), 0.80 + 0.14 * sin(t * 0.9));
            let p3 = vec2<f32>(0.78 + 0.15 * sin(t * 1.2), 0.76 + 0.15 * cos(t * 0.7));

            let w0 = 1.0 / (pow( distance(p, p0), 2.1 ) + 0.045);
            let w1 = 1.0 / (pow( distance(p, p1), 2.1 ) + 0.045);
            let w2 = 1.0 / (pow( distance(p, p2), 2.1 ) + 0.045);
            let w3 = 1.0 / (pow( distance(p, p3), 2.1 ) + 0.045);
            let sum = w0 + w1 + w2 + w3;

            let col = (params[0] * w0 + params[1] * w1 + params[2] * w2 + params[3] * w3) / sum;
            return vec4<f32>(col.rgb, 1.0);
            "#,
        )
        .with_colors([c0, c1, c2, c3])
    }

    /// Intersecting diagonal prism light beams with fine optical scanlines on a pitch-black void.
    pub fn aurora_beams(primary_beam: Color, secondary_beam: Color, core_flare: Color) -> Self {
        Self::from_wgsl(
            r#"
            let t = time * 0.55;
            // Diagonal coordinate space for primary cobalt/cyan beam
            let d1 = (uv.x * 0.82 - uv.y * 0.57) - 0.12 + 0.05 * sin(uv.y * 4.0 + t);
            let beam1 = exp(-pow(d1 * 4.2, 2.0)) * smoothstep(1.05, 0.05, uv.y * 0.75 + uv.x * 0.25);

            // Intersecting crimson/orange secondary beam rising from bottom-right
            let d2 = (uv.x * 0.65 + uv.y * 0.76) - 1.02 + 0.04 * cos(uv.x * 5.0 - t * 1.2);
            let beam2 = exp(-pow(d2 * 4.6, 2.0)) * smoothstep(0.0, 0.85, uv.x) * smoothstep(0.05, 0.95, uv.y);

            // Micro-ribbed optical refraction lines inside the beam intersection
            let rib_coord = (uv.x * size.x * 0.36 + uv.y * size.y * 0.14) + t * 6.0;
            let ribs = 0.82 + 0.18 * sin(rib_coord);

            let overlap = beam1 * beam2 * 2.4;
            var rgb = params[0].rgb * (beam1 * ribs * 1.25)
                    + params[1].rgb * (beam2 * 1.35)
                    + params[2].rgb * overlap;
            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#,
        )
        .with_colors([
            primary_beam,
            secondary_beam,
            core_flare,
            Color::srgb(0.01, 0.01, 0.02),
        ])
    }

    /// Pitch-black top void (`0.0..0.46`) with an atmospheric rising multi-color horizon glow at the bottom.
    pub fn dark_horizon(left_glow: Color, center_flare: Color, right_glow: Color) -> Self {
        Self::from_wgsl(
            r#"
            let t = time * 0.4;
            let wave = 0.035 * sin(uv.x * 5.5 + t) + 0.02 * cos(uv.x * 9.0 - t * 1.3);
            let rise = smoothstep(0.42 + wave, 1.0, uv.y);
            let horizon_band = pow(rise, 1.65);

            let dome_l = exp(-pow((uv.x - 0.22) * 2.3, 2.0)) * smoothstep(0.38, 0.98, uv.y);
            let dome_c = exp(-pow((uv.x - 0.52 - 0.06 * sin(t * 0.7)) * 2.5, 2.0)) * pow(smoothstep(0.48, 1.0, uv.y), 1.4);
            let dome_r = exp(-pow((uv.x - 0.82) * 2.2, 2.0)) * smoothstep(0.40, 0.98, uv.y);

            let horiz_mix = mix(params[0].rgb, params[2].rgb, smoothstep(0.15, 0.85, uv.x));
            var rgb = horiz_mix * (horizon_band * 0.95 + (dome_l + dome_r) * 0.45)
                    + params[1].rgb * (dome_c * 1.15);

            // Pitch-black top guarantee
            let top_mask = smoothstep(0.36, 0.56, uv.y);
            rgb = rgb * top_mask;
            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#,
        )
        .with_colors([
            left_glow,
            center_flare,
            right_glow,
            Color::srgb(0.01, 0.01, 0.02),
        ])
    }

    /// High-energy domain-warped FBM plasma nebula with glowing filaments.
    pub fn plasma_energy(core_color: Color, filament_color: Color, deep_bg: Color) -> Self {
        Self::from_wgsl(
            r#"
            let t = time * 0.5;
            let p = uv * 3.2;
            let q = vec2<f32>(
                luma_fbm(p + vec2<f32>(0.0, t * 0.6)),
                luma_fbm(p + vec2<f32>(5.2, 1.3 - t * 0.5))
            );
            let r = vec2<f32>(
                luma_fbm(p + 3.8 * q + vec2<f32>(1.7, 9.2) + 0.35 * t),
                luma_fbm(p + 3.8 * q + vec2<f32>(8.3, 2.8) - 0.30 * t)
            );
            let f = luma_fbm(p + 3.5 * r);
            let filament = pow(clamp(f * 1.35, 0.0, 1.0), 2.2);
            let pulse = clamp(length(q), 0.0, 1.0);

            var rgb = mix(params[2].rgb, params[0].rgb, clamp(f * f * 2.2, 0.0, 1.0));
            rgb = mix(rgb, params[1].rgb, filament * 0.85 + pulse * 0.25);
            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#,
        )
        .with_colors([
            core_color,
            filament_color,
            deep_bg,
            Color::WHITE,
        ])
    }

    /// Sci-fi holographic cyber grid with animated radar sweep and data pulses.
    pub fn cyber_matrix(grid_color: Color, pulse_color: Color, bg_color: Color) -> Self {
        Self::from_wgsl(
            r#"
            let aspect = size.x / max(size.y, 1.0);
            let scaled = vec2<f32>(uv.x * aspect, uv.y) * 10.0;
            let cell = fract(scaled);
            let cell_id = floor(scaled);

            let line_x = smoothstep(0.06, 0.0, min(cell.x, 1.0 - cell.x));
            let line_y = smoothstep(0.06, 0.0, min(cell.y, 1.0 - cell.y));
            let grid = max(line_x, line_y) * 0.42;

            // Animated horizontal scan wave & glowing data cells
            let scan = exp(-pow((uv.y - fract(time * 0.35)) * 9.0, 2.0));
            let rnd = luma_hash21(cell_id);
            let active_cell = step(0.82, sin(rnd * 32.0 + time * (1.5 + rnd * 2.0)) * 0.5 + 0.5);
            let inner_pad = step(0.18, cell.x) * step(cell.x, 0.82) * step(0.18, cell.y) * step(cell.y, 0.82);

            var rgb = params[2].rgb;
            rgb = rgb + params[0].rgb * (grid * (0.65 + 0.85 * scan));
            rgb = rgb + params[1].rgb * (active_cell * inner_pad * 0.55 + scan * 0.35);
            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#,
        )
        .with_colors([grid_color, pulse_color, bg_color, Color::WHITE])
    }

    /// Iridescent liquid metallic waves with specular highlights.
    pub fn liquid_chrome(tint_a: Color, tint_b: Color, specular: Color) -> Self {
        Self::from_wgsl(
            r#"
            let t = time * 0.65;
            var p = (uv - 0.5) * 3.6;
            for (var i = 1; i < 5; i = i + 1) {
                let fi = f32(i);
                p.x = p.x + 0.42 / fi * sin(fi * 2.5 * p.y + t + 0.3 * fi);
                p.y = p.y + 0.42 / fi * cos(fi * 2.5 * p.x - t + 0.5 * fi);
            }
            let ridge = 0.5 + 0.5 * sin(p.x * 2.8 + p.y * 2.8);
            let spec = pow(ridge, 5.0);
            let base = mix(params[0].rgb, params[1].rgb, 0.5 + 0.5 * cos(p.x * 1.7 - p.y * 1.4));
            let c_dist = luma_cursor_dist(uv, size);
            let hover_glow = exp(-c_dist * 4.5) * luma_cursor_hover() * 0.45;
            let rgb = base * (0.35 + 0.65 * ridge) + params[2].rgb * (spec * 0.75 + hover_glow);
            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#,
        )
        .with_colors([tint_a, tint_b, specular, Color::BLACK])
    }

    /// Mouse-reactive magnetic spotlight that illuminates a hidden cybernetic micro-grid
    /// under the cursor and emits an expanding shockwave ring when clicked.
    pub fn interactive_spotlight(glow_color: Color, ring_color: Color, bg_color: Color) -> Self {
        Self::from_wgsl(
            r#"
            let aspect = size.x / max(size.y, 1.0);
            let cursor = luma_cursor_uv();
            let hover = luma_cursor_hover();
            let press = luma_cursor_press();

            let delta = (uv - cursor) * vec2<f32>(aspect, 1.0);
            let d = length(delta);

            // Hidden precision hex/cyber grid revealed by cursor spotlight
            let grid_uv = vec2<f32>(uv.x * aspect, uv.y) * 16.0;
            let cell = fract(grid_uv);
            let line = max(
                smoothstep(0.07, 0.0, min(cell.x, 1.0 - cell.x)),
                smoothstep(0.07, 0.0, min(cell.y, 1.0 - cell.y))
            );
            let dots = smoothstep(0.14, 0.04, length(cell - 0.5));

            // Magnetic spotlight core + hover boost + click shockwave ripples
            let spot_radius = 0.38 + 0.14 * hover + 0.10 * press;
            let spot = exp(-pow(d / spot_radius, 2.0) * 2.4);
            let core = exp(-d * (9.0 - 3.0 * press)) * (0.55 + 0.45 * hover);

            let ring_wave = sin(d * 28.0 - time * 5.5) * 0.5 + 0.5;
            let target_ring = exp(-pow((d - (0.14 + 0.12 * press)) * 16.0, 2.0)) * (0.35 + 0.65 * hover);
            let click_burst = pow(ring_wave, 4.0) * exp(-d * 3.0) * press * 1.4;

            var rgb = params[2].rgb;
            rgb = rgb + params[0].rgb * (spot * (0.45 + 0.75 * line) + core * 0.85);
            rgb = rgb + params[1].rgb * (target_ring + dots * spot * 0.65 + click_burst);
            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#,
        )
        .with_colors([glow_color, ring_color, bg_color, Color::WHITE])
    }

    /// Mouse-reactive fluid plasma vortex that bends and ripples around the live pointer
    /// and erupts with a luminous wave burst when clicked.
    pub fn interactive_fluid_ripple(color_a: Color, color_b: Color, highlight: Color) -> Self {
        Self::from_wgsl(
            r#"
            let aspect = size.x / max(size.y, 1.0);
            let cursor = luma_cursor_uv();
            let hover = luma_cursor_hover();
            let press = luma_cursor_press();

            let delta = (uv - cursor) * vec2<f32>(aspect, 1.0);
            let d = length(delta);

            // Gravitational swirl + ripple distortion around mouse cursor
            let pull = exp(-d * 3.5) * (0.25 + 0.35 * hover + 0.40 * press);
            let angle = atan2(delta.y, delta.x) + pull * 3.2;
            let ripple = sin(d * 26.0 - time * 4.5) * 0.025 * (0.4 + 0.6 * hover + 1.2 * press) * exp(-d * 2.2);
            let warped_uv = uv + vec2<f32>(cos(angle), sin(angle)) * (pull * 0.12 + ripple);

            let t = time * 0.55;
            let n1 = luma_fbm(warped_uv * 3.0 + vec2<f32>(t * 0.6, -t * 0.4));
            let n2 = luma_fbm(warped_uv * 3.4 - vec2<f32>(t * 0.5, t * 0.7) + n1 * 1.8);

            let cursor_glow = exp(-d * (4.2 - 1.5 * press)) * (0.45 + 0.55 * hover + 0.75 * press);
            var rgb = mix(params[0].rgb, params[1].rgb, smoothstep(0.2, 0.8, n2));
            rgb = rgb + params[2].rgb * (pow(n2, 3.0) * 0.65 + cursor_glow * 0.90);
            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#,
        )
        .with_colors([color_a, color_b, highlight, Color::BLACK])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_constructors_set_expected_parameters() {
        let h = UGradient::horizontal(Color::WHITE, Color::BLACK);
        assert_eq!(h.kind, UGradientKind::Linear { angle: 0.0 });

        let v = UGradient::vertical(Color::WHITE, Color::BLACK);
        assert_eq!(
            v.kind,
            UGradientKind::Linear {
                angle: core::f32::consts::FRAC_PI_2
            }
        );

        let r = UGradient::radial(Color::WHITE, Color::BLACK);
        assert_eq!(
            r.kind,
            UGradientKind::Radial {
                center: Vec2::splat(0.5),
                radius: 0.5
            }
        );
        assert_eq!(r.stops.len(), 2);
        assert_eq!(r.start_color(), Color::WHITE);
        assert_eq!(r.end_color(), Color::BLACK);
    }

    #[test]
    fn multi_stop_gradient_sorts_and_clamps() {
        let grad = UGradient::linear_stops(
            1.57,
            vec![
                (1.0, Color::srgb(1.0, 0.0, 0.0)),
                (0.0, Color::srgb(0.0, 0.0, 1.0)),
                (0.5, Color::srgb(0.0, 1.0, 0.0)),
            ],
        );
        assert_eq!(grad.stops.len(), 3);
        assert_eq!(grad.stops[0].position, 0.0);
        assert_eq!(grad.stops[1].position, 0.5);
        assert_eq!(grad.stops[2].position, 1.0);
        assert_eq!(grad.start_color(), Color::srgb(0.0, 0.0, 1.0));
        assert_eq!(grad.end_color(), Color::srgb(1.0, 0.0, 0.0));

        let builder = UGradient::linear(Color::BLACK, Color::WHITE, 0.0)
            .with_stop(0.3, Color::srgb(0.5, 0.5, 0.5));
        assert_eq!(builder.stops.len(), 3);
        assert_eq!(builder.stops[1].position, 0.3);
    }

    #[test]
    fn gradient_interpolation_and_offset_modes() {
        let smooth = UGradient::linear(Color::BLACK, Color::WHITE, 0.0);
        assert_eq!(smooth.interpolation, UGradientInterpolation::Smooth);
        assert_eq!(smooth.offset, 0.0);

        let stepped = smooth.stepped();
        assert_eq!(stepped.interpolation, UGradientInterpolation::Stepped);

        let with_off = stepped.with_offset(0.45);
        assert_eq!(with_off.offset, 0.45);

        let custom = with_off.with_interpolation(UGradientInterpolation::Smooth);
        assert_eq!(custom.interpolation, UGradientInterpolation::Smooth);
    }

    #[test]
    fn shadow_constructors_set_expected_parameters() {
        let g = UShadow::glow(Color::srgb(0.0, 1.0, 0.5), 14.0);
        assert_eq!(g.offset, Vec2::ZERO);
        assert_eq!(g.blur, 14.0);
        assert!(!g.is_inner);

        let i = UShadow::inner(Color::srgb(1.0, 0.0, 0.5), 8.0);
        assert_eq!(i.blur, 8.0);
        assert!(i.is_inner);

        let d = UShadow::drop(Color::BLACK, Vec2::new(4.0, -4.0), 10.0);
        assert_eq!(d.offset, Vec2::new(4.0, -4.0));
        assert_eq!(d.blur, 10.0);
        assert!(!d.is_inner);

        let ig = UInnerGlow::new(Color::srgb(0.0, 1.0, 1.0), 12.0);
        assert_eq!(ig.blur, 12.0);
        assert_eq!(ig.spread, 0.0);
    }
}
