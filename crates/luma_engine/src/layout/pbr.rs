use bevy::prelude::*;

/// Physically based material overrides for `World3d` UI content.
///
/// Works on both [`UNode`](crate::layout::univis_node::UNode) panels and
/// [`UText`](crate::layout::text::UText) / [`UTextLabel`](crate::layout::text::UTextLabel) typography.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
#[component(
    on_insert = crate::layout::render::packet::on_pbr_insert,
    on_remove = crate::layout::render::packet::on_pbr_remove
)]
pub struct UPbr {
    /// Surface metallic factor, where `0.0` is dielectric and `1.0` is metallic.
    pub metallic: f32,

    /// Surface roughness, where `0.0` is mirror-like and `1.0` is very rough.
    pub roughness: f32,

    /// Emissive contribution for self-lit UI surfaces.
    pub emissive: LinearRgba,
}

impl Default for UPbr {
    fn default() -> Self {
        Self {
            metallic: 0.0,
            roughness: 0.5,
            emissive: LinearRgba::BLACK,
        }
    }
}

impl UPbr {
    /// Creates a dielectric or metallic PBR material without emission.
    pub fn new(metallic: f32, roughness: f32) -> Self {
        Self {
            metallic,
            roughness,
            emissive: LinearRgba::BLACK,
        }
    }

    /// Matte wood-like organic dielectric surface (zero emission; darkens completely without light).
    pub fn wood() -> Self {
        Self::new(0.0, 0.88)
    }

    /// Rough matte architectural plaster/cardboard surface.
    pub fn matte() -> Self {
        Self::new(0.0, 0.94)
    }

    /// Textured stone/slate surface with subtle micro-roughness.
    pub fn stone() -> Self {
        Self::new(0.0, 0.80)
    }

    /// Smooth satin architectural finish.
    pub fn satin() -> Self {
        Self::new(0.0, 0.42)
    }

    /// Satin brushed aluminum/steel surface.
    pub fn brushed_metal() -> Self {
        Self::new(0.88, 0.28)
    }

    /// Mirror-like polished chrome finish.
    pub fn chrome() -> Self {
        Self::new(1.0, 0.06)
    }

    /// Warm metallic gold finish.
    pub fn gold() -> Self {
        Self::new(1.0, 0.18)
    }

    /// Smooth translucent dielectric glass preset (pair with translucent background alpha).
    pub fn glass() -> Self {
        Self::new(0.0, 0.06)
    }

    /// Self-illuminated lamp surface with HDR emissive intensity (compatible with Bloom).
    pub fn lamp(color: Color, intensity: f32) -> Self {
        let lin = LinearRgba::from(color);
        let boost = intensity.max(0.0);
        Self {
            metallic: 0.0,
            roughness: 0.25,
            emissive: LinearRgba::rgb(lin.red * boost, lin.green * boost, lin.blue * boost),
        }
    }

    /// High-contrast neon emissive material for signage, borders, and emissive text.
    pub fn neon(color: Color, intensity: f32) -> Self {
        Self::lamp(color, intensity)
    }

    /// Attaches an emissive color to this PBR material.
    pub fn with_emissive(mut self, emissive: LinearRgba) -> Self {
        self.emissive = emissive;
        self
    }

    /// Sets the metallic factor (`0.0..=1.0`).
    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic;
        self
    }

    /// Sets the perceptual roughness factor (`0.0..=1.0`).
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }
}

/// Extended physical surface and shadow controls for `World3d` UI nodes and text.
///
/// Attaching [`UPbrSurface`] to an entity automatically enables PBR shading even if
/// [`UPbr`] wasn't explicitly added alongside it.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
#[component(
    on_insert = crate::layout::render::packet::on_pbr_insert,
    on_remove = crate::layout::render::packet::on_pbr_surface_remove
)]
pub struct UPbrSurface {
    /// Specular reflectance for dielectrics (`0.0..=1.0`, Bevy default is `0.5`).
    pub reflectance: f32,

    /// Whether back faces flip normals and receive lighting equally (`true` by default).
    pub double_sided: bool,

    /// Whether this surface receives real-time shadows from scene lights (`true` by default).
    pub receive_shadows: bool,

    /// Whether this UI node or text casts SDF-accurate silhouette shadows (`false` by default).
    pub cast_shadows: bool,

    /// Optional explicit Bevy [`AlphaMode`] override (`None` keeps smooth `AlphaMode::Premultiplied`).
    pub alpha_mode: Option<AlphaMode>,

    /// Alpha cutoff threshold used during shadow casting prepasses and [`AlphaMode::Mask`].
    pub alpha_cutoff: f32,

    /// Procedural 3D edge bevel width in logical UI pixels (`0.0` disables bevel normal tilting).
    pub bevel_width: f32,

    /// Procedural 3D edge bevel slope strength (`0.0..=1.0`).
    pub bevel_strength: f32,
}

impl Default for UPbrSurface {
    fn default() -> Self {
        Self {
            reflectance: 0.5,
            double_sided: true,
            receive_shadows: true,
            cast_shadows: false,
            alpha_mode: None,
            alpha_cutoff: 0.5,
            bevel_width: 0.0,
            bevel_strength: 0.0,
        }
    }
}

impl UPbrSurface {
    /// Matte/solid shadow-receiving surface with crisp 3D bevel highlights enabled.
    pub fn beveled(width_px: f32, strength: f32) -> Self {
        Self {
            bevel_width: width_px.max(0.0),
            bevel_strength: strength.clamp(0.0, 1.0),
            ..default()
        }
    }

    /// Solid shadow-casting and shadow-receiving surface preset.
    pub fn solid() -> Self {
        Self {
            cast_shadows: true,
            receive_shadows: true,
            double_sided: true,
            ..default()
        }
    }

    /// Wooden plank surface preset with subtle carved bevels and shadow interaction.
    pub fn wood_plank() -> Self {
        Self {
            reflectance: 0.35,
            cast_shadows: true,
            receive_shadows: true,
            bevel_width: 6.0,
            bevel_strength: 0.55,
            ..default()
        }
    }

    /// Translucent optical glass surface with elevated Fresnel reflectance.
    pub fn glass() -> Self {
        Self {
            reflectance: 0.9,
            double_sided: true,
            receive_shadows: true,
            cast_shadows: false,
            ..default()
        }
    }

    /// Enables or disables real-time shadow casting for this SDF shape/text.
    pub fn with_cast_shadows(mut self, cast_shadows: bool) -> Self {
        self.cast_shadows = cast_shadows;
        self
    }

    /// Enables or disables real-time shadow reception for this node/text.
    pub fn with_receive_shadows(mut self, receive_shadows: bool) -> Self {
        self.receive_shadows = receive_shadows;
        self
    }

    /// Configures procedural 3D edge bevel normal tilting.
    pub fn with_bevel(mut self, width_px: f32, strength: f32) -> Self {
        self.bevel_width = width_px.max(0.0);
        self.bevel_strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Sets dielectric specular reflectance (`0.0..=1.0`).
    pub fn with_reflectance(mut self, reflectance: f32) -> Self {
        self.reflectance = reflectance.clamp(0.0, 1.0);
        self
    }

    /// Controls double-sided normal flipping (`true` lights both front and back faces).
    pub fn with_double_sided(mut self, double_sided: bool) -> Self {
        self.double_sided = double_sided;
        self
    }

    /// Overrides the [`AlphaMode`] used by [`UNodeMaterial3d`](crate::layout::render::material_3d::UNodeMaterial3d).
    pub fn with_alpha_mode(mut self, alpha_mode: AlphaMode) -> Self {
        self.alpha_mode = Some(alpha_mode);
        self
    }

    /// Sets the alpha cutoff threshold (`0.0..=1.0`) for cutout shadows and `AlphaMode::Mask`.
    pub fn with_alpha_cutoff(mut self, alpha_cutoff: f32) -> Self {
        self.alpha_cutoff = alpha_cutoff.clamp(0.01, 0.99);
        self
    }
}

/// Opt-in marker on a descendant UI node or text to inherit [`UPbr`] and [`UPbrSurface`]
/// from its nearest ancestor carrying them (or carrying [`UPbrPropagate`]).
#[derive(Component, Reflect, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct UPbrInherit;

/// Cascades this entity's [`UPbr`] and optional [`UPbrSurface`] down to all descendant
/// UI nodes and text that do not explicitly opt out via [`UPbrKeepUnlit`].
#[derive(Component, Reflect, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct UPbrPropagate;

/// Prevents an entity from receiving propagated PBR overrides from an ancestor [`UPbrPropagate`].
#[derive(Component, Reflect, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct UPbrKeepUnlit;

/// Internal marker recording that [`UPbr`] was inherited automatically rather than authored directly.
#[derive(Component, Debug, Default)]
pub(crate) struct InheritedPbrMarker;

/// Internal marker recording that [`UPbrSurface`] was inherited automatically rather than authored directly.
#[derive(Component, Debug, Default)]
pub(crate) struct InheritedPbrSurfaceMarker;
