//! First-class 3D lighting emitters, SDF occluders, optical filters, modulators,
//! light sensors, and light-dependent availability gating for UI nodes and typography.

use std::f32::consts::{FRAC_PI_4, PI, TAU};

use bevy::camera::visibility::VisibilitySystems;
use bevy::ecs::relationship::Relationship;
use bevy::light::{GlobalAmbientLight, NotShadowCaster, NotShadowReceiver, RectLight};
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::layout::components::{CachedUiContext, UI3d};
use crate::layout::layout_system::{ResolvedRootUi, UiSpace};
use crate::layout::pbr::{
    InheritedPbrMarker, InheritedPbrSurfaceMarker, UPbr, UPbrInherit, UPbrKeepUnlit,
    UPbrPropagate, UPbrSurface,
};
use crate::layout::query::ComputedSize;
use crate::layout::text::TextChildMarker;
use crate::layout::univis_node::UNode;
use crate::prelude::UShapeMode;
use crate::schedule::{UiSettlementSchedule, UnivisPostUpdateSet};

/// Lighting flag bit: double-sided front/back normal flipping.
pub(crate) const LIGHTING_FLAG_DOUBLE_SIDED: u32 = 1 << 0;
/// Lighting flag bit: receives real-time shadows from scene lights.
pub(crate) const LIGHTING_FLAG_RECEIVE_SHADOWS: u32 = 1 << 1;
/// Lighting flag bit: casts SDF silhouette shadows in prepass/shadow passes.
pub(crate) const LIGHTING_FLAG_CAST_SHADOWS: u32 = 1 << 2;

/// Resolves surface reflectance, lighting flag bitmask, `(bevel_width, bevel_strength, alpha_cutoff, 0.0)`,
/// and `AlphaMode` consistently for both `UNode` and `UText` 3D materials.
pub(crate) fn resolve_surface_lighting(
    surface_opt: Option<&UPbrSurface>,
    occluder_opt: Option<&ULightOccluder>,
) -> (f32, u32, Vec4, AlphaMode) {
    let surf = surface_opt.cloned().unwrap_or_default();
    let cast_shadows = surf.cast_shadows || occluder_opt.is_some_and(|o| o.enabled && o.cast_shadows);
    let mut flags = 0u32;
    if surf.double_sided {
        flags |= LIGHTING_FLAG_DOUBLE_SIDED;
    }
    if surf.receive_shadows {
        flags |= LIGHTING_FLAG_RECEIVE_SHADOWS;
    }
    if cast_shadows {
        flags |= LIGHTING_FLAG_CAST_SHADOWS;
    }
    let alpha_mode = surf.alpha_mode.unwrap_or(AlphaMode::Premultiplied);
    let alpha_cutoff = match alpha_mode {
        AlphaMode::Mask(c) => c.clamp(0.0, 1.0),
        _ => surf.alpha_cutoff.clamp(0.0, 1.0),
    };
    let bevel_params = Vec4::new(
        surf.bevel_width.max(0.0),
        surf.bevel_strength.max(0.0),
        alpha_cutoff,
        0.0,
    );
    (surf.reflectance.clamp(0.0, 1.0), flags, bevel_params, alpha_mode)
}

/// Shape type emitted by [`ULightSource`].
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum ULightSourceKind {
    /// True rectangular area light matching the UI element's physical dimensions.
    Rect {
        /// Optional explicit size `(width, height)` in world meters; `None` auto-sizes from [`ComputedSize`].
        size_override: Option<Vec2>,
    },
    /// Omnidirectional spherical bulb centered near the UI element.
    Point {
        /// Physical bulb radius used for soft highlights.
        radius: f32,
    },
    /// Directional cone spotlight aimed out of the UI element's surface normal.
    Spot {
        /// Physical emitter radius.
        radius: f32,
        /// Cone angle with full intensity, in radians.
        inner_angle: f32,
        /// Outer cutoff cone angle, in radians.
        outer_angle: f32,
    },
}

impl Default for ULightSourceKind {
    fn default() -> Self {
        Self::Rect {
            size_override: None,
        }
    }
}

/// Turns any `World3d` [`UNode`] or [`UText`](crate::layout::text::UText) into an actual physical light emitter.
///
/// Spawns and synchronizes a child Bevy light (`RectLight`, `PointLight`, or `SpotLight`),
/// optionally keeping the host node or text's emissive [`UPbr`] glow in sync so turning
/// a lamp on or off toggles both scene illumination and self-emission seamlessly.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[require(Transform, Visibility)]
#[reflect(Component, Default)]
pub struct ULightSource {
    /// Turns this light source on or off.
    pub enabled: bool,
    /// Underlying Bevy light archetype (`Rect`, `Point`, or `Spot`).
    pub kind: ULightSourceKind,
    /// Light chromatic color.
    pub color: Color,
    /// Luminous power in lumens.
    pub intensity: f32,
    /// Effective illumination range in world meters.
    pub range: f32,
    /// Enables hardware shadow maps (`Point` and `Spot` lights).
    pub shadow_maps_enabled: bool,
    /// Offset from the host entity center in local UI space.
    pub local_offset: Vec3,
    /// `+1.0` shines out of the front `+Z` face of UI elements; `-1.0` shines out of `-Z`.
    pub direction_sign: f32,
    /// Synchronizes HDR emissive [`UPbr`] on the host [`UNode`] or [`UText`](crate::layout::text::UText).
    pub sync_emissive: bool,
    /// HDR multiplier applied when generating [`UPbr::emissive`].
    pub emissive_strength: f32,
    /// Normalized dimmer multiplier (`>= 0.0`) scaled onto intensity and emissive glow.
    pub dimmer: f32,
}

impl Default for ULightSource {
    fn default() -> Self {
        Self {
            enabled: true,
            kind: ULightSourceKind::default(),
            color: Color::WHITE,
            intensity: 12_000.0,
            range: 14.0,
            shadow_maps_enabled: false,
            local_offset: Vec3::new(0.0, 0.0, 0.04),
            direction_sign: 1.0,
            sync_emissive: true,
            emissive_strength: 3.5,
            dimmer: 1.0,
        }
    }
}

impl ULightSource {
    /// Turns any UI panel or `UText` into a physical rectangular area lamp (`RectLight`).
    pub fn lamp(color: Color, intensity: f32) -> Self {
        Self {
            kind: ULightSourceKind::Rect {
                size_override: None,
            },
            color,
            intensity,
            ..default()
        }
    }

    /// Creates an omnidirectional point bulb (`PointLight`) centered on the UI element.
    pub fn bulb(color: Color, intensity: f32, range: f32) -> Self {
        Self {
            kind: ULightSourceKind::Point { radius: 0.08 },
            color,
            intensity,
            range,
            ..default()
        }
    }

    /// Creates a focused directional spotlight (`SpotLight`) projected from the UI face.
    pub fn spotlight(color: Color, intensity: f32, range: f32, outer_angle: f32) -> Self {
        let clamped_outer = outer_angle.clamp(0.05, PI * 0.49);
        Self {
            kind: ULightSourceKind::Spot {
                radius: 0.06,
                inner_angle: clamped_outer * 0.55,
                outer_angle: clamped_outer,
            },
            color,
            intensity,
            range,
            ..default()
        }
    }

    /// Sets whether this light source casts real-time shadow maps (`Point` / `Spot`).
    pub fn with_shadows(mut self, enabled: bool) -> Self {
        self.shadow_maps_enabled = enabled;
        self
    }

    /// Sets illumination cutoff range in meters.
    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range.max(0.05);
        self
    }

    /// Sets local offset in the host's coordinate space.
    pub fn with_offset(mut self, offset: Vec3) -> Self {
        self.local_offset = offset;
        self
    }

    /// Sets HDR surface emissive boost (or disables `sync_emissive` when `<= 0.0`).
    pub fn with_emissive(mut self, strength: f32) -> Self {
        if strength <= 0.0 {
            self.sync_emissive = false;
            self.emissive_strength = 0.0;
        } else {
            self.sync_emissive = true;
            self.emissive_strength = strength;
        }
        self
    }

    /// Sets initial dimmer multiplier (`0.0..=1.0+`).
    pub fn with_dimmer(mut self, dimmer: f32) -> Self {
        self.dimmer = dimmer.max(0.0);
        self
    }

    /// Sets whether this lamp starts enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Points emission out of the back `-Z` face instead of the front `+Z` face.
    pub fn facing_back(mut self) -> Self {
        self.direction_sign = -1.0;
        self.local_offset.z = -self.local_offset.z.abs();
        self
    }
}

/// Marker placed on child Bevy light entities managed automatically by [`ULightSource`].
#[derive(Component, Debug)]
pub struct ULightSourceChild {
    /// Host entity carrying [`ULightSource`].
    pub source_entity: Entity,
}

#[derive(Component, Debug, Default)]
pub(crate) struct AutoEmissivePbrMarker;

/// Marks a `World3d` [`UNode`] or [`UText`](crate::layout::text::UText) as a physical light blocker/occluder.
///
/// Enables pixel-exact SDF silhouette shadow casting in Bevy light shadow maps AND
/// blocks rays reaching [`ULightSensor`] / [`URequiresLight`] components.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
#[component(
    on_insert = crate::layout::render::packet::on_pbr_insert,
    on_remove = crate::layout::render::packet::on_occluder_remove
)]
pub struct ULightOccluder {
    /// Enables or disables occlusion without removing the component.
    pub enabled: bool,
    /// Participates in GPU shadow maps using exact SDF node/glyph silhouettes.
    pub cast_shadows: bool,
    /// Blocks incident irradiance rays evaluated by [`ULightSensor`].
    pub block_sensors: bool,
    /// Fraction of light transmitted through this blocker (`0.0` = fully opaque blocker).
    pub transmittance: f32,
}

impl Default for ULightOccluder {
    fn default() -> Self {
        Self {
            enabled: true,
            cast_shadows: true,
            block_sensors: true,
            transmittance: 0.0,
        }
    }
}

impl ULightOccluder {
    /// Creates a fully opaque shadow-casting and sensor-blocking UI occluder.
    pub fn solid() -> Self {
        Self::default()
    }

    /// Creates a partially translucent blocker allowing a fraction of light through.
    pub fn translucent(transmittance: f32) -> Self {
        Self {
            transmittance: transmittance.clamp(0.0, 1.0),
            ..default()
        }
    }

    /// Controls GPU shadow-map casting (`NotShadowCaster` management).
    pub fn with_cast_shadows(mut self, cast: bool) -> Self {
        self.cast_shadows = cast;
        self
    }

    /// Controls analytical ray blocking against [`ULightSensor`].
    pub fn with_block_sensors(mut self, block: bool) -> Self {
        self.block_sensors = block;
        self
    }
}

/// Optical color filter / light-changer component (like translucent stained glass or theatrical gels).
///
/// Can be attached:
/// - **Directly on a light or [`ULightSource`]** to filter its emitted color/intensity non-destructively.
/// - **On a `World3d` [`UNode`]** placed in front of a light beam (`spatial_beam_filtering: true`)
///   so sliding the UI card across a `SpotLight`, `RectLight`, or `DirectionalLight` beam physically
///   changes the light color and attenuation downstream and restores cleanly when moved away.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct ULightFilter {
    /// Enables or disables this optical filter.
    pub enabled: bool,
    /// Multiplicative optical gel tint color.
    pub tint: Color,
    /// Energy transmission factor (`0.0..=1.0`, `1.0` preserves full luminance of passed wavelengths).
    pub transmittance: f32,
    /// Additional gain/dimming multiplier.
    pub intensity_scale: f32,
    /// When attached to a UI node, intercepts matching light beams intersecting the node's SDF shape.
    pub spatial_beam_filtering: bool,
    /// Optional explicit target light entity (`None` checks spatial beam intersection).
    pub target_light: Option<Entity>,
    /// Maximum distance in front of directional/spot lights to probe for beam interception.
    pub probe_distance: f32,
}

impl Default for ULightFilter {
    fn default() -> Self {
        Self {
            enabled: true,
            tint: Color::srgb(0.3, 0.85, 1.0),
            transmittance: 0.92,
            intensity_scale: 1.0,
            spatial_beam_filtering: true,
            target_light: None,
            probe_distance: 12.0,
        }
    }
}

impl ULightFilter {
    /// Creates an optical filter with explicit tint color and transmittance.
    pub fn new(tint: Color, transmittance: f32) -> Self {
        Self {
            tint,
            transmittance: transmittance.clamp(0.0, 1.0),
            ..default()
        }
    }

    /// Creates a saturated theater gel optical filter (`transmittance = 0.95`).
    pub fn gel(tint: Color) -> Self {
        Self::new(tint, 0.95)
    }

    /// Targets a specific Bevy light or [`ULightSource`] entity unconditionally.
    pub fn targeting(mut self, target_light: Entity) -> Self {
        self.target_light = Some(target_light);
        self
    }

    /// Sets multiplicative gain scale.
    pub fn with_intensity_scale(mut self, scale: f32) -> Self {
        self.intensity_scale = scale.max(0.0);
        self
    }
}

/// Time-varying light modulation patterns for dynamic light-changing effects.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum ULightModulationMode {
    /// Steady output without animation.
    Steady,
    /// Smooth sinusoidal breathing / dimming wave.
    Pulse {
        speed: f32,
        min_factor: f32,
        max_factor: f32,
    },
    /// Candle / neon-tube realistic multi-harmonic flicker.
    Flicker { speed: f32, depth: f32 },
    /// Smooth rainbow hue rotation preserving saturation and lightness.
    ColorCycle {
        speed: f32,
        saturation: f32,
        lightness: f32,
    },
}

impl Default for ULightModulationMode {
    fn default() -> Self {
        Self::Steady
    }
}

/// Dynamically animates the color and/or power of any Bevy light (`PointLight`, `SpotLight`,
/// `DirectionalLight`, `RectLight`) or [`ULightSource`].
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct ULightModulator {
    /// Enables or pauses modulation.
    pub enabled: bool,
    /// Modulation waveform or color animation.
    pub mode: ULightModulationMode,
    /// Phase offset in radians.
    pub phase_offset: f32,
}

impl Default for ULightModulator {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: ULightModulationMode::Steady,
            phase_offset: 0.0,
        }
    }
}

impl ULightModulator {
    /// Smooth pulsing modulator.
    pub fn pulse(speed: f32, min_factor: f32, max_factor: f32) -> Self {
        Self {
            mode: ULightModulationMode::Pulse {
                speed,
                min_factor: min_factor.max(0.0),
                max_factor: max_factor.max(0.0),
            },
            ..default()
        }
    }

    /// Organic flicker modulator (flame / faulty lamp).
    pub fn flicker(speed: f32, depth: f32) -> Self {
        Self {
            mode: ULightModulationMode::Flicker {
                speed,
                depth: depth.clamp(0.0, 1.0),
            },
            ..default()
        }
    }

    /// Smooth RGB rainbow hue cycling modulator.
    pub fn color_cycle(speed: f32, saturation: f32, lightness: f32) -> Self {
        Self {
            mode: ULightModulationMode::ColorCycle {
                speed,
                saturation: saturation.clamp(0.0, 1.0),
                lightness: lightness.clamp(0.0, 1.0),
            },
            ..default()
        }
    }
}

/// Tracks unmodulated author baseline values for Bevy light entities so modulators and spatial
/// [`ULightFilter`] lenses compose non-destructively and restore cleanly.
#[derive(Component, Debug, Clone)]
pub(crate) struct ULightBaseState {
    pub base_color: LinearRgba,
    pub base_power: f32,
    pub last_written_color: LinearRgba,
    pub last_written_power: f32,
}

/// Real-time physical irradiance sensor attached to any `World3d` UI node, text, or entity.
///
/// Aggregates direct lighting from `DirectionalLight`, `PointLight`, `SpotLight`, and `RectLight`,
/// tests analytical ray intersections against [`ULightOccluder`] and [`ULightFilter`] SDF shapes,
/// and separately reports ambient illumination.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct ULightSensor {
    /// Weighted direct irradiance hitting the sensor surface (lux-equivalent scaled by `1 / 1000`).
    pub direct_illuminance: f32,
    /// Ambient irradiance contribution (scaled `brightness / 100.0`).
    pub ambient_illuminance: f32,
    /// Sum of `direct_illuminance + ambient_illuminance`.
    pub total_illuminance: f32,
    /// Normalized activation ratio (`0.0..=1.0`) against `reference_illuminance`.
    pub normalized_level: f32,
    /// Dominant incident color after optical filtering.
    pub dominant_color: LinearRgba,
    /// `true` when measured light meets [`threshold`](Self::threshold).
    pub is_lit: bool,
    /// Entity of the strongest blocking [`ULightOccluder`] that shadowed a light ray this frame.
    pub occluded_by: Option<Entity>,
    /// Entity of the primary [`ULightFilter`] tinting incident light this frame.
    pub filtered_by: Option<Entity>,
    /// Accepts light hitting either `+Z` or `-Z` face (`true`) or only front `+Z` face (`false`).
    pub double_sided: bool,
    /// Counts global/camera ambient fill when evaluating [`is_lit`](Self::is_lit) and [`normalized_level`](Self::normalized_level).
    pub include_ambient: bool,
    /// Reference illuminance mapped to `normalized_level = 1.0`.
    pub reference_illuminance: f32,
    /// Minimum illuminance required for [`is_lit`](Self::is_lit) to become `true`.
    pub threshold: f32,
}

impl Default for ULightSensor {
    fn default() -> Self {
        Self {
            direct_illuminance: 0.0,
            ambient_illuminance: 0.0,
            total_illuminance: 0.0,
            normalized_level: 0.0,
            dominant_color: LinearRgba::BLACK,
            is_lit: false,
            occluded_by: None,
            filtered_by: None,
            double_sided: true,
            include_ambient: false,
            reference_illuminance: 8.0,
            threshold: 0.65,
        }
    }
}

impl ULightSensor {
    /// Creates a sensor with custom sensitivity reference and lit threshold.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.max(0.0);
        self
    }

    /// Sets the reference illuminance corresponding to `normalized_level = 1.0`.
    pub fn with_reference(mut self, reference_illuminance: f32) -> Self {
        self.reference_illuminance = reference_illuminance.max(0.01);
        self
    }

    /// Chooses whether `GlobalAmbientLight` contributes to [`is_lit`](Self::is_lit).
    pub fn with_ambient(mut self, include_ambient: bool) -> Self {
        self.include_ambient = include_ambient;
        self
    }

    /// Restricts light sensing to the local `+Z` front face only (`double_sided = false`).
    pub fn front_only(mut self) -> Self {
        self.double_sided = false;
        self
    }

    /// Returns whichever illuminance bucket applies according to [`include_ambient`](Self::include_ambient).
    pub fn effective_illuminance(&self) -> f32 {
        if self.include_ambient {
            self.total_illuminance
        } else {
            self.direct_illuminance
        }
    }
}

/// Entity event fired whenever [`URequiresLight::is_active`] changes state.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct ULightAvailabilityChanged {
    /// The entity whose light power state changed.
    #[event_target]
    pub entity: Entity,
    /// New powered state (`true` = sufficient light available).
    pub is_active: bool,
    /// Current measured illuminance at the transition.
    pub illuminance: f32,
}

/// Marks a UI node or interactive control as **dependent on scene lighting**.
///
/// Automatically attaches [`ULightSensor`]. When measured illuminance falls below
/// `min_illuminance` (light off or blocked by an occluder), `is_active` drops to `false`,
/// and interactive picking / focus systems cleanly refuse interaction until light returns.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
#[require(ULightSensor)]
pub struct URequiresLight {
    /// Minimum required illuminance before the element powers on.
    pub min_illuminance: f32,
    /// Whether scene `GlobalAmbientLight` counts toward `min_illuminance` (`false` by default).
    pub include_ambient: bool,
    /// Deadband hysteresis margin to avoid rapid flicker right near the cutoff threshold.
    pub hysteresis: f32,
    /// Current powered state (`true` when sufficient light is present).
    pub is_active: bool,
    /// Automatically blocks pointer picking and clicks when `is_active == false`.
    pub disable_picking_when_unlit: bool,
}

impl Default for URequiresLight {
    fn default() -> Self {
        Self {
            min_illuminance: 0.65,
            include_ambient: false,
            hysteresis: 0.06,
            is_active: false,
            disable_picking_when_unlit: true,
        }
    }
}

impl URequiresLight {
    /// Requires at least `min_illuminance` direct light to operate.
    pub fn with_min_illuminance(min_illuminance: f32) -> Self {
        Self {
            min_illuminance: min_illuminance.max(0.0),
            ..default()
        }
    }

    /// Controls whether global ambient light contributes to powering this component.
    pub fn with_ambient(mut self, include_ambient: bool) -> Self {
        self.include_ambient = include_ambient;
        self
    }

    /// Controls whether pointer picking/clicks are disabled while unlit.
    pub fn with_picking_gate(mut self, disable_picking_when_unlit: bool) -> Self {
        self.disable_picking_when_unlit = disable_picking_when_unlit;
        self
    }

    /// Returns `true` when this node has enough light (or picking gating is disabled)
    /// to accept pointer hits, clicks, and focus.
    pub fn allows_interaction(&self) -> bool {
        !self.disable_picking_when_unlit || self.is_active
    }
}

/// Optional helper that smoothly drives a node's [`UNode::background_color`] and [`UPbr::emissive`]
/// from its [`ULightSensor`] / [`URequiresLight`] state.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
#[require(ULightSensor)]
pub struct ULightReactiveVisual {
    /// Color when fully unlit / offline.
    pub dark_color: Color,
    /// Color when illuminated / active.
    pub lit_color: Color,
    /// Optional HDR emissive glow when lit (zero when unlit).
    pub lit_emissive_boost: f32,
    /// Tints lit color by [`ULightSensor::dominant_color`] when illuminated.
    pub adopt_incident_color: bool,
}

impl Default for ULightReactiveVisual {
    fn default() -> Self {
        Self {
            dark_color: Color::srgba(0.08, 0.09, 0.11, 1.0),
            lit_color: Color::srgba(0.22, 0.85, 0.55, 1.0),
            lit_emissive_boost: 1.5,
            adopt_incident_color: false,
        }
    }
}

/// Evaluates the exact 2D UI SDF (`Round` or `Cut`) matching `sd_box_dynamic` in WGSL/WESL shaders.
pub fn sdf_box_distance_2d(
    p: Vec2,
    half_size: Vec2,
    radius_in: Vec4,
    shape_mode: UShapeMode,
) -> f32 {
    let limit = half_size.x.min(half_size.y).max(0.0);
    let r_vec = radius_in.clamp(Vec4::ZERO, Vec4::splat(limit));

    let is_right = p.x > 0.0;
    let is_top = p.y > 0.0;
    let r_top = if is_right { r_vec.x } else { r_vec.z };
    let r_bot = if is_right { r_vec.y } else { r_vec.w };
    let radius = if is_top { r_top } else { r_bot };

    let q = p.abs() - half_size + Vec2::splat(radius);

    match shape_mode {
        UShapeMode::Cut => {
            let d_square = q.x.max(q.y);
            let d_diagonal = (q.x + q.y) * std::f32::consts::FRAC_1_SQRT_2;
            d_square.max(d_diagonal) - radius
        }
        UShapeMode::Round => q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - radius,
    }
}

fn resolve_entity_world_scale(
    entity: Entity,
    cached: Option<&CachedUiContext>,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
) -> f32 {
    if let Some(ctx) = cached
        && matches!(ctx.space, UiSpace::World3d)
    {
        return ctx.ui_to_world_scale;
    }
    let mut cur = entity;
    loop {
        if let Ok(root) = root_query.get(cur) {
            return root.ui_units_to_world_scale();
        }
        let Ok(parent) = parents_query.get(cur) else {
            return 0.001;
        };
        cur = parent.get();
    }
}

/// Tests whether a world-space ray `origin + dir * t` (`t` in `min_t..max_t`) hits a UI node's
/// local XY plane inside its SDF shape (`Round` or `Cut`), returning hit distance `t` if hit.
pub fn ray_intersects_ui_panel(
    origin: Vec3,
    dir: Vec3,
    min_t: f32,
    max_t: f32,
    panel_transform: &GlobalTransform,
    computed_size: ComputedSize,
    node: &UNode,
    world_scale: f32,
) -> Option<f32> {
    if computed_size.width <= 0.0 || computed_size.height <= 0.0 {
        return None;
    }

    let inv = panel_transform.affine().inverse();
    let origin_local = inv.transform_point3(origin);
    let dir_local = inv.transform_vector3(dir);

    if dir_local.z.abs() < 1.0e-5 {
        return None;
    }

    let t = -origin_local.z / dir_local.z;
    if t <= min_t || t >= max_t {
        return None;
    }

    let hit_local = (origin_local + dir_local * t).truncate();
    let half_size = Vec2::new(computed_size.width, computed_size.height) * 0.5 * world_scale;
    let radius = Vec4::new(
        node.border_radius.top_right,
        node.border_radius.bottom_right,
        node.border_radius.top_left,
        node.border_radius.bottom_left,
    ) * world_scale;

    let dist = sdf_box_distance_2d(hit_local, half_size, radius, node.shape_mode);
    (dist <= 0.0).then_some(t)
}

/// Propagates [`UPbr`] and [`UPbrSurface`] from [`UPbrPropagate`] ancestors to children with [`UPbrInherit`]
/// or all descendants unless [`UPbrKeepUnlit`] is attached.
pub(crate) fn sync_propagated_pbr(
    mut commands: Commands,
    parents_query: Query<&ChildOf>,
    mut queries: ParamSet<(
        Query<(
            Entity,
            Has<UNode>,
            Has<crate::layout::text::UTextLabel>,
            Has<UPbrPropagate>,
            Has<UPbrInherit>,
            Has<UPbrKeepUnlit>,
            Has<InheritedPbrMarker>,
            Has<InheritedPbrSurfaceMarker>,
            Option<&UPbr>,
            Option<&UPbrSurface>,
        )>,
        Query<(Option<&mut UPbr>, Option<&mut UPbrSurface>)>,
    )>,
) {
    let mut pbr_updates: Vec<(Entity, UPbr)> = Vec::new();
    let mut surface_updates: Vec<(Entity, UPbrSurface)> = Vec::new();

    {
        let ro = queries.p0();
        for (
            entity,
            is_node,
            is_text,
            _is_propagating,
            has_inherit,
            keep_unlit,
            has_inherited_pbr_marker,
            has_inherited_surface_marker,
            own_pbr,
            own_surface,
        ) in ro.iter()
        {
            if !is_node && !is_text {
                continue;
            }
            if keep_unlit {
                if has_inherited_pbr_marker
                    && let Ok(mut cmd) = commands.get_entity(entity)
                {
                    cmd.try_remove::<(UPbr, InheritedPbrMarker)>();
                }
                if has_inherited_surface_marker
                    && let Ok(mut cmd) = commands.get_entity(entity)
                {
                    cmd.try_remove::<(UPbrSurface, InheritedPbrSurfaceMarker)>();
                }
                continue;
            }

            let mut inherited_pbr: Option<UPbr> = None;
            let mut inherited_surface: Option<UPbrSurface> = None;

            let mut cur = entity;
            while let Ok(parent) = parents_query.get(cur) {
                let ancestor = parent.get();
                if let Ok((
                    _anc_ent,
                    _anc_node,
                    _anc_text,
                    anc_prop,
                    _anc_inh,
                    _anc_unlit,
                    anc_inh_pbr,
                    anc_inh_surf,
                    anc_pbr,
                    anc_surf,
                )) = ro.get(ancestor)
                {
                    if anc_prop {
                        if inherited_pbr.is_none() {
                            inherited_pbr = anc_pbr.cloned();
                        }
                        if inherited_surface.is_none() {
                            inherited_surface = anc_surf.cloned();
                        }
                        break;
                    }
                    if has_inherit {
                        if inherited_pbr.is_none()
                            && !anc_inh_pbr
                            && let Some(pbr) = anc_pbr
                        {
                            inherited_pbr = Some(pbr.clone());
                            if !anc_inh_surf
                                && let Some(s) = anc_surf
                            {
                                inherited_surface = Some(s.clone());
                            }
                            break;
                        }
                        if inherited_surface.is_none()
                            && !anc_inh_surf
                            && let Some(s) = anc_surf
                        {
                            inherited_surface = Some(s.clone());
                            break;
                        }
                    }
                }
                cur = ancestor;
            }

            match (inherited_pbr, own_pbr, has_inherited_pbr_marker) {
                (Some(target_pbr), Some(current_pbr), true) => {
                    if *current_pbr != target_pbr {
                        pbr_updates.push((entity, target_pbr));
                    }
                }
                (Some(target_pbr), None, _) => {
                    if let Ok(mut cmd) = commands.get_entity(entity) {
                        cmd.try_insert((target_pbr, InheritedPbrMarker));
                    }
                }
                (None, Some(_), true) => {
                    if let Ok(mut cmd) = commands.get_entity(entity) {
                        cmd.try_remove::<(UPbr, InheritedPbrMarker)>();
                    }
                }
                _ => {}
            }

            match (
                inherited_surface,
                own_surface,
                has_inherited_surface_marker,
            ) {
                (Some(target_surface), Some(current_surface), true) => {
                    if *current_surface != target_surface {
                        surface_updates.push((entity, target_surface));
                    }
                }
                (Some(target_surface), None, _) => {
                    if let Ok(mut cmd) = commands.get_entity(entity) {
                        cmd.try_insert((target_surface, InheritedPbrSurfaceMarker));
                    }
                }
                (None, Some(_), true) => {
                    if let Ok(mut cmd) = commands.get_entity(entity) {
                        cmd.try_remove::<(UPbrSurface, InheritedPbrSurfaceMarker)>();
                    }
                }
                _ => {}
            }
        }
    }

    if !pbr_updates.is_empty() || !surface_updates.is_empty() {
        let mut rw = queries.p1();
        for (entity, target_pbr) in pbr_updates {
            if let Ok((Some(mut current), _)) = rw.get_mut(entity) {
                *current = target_pbr;
            }
        }
        for (entity, target_surface) in surface_updates {
            if let Ok((_, Some(mut current))) = rw.get_mut(entity) {
                *current = target_surface;
            }
        }
    }
}

/// Synchronizes GPU shadow casting/receiving markers (`NotShadowCaster`, `NotShadowReceiver`)
/// on both UI node meshes and `UText` glyph page meshes.
pub(crate) fn sync_shadow_caster_markers(
    mut commands: Commands,
    nodes: Query<
        (
            Entity,
            Option<&UPbrSurface>,
            Option<&ULightOccluder>,
            Option<&Children>,
            Has<NotShadowCaster>,
            Has<NotShadowReceiver>,
        ),
        (With<UNode>, With<UI3d>),
    >,
    text_children: Query<
        (Has<NotShadowCaster>, Has<NotShadowReceiver>),
        (With<TextChildMarker>, With<Mesh3d>),
    >,
) {
    for (entity, surface_opt, occluder_opt, children_opt, has_no_cast, has_no_recv) in nodes.iter()
    {
        let should_cast = surface_opt.is_some_and(|s| s.cast_shadows)
            || occluder_opt.is_some_and(|o| o.enabled && o.cast_shadows);
        let should_recv = surface_opt.is_none_or(|s| s.receive_shadows);

        if should_cast == has_no_cast
            && let Ok(mut cmd) = commands.get_entity(entity)
        {
            if should_cast {
                cmd.try_remove::<NotShadowCaster>();
            } else {
                cmd.try_insert(NotShadowCaster);
            }
        }

        if should_recv == has_no_recv
            && let Ok(mut cmd) = commands.get_entity(entity)
        {
            if should_recv {
                cmd.try_remove::<NotShadowReceiver>();
            } else {
                cmd.try_insert(NotShadowReceiver);
            }
        }

        if let Some(children) = children_opt {
            for child in children.iter() {
                if let Ok((child_no_cast, child_no_recv)) = text_children.get(child) {
                    if should_cast == child_no_cast
                        && let Ok(mut cmd) = commands.get_entity(child)
                    {
                        if should_cast {
                            cmd.try_remove::<NotShadowCaster>();
                        } else {
                            cmd.try_insert(NotShadowCaster);
                        }
                    }
                    if should_recv == child_no_recv
                        && let Ok(mut cmd) = commands.get_entity(child)
                    {
                        if should_recv {
                            cmd.try_remove::<NotShadowReceiver>();
                        } else {
                            cmd.try_insert(NotShadowReceiver);
                        }
                    }
                }
            }
        }
    }
}

/// Synchronizes [`ULightSource`] host UI elements / text with physical child Bevy lights
/// (`RectLight`, `PointLight`, `SpotLight`) and host HDR emissive [`UPbr`] glow.
pub(crate) fn sync_light_source_entities(
    mut commands: Commands,
    mut removed_sources: RemovedComponents<ULightSource>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    mut hosts: Query<(
        Entity,
        &ULightSource,
        Option<&ComputedSize>,
        Option<&CachedUiContext>,
        Option<&Children>,
        Option<&mut UPbr>,
        Has<AutoEmissivePbrMarker>,
    )>,
    mut light_children: Query<
        (
            Entity,
            &ULightSourceChild,
            &mut Transform,
            Option<&mut PointLight>,
            Option<&mut SpotLight>,
            Option<&mut RectLight>,
        ),
        Without<ULightSource>,
    >,
) {
    // Cleanup orphaned light children if host lost ULightSource.
    for removed_host in removed_sources.read() {
        if let Ok(mut cmd) = commands.get_entity(removed_host)
            && hosts
                .get(removed_host)
                .is_ok_and(|(_, _, _, _, _, _, auto)| auto)
        {
            cmd.try_remove::<(UPbr, AutoEmissivePbrMarker)>();
        }
        for (child_entity, child_tag, ..) in light_children.iter() {
            if child_tag.source_entity == removed_host
                && let Ok(mut cmd) = commands.get_entity(child_entity)
            {
                cmd.try_despawn();
            }
        }
    }

    for (entity, source, size_opt, cached_opt, children_opt, pbr_opt, has_auto_pbr) in
        hosts.iter_mut()
    {
        let eff_dim = if source.enabled {
            source.dimmer.max(0.0)
        } else {
            0.0
        };
        let target_emissive = if source.sync_emissive {
            let lin = LinearRgba::from(source.color);
            let gain = eff_dim * source.emissive_strength.max(0.0);
            LinearRgba::rgb(lin.red * gain, lin.green * gain, lin.blue * gain)
        } else {
            LinearRgba::BLACK
        };

        if source.sync_emissive {
            if let Some(mut pbr) = pbr_opt {
                if pbr.emissive != target_emissive {
                    pbr.emissive = target_emissive;
                }
            } else if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_insert((
                    UPbr::new(0.0, 0.25).with_emissive(target_emissive),
                    AutoEmissivePbrMarker,
                ));
            }
        } else if has_auto_pbr
            && let Some(mut pbr) = pbr_opt
            && pbr.emissive != LinearRgba::BLACK
        {
            pbr.emissive = LinearRgba::BLACK;
        }

        // Bevy SpotLight & RectLight shine along local -Z; UI nodes face local +Z.
        let desired_rot = if source.direction_sign >= 0.0 {
            Quat::from_rotation_y(PI)
        } else {
            Quat::IDENTITY
        };
        let desired_tf = Transform::from_translation(source.local_offset).with_rotation(desired_rot);

        let world_scale = resolve_entity_world_scale(entity, cached_opt, &parents_query, &root_query);
        let logical_size = size_opt.map_or(Vec2::new(120.0, 120.0), |s| {
            Vec2::new(s.width.max(12.0), s.height.max(12.0))
        });
        let world_panel_size = (logical_size * world_scale).max(Vec2::splat(0.04));

        let eff_intensity = source.intensity.max(0.0) * eff_dim;

        // Locate existing light child if attached.
        let mut matched_child: Option<Entity> = None;
        if let Some(children) = children_opt {
            for child in children.iter() {
                if let Ok((_, child_tag, ..)) = light_children.get(child)
                    && child_tag.source_entity == entity
                {
                    if matched_child.is_none() {
                        matched_child = Some(child);
                    } else if let Ok(mut cmd) = commands.get_entity(child) {
                        cmd.try_despawn();
                    }
                }
            }
        }

        if let Some(child_entity) = matched_child
            && let Ok((_, _, mut tf, pt_opt, sp_opt, rect_opt)) =
                light_children.get_mut(child_entity)
        {
            if *tf != desired_tf {
                *tf = desired_tf;
            }
            let type_matches = match (source.kind, pt_opt, sp_opt, rect_opt) {
                (ULightSourceKind::Point { radius }, Some(mut pt), None, None) => {
                    pt.color = source.color;
                    pt.intensity = eff_intensity;
                    pt.range = source.range;
                    pt.radius = radius;
                    pt.shadow_maps_enabled = source.shadow_maps_enabled;
                    true
                }
                (
                    ULightSourceKind::Spot {
                        radius,
                        inner_angle,
                        outer_angle,
                    },
                    None,
                    Some(mut sp),
                    None,
                ) => {
                    sp.color = source.color;
                    sp.intensity = eff_intensity;
                    sp.range = source.range;
                    sp.radius = radius;
                    sp.inner_angle = inner_angle;
                    sp.outer_angle = outer_angle;
                    sp.shadow_maps_enabled = source.shadow_maps_enabled;
                    true
                }
                (ULightSourceKind::Rect { size_override }, None, None, Some(mut rect)) => {
                    let sz = size_override.unwrap_or(world_panel_size).max(Vec2::splat(0.02));
                    rect.color = source.color;
                    rect.intensity = eff_intensity;
                    rect.range = source.range;
                    rect.width = sz.x;
                    rect.height = sz.y;
                    true
                }
                _ => false,
            };

            if type_matches {
                continue;
            }
            if let Ok(mut cmd) = commands.get_entity(child_entity) {
                cmd.try_despawn();
            }
        }

        // Spawn a fresh child Bevy light.
        let spawn_child = |parent: &mut ChildSpawnerCommands| match source.kind {
            ULightSourceKind::Point { radius } => {
                parent.spawn((
                    ULightSourceChild {
                        source_entity: entity,
                    },
                    desired_tf,
                    PointLight {
                        color: source.color,
                        intensity: eff_intensity,
                        range: source.range,
                        radius,
                        shadow_maps_enabled: source.shadow_maps_enabled,
                        ..default()
                    },
                ));
            }
            ULightSourceKind::Spot {
                radius,
                inner_angle,
                outer_angle,
            } => {
                parent.spawn((
                    ULightSourceChild {
                        source_entity: entity,
                    },
                    desired_tf,
                    SpotLight {
                        color: source.color,
                        intensity: eff_intensity,
                        range: source.range,
                        radius,
                        inner_angle,
                        outer_angle,
                        shadow_maps_enabled: source.shadow_maps_enabled,
                        ..default()
                    },
                ));
            }
            ULightSourceKind::Rect { size_override } => {
                let sz = size_override.unwrap_or(world_panel_size).max(Vec2::splat(0.02));
                parent.spawn((
                    ULightSourceChild {
                        source_entity: entity,
                    },
                    desired_tf,
                    RectLight {
                        color: source.color,
                        intensity: eff_intensity,
                        range: source.range,
                        width: sz.x,
                        height: sz.y,
                    },
                ));
            }
        };

        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.with_children(|parent| spawn_child(parent));
        }
    }
}

fn evaluate_modulator(modulator: &ULightModulator, elapsed: f32) -> (Option<LinearRgba>, f32) {
    if !modulator.enabled {
        return (None, 1.0);
    }
    let t = elapsed + modulator.phase_offset;
    match modulator.mode {
        ULightModulationMode::Steady => (None, 1.0),
        ULightModulationMode::Pulse {
            speed,
            min_factor,
            max_factor,
        } => {
            let s = (t * speed).sin() * 0.5 + 0.5;
            (None, min_factor + (max_factor - min_factor) * s)
        }
        ULightModulationMode::Flicker { speed, depth } => {
            let wave = ((t * speed).sin() * 0.55
                + (t * speed * 2.37 + 1.1).sin() * 0.3
                + (t * speed * 5.13 + 2.7).sin() * 0.15)
                * 0.5
                + 0.5;
            let factor = 1.0 - depth * (1.0 - wave.clamp(0.0, 1.0));
            (None, factor.max(0.0))
        }
        ULightModulationMode::ColorCycle {
            speed,
            saturation,
            lightness,
        } => {
            let hue = (t * speed * 60.0).rem_euclid(360.0);
            let col = LinearRgba::from(Color::hsl(hue, saturation, lightness));
            (Some(col), 1.0)
        }
    }
}

fn linear_to_vec4(c: LinearRgba) -> Vec4 {
    Vec4::new(c.red, c.green, c.blue, c.alpha)
}

fn resolve_base_and_update_light(
    entity: Entity,
    current_color: Color,
    current_power: f32,
    base_state_opt: Option<&ULightBaseState>,
    commands: &mut Commands,
) -> (LinearRgba, f32) {
    let cur_lin = LinearRgba::from(current_color);
    if let Some(state) = base_state_opt {
        let color_changed = (linear_to_vec4(cur_lin) - linear_to_vec4(state.last_written_color))
            .abs()
            .max_element()
            > 1.0e-4;
        let power_changed = (current_power - state.last_written_power).abs() > 1.0e-2;
        let base_c = if color_changed {
            cur_lin
        } else {
            state.base_color
        };
        let base_p = if power_changed {
            current_power
        } else {
            state.base_power
        };
        (base_c, base_p)
    } else {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.try_insert(ULightBaseState {
                base_color: cur_lin,
                base_power: current_power,
                last_written_color: cur_lin,
                last_written_power: current_power,
            });
        }
        (cur_lin, current_power)
    }
}

fn commit_light_base_state(
    entity: Entity,
    base_color: LinearRgba,
    base_power: f32,
    written_color: LinearRgba,
    written_power: f32,
    base_state_opt: Option<Mut<'_, ULightBaseState>>,
    commands: &mut Commands,
) {
    if let Some(mut state) = base_state_opt {
        state.base_color = base_color;
        state.base_power = base_power;
        state.last_written_color = written_color;
        state.last_written_power = written_power;
    } else if let Ok(mut cmd) = commands.get_entity(entity) {
        cmd.try_insert(ULightBaseState {
            base_color,
            base_power,
            last_written_color: written_color,
            last_written_power: written_power,
        });
    }
}

/// Applies [`ULightModulator`] and [`ULightFilter`] (direct or spatial beam interception)
/// cleanly across scene Bevy lights (`PointLight`, `SpotLight`, `DirectionalLight`, `RectLight`).
pub(crate) fn apply_light_filters_and_modulators(
    mut commands: Commands,
    time: Option<Res<Time>>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    spatial_filters: Query<(
        Entity,
        &ULightFilter,
        &GlobalTransform,
        Option<&ComputedSize>,
        Option<&UNode>,
        Option<&CachedUiContext>,
        Option<&Visibility>,
    )>,
    mut lights: Query<(
        Entity,
        &GlobalTransform,
        Option<&ULightSourceChild>,
        Option<&ULightModulator>,
        Option<&ULightFilter>,
        Option<&mut ULightBaseState>,
        Option<&mut PointLight>,
        Option<&mut SpotLight>,
        Option<&mut DirectionalLight>,
        Option<&mut RectLight>,
    )>,
) {
    let elapsed = time.as_ref().map_or(0.0, |t| t.elapsed_secs());

    // Precompute spatial filter descriptors.
    struct FilterEntry {
        entity: Entity,
        target_light: Option<Entity>,
        spatial: bool,
        probe_dist: f32,
        tint: LinearRgba,
        gain: f32,
        gt: GlobalTransform,
        size: ComputedSize,
        node: UNode,
        scale: f32,
    }

    let mut active_filters = Vec::new();
    for (f_ent, f, f_gt, size_opt, node_opt, cached_opt, vis_opt) in spatial_filters.iter() {
        if !f.enabled || matches!(vis_opt, Some(Visibility::Hidden)) {
            continue;
        }
        let scale = resolve_entity_world_scale(f_ent, cached_opt, &parents_query, &root_query);
        active_filters.push(FilterEntry {
            entity: f_ent,
            target_light: f.target_light,
            spatial: f.spatial_beam_filtering,
            probe_dist: f.probe_distance.max(0.05),
            tint: LinearRgba::from(f.tint),
            gain: f.transmittance.clamp(0.0, 1.0) * f.intensity_scale.max(0.0),
            gt: *f_gt,
            size: size_opt.copied().unwrap_or_default(),
            node: node_opt.cloned().unwrap_or_default(),
            scale,
        });
    }

    for (
        light_ent,
        light_gt,
        source_child_opt,
        modulator_opt,
        self_filter_opt,
        base_opt,
        pt_opt,
        sp_opt,
        dir_opt,
        rect_opt,
    ) in lights.iter_mut()
    {
        if pt_opt.is_none() && sp_opt.is_none() && dir_opt.is_none() && rect_opt.is_none() {
            continue;
        }
        let light_pos = light_gt.translation();
        let light_dir = (light_gt.affine().matrix3 * Vec3::NEG_Z).normalize_or(Vec3::NEG_Z);
        let is_directional_beam = sp_opt.is_some() || dir_opt.is_some() || rect_opt.is_some();

        // Accumulate modulators + filters matching this light.
        let mut combined_tint = LinearRgba::WHITE;
        let mut combined_gain = 1.0_f32;

        if let Some(modulator) = modulator_opt {
            let (override_col, factor) = evaluate_modulator(modulator, elapsed);
            if let Some(col) = override_col {
                combined_tint = LinearRgba::rgb(
                    combined_tint.red * col.red,
                    combined_tint.green * col.green,
                    combined_tint.blue * col.blue,
                );
            }
            combined_gain *= factor;
        }

        if let Some(self_f) = self_filter_opt
            && self_f.enabled
        {
            let tint = LinearRgba::from(self_f.tint);
            combined_tint = LinearRgba::rgb(
                combined_tint.red * tint.red,
                combined_tint.green * tint.green,
                combined_tint.blue * tint.blue,
            );
            combined_gain *= self_f.transmittance.clamp(0.0, 1.0) * self_f.intensity_scale.max(0.0);
        }

        for entry in &active_filters {
            if entry.entity == light_ent {
                continue;
            }
            let targeted = entry.target_light == Some(light_ent)
                || source_child_opt
                    .is_some_and(|sc| entry.target_light == Some(sc.source_entity));
            let spatial_hit = !targeted
                && entry.spatial
                && is_directional_beam
                && ray_intersects_ui_panel(
                    light_pos,
                    light_dir,
                    0.02,
                    entry.probe_dist,
                    &entry.gt,
                    entry.size,
                    &entry.node,
                    entry.scale,
                )
                .is_some();

            if targeted || spatial_hit {
                combined_tint = LinearRgba::rgb(
                    combined_tint.red * entry.tint.red,
                    combined_tint.green * entry.tint.green,
                    combined_tint.blue * entry.tint.blue,
                );
                combined_gain *= entry.gain;
            }
        }

        if let Some(mut pt) = pt_opt {
            let (base_c, base_p) = resolve_base_and_update_light(
                light_ent,
                pt.color,
                pt.intensity,
                base_opt.as_deref(),
                &mut commands,
            );
            let final_c = LinearRgba::rgb(
                base_c.red * combined_tint.red,
                base_c.green * combined_tint.green,
                base_c.blue * combined_tint.blue,
            );
            let final_p = base_p * combined_gain;
            pt.color = Color::LinearRgba(final_c);
            pt.intensity = final_p;
            commit_light_base_state(
                light_ent,
                base_c,
                base_p,
                final_c,
                final_p,
                base_opt,
                &mut commands,
            );
        } else if let Some(mut sp) = sp_opt {
            let (base_c, base_p) = resolve_base_and_update_light(
                light_ent,
                sp.color,
                sp.intensity,
                base_opt.as_deref(),
                &mut commands,
            );
            let final_c = LinearRgba::rgb(
                base_c.red * combined_tint.red,
                base_c.green * combined_tint.green,
                base_c.blue * combined_tint.blue,
            );
            let final_p = base_p * combined_gain;
            sp.color = Color::LinearRgba(final_c);
            sp.intensity = final_p;
            commit_light_base_state(
                light_ent,
                base_c,
                base_p,
                final_c,
                final_p,
                base_opt,
                &mut commands,
            );
        } else if let Some(mut dir) = dir_opt {
            let (base_c, base_p) = resolve_base_and_update_light(
                light_ent,
                dir.color,
                dir.illuminance,
                base_opt.as_deref(),
                &mut commands,
            );
            let final_c = LinearRgba::rgb(
                base_c.red * combined_tint.red,
                base_c.green * combined_tint.green,
                base_c.blue * combined_tint.blue,
            );
            let final_p = base_p * combined_gain;
            dir.color = Color::LinearRgba(final_c);
            dir.illuminance = final_p;
            commit_light_base_state(
                light_ent,
                base_c,
                base_p,
                final_c,
                final_p,
                base_opt,
                &mut commands,
            );
        } else if let Some(mut rect) = rect_opt {
            let (base_c, base_p) = resolve_base_and_update_light(
                light_ent,
                rect.color,
                rect.intensity,
                base_opt.as_deref(),
                &mut commands,
            );
            let final_c = LinearRgba::rgb(
                base_c.red * combined_tint.red,
                base_c.green * combined_tint.green,
                base_c.blue * combined_tint.blue,
            );
            let final_p = base_p * combined_gain;
            rect.color = Color::LinearRgba(final_c);
            rect.intensity = final_p;
            commit_light_base_state(
                light_ent,
                base_c,
                base_p,
                final_c,
                final_p,
                base_opt,
                &mut commands,
            );
        }
    }
}

/// Evaluates [`ULightSensor`] irradiance, occlusion, and optical filtering,
/// updates [`URequiresLight`] + fires [`ULightAvailabilityChanged`], and applies [`ULightReactiveVisual`].
pub(crate) fn update_light_sensors_and_requirements(
    mut commands: Commands,
    global_ambient: Option<Res<GlobalAmbientLight>>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    dir_lights: Query<(Entity, &DirectionalLight, &GlobalTransform)>,
    point_lights: Query<(Entity, &PointLight, &GlobalTransform, Option<&ULightSourceChild>)>,
    spot_lights: Query<(Entity, &SpotLight, &GlobalTransform, Option<&ULightSourceChild>)>,
    rect_lights: Query<(Entity, &RectLight, &GlobalTransform, Option<&ULightSourceChild>)>,
    mut queries: ParamSet<(
        Query<(
            Entity,
            &ULightOccluder,
            &GlobalTransform,
            &ComputedSize,
            &UNode,
            Option<&CachedUiContext>,
            Option<&Visibility>,
        )>,
        Query<(
            Entity,
            &ULightFilter,
            &GlobalTransform,
            &ComputedSize,
            &UNode,
            Option<&CachedUiContext>,
            Option<&Visibility>,
        )>,
        Query<(
            Entity,
            &GlobalTransform,
            &mut ULightSensor,
            Option<&mut URequiresLight>,
            Option<&ULightReactiveVisual>,
            Option<&mut UNode>,
            Option<&mut UPbr>,
        )>,
    )>,
) {
    struct OccluderData {
        entity: Entity,
        gt: GlobalTransform,
        size: ComputedSize,
        node: UNode,
        scale: f32,
        transmittance: f32,
    }

    let mut occluders = Vec::new();
    for (ent, occ, gt, size, node, cached, vis) in queries.p0().iter() {
        if !occ.enabled || !occ.block_sensors || matches!(vis, Some(Visibility::Hidden)) {
            continue;
        }
        occluders.push(OccluderData {
            entity: ent,
            gt: *gt,
            size: *size,
            node: node.clone(),
            scale: resolve_entity_world_scale(ent, cached, &parents_query, &root_query),
            transmittance: occ.transmittance.clamp(0.0, 1.0),
        });
    }

    struct FilterRayData {
        entity: Entity,
        gt: GlobalTransform,
        size: ComputedSize,
        node: UNode,
        scale: f32,
        tint: LinearRgba,
        gain: f32,
    }

    let mut ray_filters = Vec::new();
    for (ent, filter, gt, size, node, cached, vis) in queries.p1().iter() {
        if !filter.enabled
            || !filter.spatial_beam_filtering
            || matches!(vis, Some(Visibility::Hidden))
        {
            continue;
        }
        ray_filters.push(FilterRayData {
            entity: ent,
            gt: *gt,
            size: *size,
            node: node.clone(),
            scale: resolve_entity_world_scale(ent, cached, &parents_query, &root_query),
            tint: LinearRgba::from(filter.tint),
            gain: filter.transmittance.clamp(0.0, 1.0) * filter.intensity_scale.max(0.0),
        });
    }

    let (ambient_color, ambient_brightness) = global_ambient.as_ref().map_or(
        (LinearRgba::WHITE, 0.0),
        |a| (LinearRgba::from(a.color), a.brightness.max(0.0)),
    );
    let ambient_irrad = ambient_brightness / 100.0;

    for (
        sensor_entity,
        sensor_gt,
        mut sensor,
        mut req_opt,
        reactive_opt,
        mut node_opt,
        mut pbr_opt,
    ) in queries.p2().iter_mut()
    {
        let sensor_pos = sensor_gt.translation();
        let sensor_normal = (sensor_gt.affine().matrix3 * Vec3::Z).normalize_or(Vec3::Z); // local +Z front face

        let mut direct_total = 0.0_f32;
        let mut color_accum = Vec3::ZERO;
        let mut strongest_blocker: Option<(f32, Entity)> = None;
        let mut strongest_filter: Option<(f32, Entity)> = None;

        let trace_ray = |ray_origin: Vec3,
                         to_light: Vec3,
                         max_dist: f32,
                         base_power: f32,
                         base_rgb: Vec3,
                         strongest_blocker: &mut Option<(f32, Entity)>,
                         strongest_filter: &mut Option<(f32, Entity)>|
         -> (f32, Vec3) {
            if base_power <= 1.0e-5 {
                return (0.0, base_rgb);
            }
            let mut transmission = 1.0_f32;
            let mut rgb = base_rgb;

            for occ in &occluders {
                if occ.entity == sensor_entity {
                    continue;
                }
                if ray_intersects_ui_panel(
                    ray_origin,
                    to_light,
                    0.015,
                    max_dist,
                    &occ.gt,
                    occ.size,
                    &occ.node,
                    occ.scale,
                )
                .is_some()
                {
                    transmission *= occ.transmittance;
                    if strongest_blocker
                        .as_ref()
                        .is_none_or(|(best_p, _)| base_power > *best_p)
                    {
                        *strongest_blocker = Some((base_power, occ.entity));
                    }
                    if transmission <= 1.0e-4 {
                        return (0.0, rgb);
                    }
                }
            }

            for rf in &ray_filters {
                if rf.entity == sensor_entity {
                    continue;
                }
                if ray_intersects_ui_panel(
                    ray_origin,
                    to_light,
                    0.015,
                    max_dist,
                    &rf.gt,
                    rf.size,
                    &rf.node,
                    rf.scale,
                )
                .is_some()
                {
                    transmission *= rf.gain;
                    rgb *= Vec3::new(rf.tint.red, rf.tint.green, rf.tint.blue);
                    if strongest_filter
                        .as_ref()
                        .is_none_or(|(best_p, _)| base_power > *best_p)
                    {
                        *strongest_filter = Some((base_power, rf.entity));
                    }
                }
            }

            (base_power * transmission, rgb)
        };

        let incidence = |to_light: Vec3, double_sided: bool| -> f32 {
            let d = sensor_normal.dot(to_light);
            if double_sided {
                d.abs().max(0.2)
            } else {
                d.max(0.0)
            }
        };

        // 1. Directional Lights
        for (_l_ent, dir, l_gt) in dir_lights.iter() {
            if dir.illuminance <= 0.0 {
                continue;
            }
            // Directional light shines along local -Z, so light direction vector *to* light is +Z (back).
            let to_light = (l_gt.affine().matrix3 * Vec3::Z).normalize_or(Vec3::Y);
            let ndotl = incidence(to_light, sensor.double_sided);
            if ndotl <= 0.0 {
                continue;
            }
            let base = (dir.illuminance / 1000.0) * ndotl;
            let lin = LinearRgba::from(dir.color);
            let (power, rgb) = trace_ray(
                sensor_pos,
                to_light,
                500.0,
                base,
                Vec3::new(lin.red, lin.green, lin.blue),
                &mut strongest_blocker,
                &mut strongest_filter,
            );
            direct_total += power;
            color_accum += rgb * power;
        }

        // 2. Point Lights
        for (_l_ent, pt, l_gt, source_child_opt) in point_lights.iter() {
            if pt.intensity <= 0.0 || source_child_opt.is_some_and(|sc| sc.source_entity == sensor_entity) {
                continue;
            }
            let delta = l_gt.translation() - sensor_pos;
            let dist = delta.length();
            if dist > pt.range || dist < 1.0e-4 {
                continue;
            }
            let to_light = delta / dist;
            let ndotl = incidence(to_light, sensor.double_sided);
            if ndotl <= 0.0 {
                continue;
            }
            let range_atten = (1.0 - (dist / pt.range).powi(4)).clamp(0.0, 1.0).powi(2);
            let base = ((pt.intensity / (4.0 * PI * (dist * dist + 0.1))) * range_atten / 100.0)
                * ndotl;
            let lin = LinearRgba::from(pt.color);
            let (power, rgb) = trace_ray(
                sensor_pos,
                to_light,
                dist,
                base,
                Vec3::new(lin.red, lin.green, lin.blue),
                &mut strongest_blocker,
                &mut strongest_filter,
            );
            direct_total += power;
            color_accum += rgb * power;
        }

        // 3. Spot Lights
        for (_l_ent, sp, l_gt, source_child_opt) in spot_lights.iter() {
            if sp.intensity <= 0.0 || source_child_opt.is_some_and(|sc| sc.source_entity == sensor_entity) {
                continue;
            }
            let delta = l_gt.translation() - sensor_pos;
            let dist = delta.length();
            if dist > sp.range || dist < 1.0e-4 {
                continue;
            }
            let to_light = delta / dist;
            let spot_forward = (l_gt.affine().matrix3 * Vec3::NEG_Z).normalize_or(Vec3::NEG_Z);
            let cos_theta = (-to_light).dot(spot_forward);
            let cos_outer = sp.outer_angle.cos();
            let cos_inner = sp.inner_angle.cos().max(cos_outer + 1.0e-4);
            if cos_theta <= cos_outer {
                continue;
            }
            let cone_atten = ((cos_theta - cos_outer) / (cos_inner - cos_outer)).clamp(0.0, 1.0);
            let cone_smooth = cone_atten * cone_atten;

            let ndotl = incidence(to_light, sensor.double_sided);
            if ndotl <= 0.0 {
                continue;
            }
            let range_atten = (1.0 - (dist / sp.range).powi(4)).clamp(0.0, 1.0).powi(2);
            let base = ((sp.intensity / (4.0 * PI * (dist * dist + 0.1)))
                * range_atten
                * cone_smooth
                / 80.0)
                * ndotl;
            let lin = LinearRgba::from(sp.color);
            let (power, rgb) = trace_ray(
                sensor_pos,
                to_light,
                dist,
                base,
                Vec3::new(lin.red, lin.green, lin.blue),
                &mut strongest_blocker,
                &mut strongest_filter,
            );
            direct_total += power;
            color_accum += rgb * power;
        }

        // 4. Rect Area Lights
        for (_l_ent, rect, l_gt, source_child_opt) in rect_lights.iter() {
            if rect.intensity <= 0.0 || source_child_opt.is_some_and(|sc| sc.source_entity == sensor_entity) {
                continue;
            }
            let delta = l_gt.translation() - sensor_pos;
            let dist = delta.length();
            if dist > rect.range || dist < 1.0e-4 {
                continue;
            }
            let to_light = delta / dist;
            let rect_forward = (l_gt.affine().matrix3 * Vec3::NEG_Z).normalize_or(Vec3::NEG_Z);
            let emitter_cos = (-to_light).dot(rect_forward);
            if emitter_cos <= 0.0 {
                continue;
            }
            let ndotl = incidence(to_light, sensor.double_sided);
            if ndotl <= 0.0 {
                continue;
            }
            let range_atten = (1.0 - (dist / rect.range).powi(4)).clamp(0.0, 1.0).powi(2);
            let base = ((rect.intensity * emitter_cos / (PI * (dist * dist + 0.25)))
                * range_atten
                / 90.0)
                * ndotl;
            let lin = LinearRgba::from(rect.color);
            let (power, rgb) = trace_ray(
                sensor_pos,
                to_light,
                dist,
                base,
                Vec3::new(lin.red, lin.green, lin.blue),
                &mut strongest_blocker,
                &mut strongest_filter,
            );
            direct_total += power;
            color_accum += rgb * power;
        }

        sensor.direct_illuminance = direct_total;
        sensor.ambient_illuminance = ambient_irrad;
        sensor.total_illuminance = direct_total + ambient_irrad;
        sensor.occluded_by = strongest_blocker.map(|(_, e)| e);
        sensor.filtered_by = strongest_filter.map(|(_, e)| e);

        let effective = if sensor.include_ambient {
            sensor.total_illuminance
        } else {
            sensor.direct_illuminance
        };
        sensor.normalized_level =
            (effective / sensor.reference_illuminance.max(0.01)).clamp(0.0, 1.0);
        sensor.is_lit = effective >= sensor.threshold;

        let total_color_w = if sensor.include_ambient && ambient_irrad > 0.0 {
            let amb_rgb = Vec3::new(ambient_color.red, ambient_color.green, ambient_color.blue)
                * ambient_irrad;
            color_accum + amb_rgb
        } else {
            color_accum
        };
        let weight_sum = if sensor.include_ambient {
            direct_total + ambient_irrad
        } else {
            direct_total
        };
        sensor.dominant_color = if weight_sum > 1.0e-5 {
            let norm = total_color_w / weight_sum;
            LinearRgba::rgb(norm.x.clamp(0.0, 4.0), norm.y.clamp(0.0, 4.0), norm.z.clamp(0.0, 4.0))
        } else {
            LinearRgba::BLACK
        };

        if let Some(ref mut req) = req_opt {
            let req_illuminance = if req.include_ambient {
                sensor.total_illuminance
            } else {
                sensor.direct_illuminance
            };
            let turn_on_thresh = req.min_illuminance;
            let turn_off_thresh = (req.min_illuminance - req.hysteresis.max(0.0)).max(0.0);
            let next_active = if req.is_active {
                req_illuminance >= turn_off_thresh
            } else {
                req_illuminance >= turn_on_thresh
            };
            if next_active != req.is_active {
                req.is_active = next_active;
                commands.trigger(ULightAvailabilityChanged {
                    entity: sensor_entity,
                    is_active: next_active,
                    illuminance: req_illuminance,
                });
            }
        }

        if let Some(reactive) = reactive_opt {
            let factor = sensor.normalized_level;
            let base_dark = LinearRgba::from(reactive.dark_color);
            let mut base_lit = LinearRgba::from(reactive.lit_color);
            if reactive.adopt_incident_color && direct_total > 1.0e-4 {
                base_lit = LinearRgba::rgb(
                    base_lit.red * sensor.dominant_color.red,
                    base_lit.green * sensor.dominant_color.green,
                    base_lit.blue * sensor.dominant_color.blue,
                );
            }
            let mixed = LinearRgba::new(
                base_dark.red + (base_lit.red - base_dark.red) * factor,
                base_dark.green + (base_lit.green - base_dark.green) * factor,
                base_dark.blue + (base_lit.blue - base_dark.blue) * factor,
                base_dark.alpha + (base_lit.alpha - base_dark.alpha) * factor,
            );
            if let Some(ref mut node) = node_opt {
                let target_col = Color::LinearRgba(mixed);
                if node.background_color != target_col {
                    node.background_color = target_col;
                }
            }
            if let Some(ref mut pbr) = pbr_opt {
                let e_scale = factor * reactive.lit_emissive_boost.max(0.0);
                let target_emissive = LinearRgba::rgb(
                    base_lit.red * e_scale,
                    base_lit.green * e_scale,
                    base_lit.blue * e_scale,
                );
                if pbr.emissive != target_emissive {
                    pbr.emissive = target_emissive;
                }
            }
        }
    }
}

/// Plugin installing 3D lighting sources, SDF occluders, optical filters, modulators,
/// and light-availability sensor systems.
pub struct UnivisLightingPlugin;

impl Plugin for UnivisLightingPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UPbr>()
            .register_type::<UPbrSurface>()
            .register_type::<UPbrInherit>()
            .register_type::<UPbrPropagate>()
            .register_type::<UPbrKeepUnlit>()
            .register_type::<ULightSourceKind>()
            .register_type::<ULightSource>()
            .register_type::<ULightOccluder>()
            .register_type::<ULightFilter>()
            .register_type::<ULightModulationMode>()
            .register_type::<ULightModulator>()
            .register_type::<ULightSensor>()
            .register_type::<URequiresLight>()
            .register_type::<ULightReactiveVisual>()
            .add_systems(
                UiSettlementSchedule,
                (
                    sync_propagated_pbr,
                    sync_light_source_entities,
                    apply_light_filters_and_modulators,
                )
                    .chain()
                    .before(UnivisPostUpdateSet::RenderSync),
            )
            .add_systems(
                PostUpdate,
                (
                    sync_shadow_caster_markers.before(VisibilitySystems::VisibilityPropagate),
                    update_light_sensors_and_requirements.after(TransformSystems::Propagate),
                ),
            );
    }
}

// Keep unused PI/TAU/FRAC_PI_4 imports neat if needed in helpers/tests.
#[allow(dead_code)]
fn _lighting_angle_consts() -> (f32, f32) {
    (FRAC_PI_4, TAU)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::geometry::{UCornerRadius, UVal};
    use crate::layout::layout_system::URootUi;
    use crate::layout::text::UTextLabel;
    use bevy::asset::AssetPlugin;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn pbr_source_literal_compatibility_and_presets() {
        let custom = UPbr {
            metallic: 0.75,
            roughness: 0.25,
            emissive: LinearRgba::rgb(0.1, 0.2, 0.3),
        };
        assert!((custom.metallic - 0.75).abs() < 1e-6);

        let wood = UPbr::wood();
        assert_eq!(wood.metallic, 0.0);
        assert!(wood.roughness > 0.8);
        assert_eq!(wood.emissive, LinearRgba::BLACK);

        let lamp = UPbr::lamp(Color::WHITE, 4.0);
        assert!(lamp.emissive.red >= 3.9);
    }

    #[test]
    fn sdf_ray_occlusion_hits_rounded_and_cut_ui_shapes() {
        let gt = GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0));
        let size = ComputedSize {
            width: 200.0,
            height: 200.0,
            ..default()
        };
        let scale = 0.01; // 2.0 x 2.0 meters, half-size = 1.0

        let rounded_node = UNode {
            border_radius: UCornerRadius::all(40.0), // 0.40 meters radius
            shape_mode: UShapeMode::Round,
            ..default()
        };
        // Center hit succeeds
        assert!(
            ray_intersects_ui_panel(
                Vec3::new(0.0, 0.0, -2.0),
                Vec3::Z,
                0.01,
                10.0,
                &gt,
                size,
                &rounded_node,
                scale,
            )
            .is_some()
        );
        // Far corner (0.95, 0.95) is outside rounded radius 0.40 -> ray passes cleanly!
        assert!(
            ray_intersects_ui_panel(
                Vec3::new(0.95, 0.95, -2.0),
                Vec3::Z,
                0.01,
                10.0,
                &gt,
                size,
                &rounded_node,
                scale,
            )
            .is_none()
        );

        let cut_node = UNode {
            border_radius: UCornerRadius::all(40.0),
            shape_mode: UShapeMode::Cut,
            ..default()
        };
        assert!(
            ray_intersects_ui_panel(
                Vec3::new(0.90, 0.90, -2.0),
                Vec3::Z,
                0.01,
                10.0,
                &gt,
                size,
                &cut_node,
                scale,
            )
            .is_none()
        );
    }

    #[test]
    fn light_sensor_detects_light_blocking_filtering_and_requirements() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, AssetPlugin::default()));
        app.insert_resource(GlobalAmbientLight::NONE);

        // Directional sun shining from +Z toward -Z (looking along -Z).
        app.world_mut().spawn((
            DirectionalLight {
                color: Color::WHITE,
                illuminance: 10_000.0,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            GlobalTransform::from(
                Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ),
        ));

        // Sensor node at origin facing +Z
        let sensor_entity = app
            .world_mut()
            .spawn((
                UNode {
                    width: UVal::Px(100.0),
                    height: UVal::Px(100.0),
                    ..default()
                },
                URequiresLight::with_min_illuminance(2.0),
                Transform::default(),
                GlobalTransform::default(),
            ))
            .id();

        app.world_mut()
            .run_system_once(update_light_sensors_and_requirements)
            .unwrap();

        let req = app.world().entity(sensor_entity).get::<URequiresLight>().unwrap();
        assert!(req.is_active, "Direct light should activate URequiresLight");

        // Place an opaque UI occluder between light and sensor
        let blocker = app
            .world_mut()
            .spawn((
                UNode::default(),
                ComputedSize {
                    width: 400.0,
                    height: 400.0,
                    ..default()
                },
                ULightOccluder::solid(),
                Transform::from_xyz(0.0, 0.0, 1.5),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 1.5)),
            ))
            .id();

        app.world_mut()
            .run_system_once(update_light_sensors_and_requirements)
            .unwrap();

        let sensor = app.world().entity(sensor_entity).get::<ULightSensor>().unwrap();
        let req = app.world().entity(sensor_entity).get::<URequiresLight>().unwrap();
        assert_eq!(sensor.occluded_by, Some(blocker));
        assert!(sensor.direct_illuminance < 0.01);
        assert!(
            !req.is_active,
            "Occluder must cut light and deactivate URequiresLight"
        );

        // Disable blocker and insert a red optical filter lens instead
        app.world_mut()
            .entity_mut(blocker)
            .get_mut::<ULightOccluder>()
            .unwrap()
            .enabled = false;
        let filter = app
            .world_mut()
            .spawn((
                UNode::default(),
                ComputedSize {
                    width: 400.0,
                    height: 400.0,
                    ..default()
                },
                ULightFilter::gel(Color::srgb(1.0, 0.1, 0.1)),
                Transform::from_xyz(0.0, 0.0, 1.5),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 1.5)),
            ))
            .id();

        app.world_mut()
            .run_system_once(update_light_sensors_and_requirements)
            .unwrap();

        let sensor = app.world().entity(sensor_entity).get::<ULightSensor>().unwrap();
        assert_eq!(sensor.filtered_by, Some(filter));
        assert!(sensor.dominant_color.red > sensor.dominant_color.blue * 3.0);
    }

    #[test]
    fn ulight_source_spawns_forward_facing_light_child_and_emissive_on_node_or_text() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, AssetPlugin::default()));

        let root = app
            .world_mut()
            .spawn((
                URootUi::world_3d(Vec2::new(600.0, 400.0)),
                ResolvedRootUi {
                    space: UiSpace::World3d,
                    ..default()
                },
            ))
            .id();

        let lamp_text = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UNode::default(),
                ComputedSize {
                    width: 240.0,
                    height: 60.0,
                    ..default()
                },
                UTextLabel {
                    text: "LAMP TEXT".into(),
                    ..default()
                },
                ULightSource::lamp(Color::srgb(0.2, 0.9, 1.0), 9_000.0),
            ))
            .id();

        app.world_mut()
            .run_system_once(sync_light_source_entities)
            .unwrap();

        let pbr = app
            .world()
            .entity(lamp_text)
            .get::<UPbr>()
            .expect("ULightSource must auto-attach emissive UPbr");
        assert!(pbr.emissive.blue > 1.0);

        let children = app.world().entity(lamp_text).get::<Children>().unwrap();
        let rect_child = children
            .iter()
            .find(|c| app.world().entity(*c).contains::<RectLight>())
            .expect("ULightSource::lamp must spawn RectLight child");
        let rect = app.world().entity(rect_child).get::<RectLight>().unwrap();
        assert!(rect.width > 0.2 && rect.height > 0.05);

        let tf = app.world().entity(rect_child).get::<Transform>().unwrap();
        // Child RectLight must be rotated 180 deg around Y so local -Z faces host +Z front!
        assert!(tf.forward().dot(Vec3::Z) > 0.95);
    }
}
