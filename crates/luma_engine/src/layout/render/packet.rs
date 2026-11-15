//! Decoupled visual render packet and plugin hooks for Univis UI nodes.
//!
//! [`VisualRenderPacket`] acts as the neutral data bridge between the core geometry/layout
//! engine (`UNode` + `ComputedSize` + `Transform`) and independent visual effect components
//! (such as [`UBorder`], [`UShadow`], [`UInnerGlow`], [`UGradient`], [`UImage`], [`UShader`],
//! or user-defined visual plugins).
//!
//! Because visual components push their parameters into [`VisualRenderPacket`] via Bevy ECS
//! lifecycle hooks (`on_insert` / `on_remove`) and lightweight `Changed<T>` sync systems,
//! the core layout cache, material synchronizer, and 2D batch extractor do not need to
//! query or track individual optional visual components.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use crate::layout::geometry::{UCornerRadius, UVal};
use crate::layout::image::UImage;
use crate::layout::invalidation::{UiInvalidateRequestQueue, UiLayoutInvalidation};
use crate::layout::lighting::{ULightOccluder, resolve_surface_lighting};
use crate::layout::pbr::{UPbr, UPbrSurface};
use crate::layout::univis_node::{UBorder, UNode};
use crate::layout::visual::{
    UGradient, UGradientInterpolation, UGradientKind, UInnerGlow, UShader, UShaderSource, UShadow,
};
use crate::schedule::{UiSettlementSchedule, UnivisPostUpdateSet};

/// Bitmask flags describing which visual effect channels are active in a [`VisualRenderPacket`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, PartialEq)]
pub struct VisualFlags(pub u32);

impl VisualFlags {
    /// No optional visual effects are active.
    pub const NONE: Self = Self(0);
    /// Active border stroke channel (`UBorder` or custom border plugin).
    pub const BORDER: Self = Self(1 << 0);
    /// Active outer drop shadow / outer glow channel (`UShadow` with `is_inner == false`).
    pub const OUTER_SHADOW: Self = Self(1 << 1);
    /// Active inner glow channel (`UInnerGlow` or `UShadow` with `is_inner == true`).
    pub const INNER_GLOW: Self = Self(1 << 2);
    /// Active multi-stop gradient fill channel (`UGradient`).
    pub const GRADIENT: Self = Self(1 << 3);
    /// Active image/texture fill channel (`UImage`).
    pub const TEXTURE: Self = Self(1 << 4);
    /// Active custom WGSL / asset fragment shader channel (`UShader`).
    pub const CUSTOM_SHADER: Self = Self(1 << 5);
    /// Forces the full `UNodeMaterial` / `UNodeMaterial3d` pipeline instead of 2D instanced batching.
    pub const FORCE_MATERIAL: Self = Self(1 << 6);
    /// Active 3D PBR material channel (`UPbr`).
    pub const PBR: Self = Self(1 << 7);

    /// Returns `true` if all bits in `other` are set in `self`.
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Inserts the bits of `other` into `self`.
    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Removes the bits of `other` from `self`.
    #[inline]
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Sets or clears `other` according to `enabled`.
    #[inline]
    pub fn set(&mut self, other: Self, enabled: bool) {
        if enabled {
            self.insert(other);
        } else {
            self.remove(other);
        }
    }

    /// Returns `true` if this node only uses visual channels supported by the single-draw 2D instanced batcher.
    #[inline]
    pub const fn can_batch_2d(self) -> bool {
        let non_batchable = Self::INNER_GLOW.0
            | Self::GRADIENT.0
            | Self::TEXTURE.0
            | Self::CUSTOM_SHADER.0
            | Self::FORCE_MATERIAL.0;
        (self.0 & non_batchable) == 0
    }
}

/// Neutral visual data packet attached to every [`UNode`].
///
/// Visual effect plugins populate this packet when their components are inserted, modified,
/// or removed. The core renderer (`update_materials_optimized` and `extract_univis_nodes`)
/// reads exclusively from [`VisualRenderPacket`] without coupling to concrete visual component types.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct VisualRenderPacket {
    /// Active visual effect flags.
    pub flags: VisualFlags,

    // --- Border Channel ---
    /// Linear RGBA border color.
    pub border_color: LinearRgba,
    /// Border stroke width in logical pixels.
    pub border_width: f32,
    /// Border offset in logical pixels.
    pub border_offset: f32,
    /// Optional border corner radius override when `UNode::border_radius` is zero.
    pub border_radius_override: Option<UCornerRadius>,

    // --- Outer Shadow Channel ---
    /// Linear RGBA outer shadow color.
    pub shadow_color: LinearRgba,
    /// Shadow offset in logical pixels.
    pub shadow_offset: Vec2,
    /// Shadow blur softness in logical pixels.
    pub shadow_blur: f32,
    /// Shadow spread distance in logical pixels.
    pub shadow_spread: f32,

    // --- Inner Glow Channel ---
    /// Linear RGBA inner glow color.
    pub inner_glow_color: LinearRgba,
    /// Inner glow blur radius in logical pixels.
    pub inner_glow_blur: f32,
    /// Inner glow spread distance in logical pixels.
    pub inner_glow_spread: f32,

    // --- Gradient Channel ---
    /// Up to 8 linear RGBA stop colors.
    pub grad_colors: [Vec4; 8],
    /// Packed stop positions (`[0..4]` and `[4..8]`).
    pub grad_stops: [Vec4; 2],
    /// `(grad_type, angle_or_radius, stop_count, interpolation_mode)`.
    pub grad_params: Vec4,
    /// `(center.x, center.y, offset, 0.0)`.
    pub grad_center: Vec4,

    // --- Texture / Image Channel ---
    /// Optional image texture handle.
    pub texture: Option<Handle<Image>>,
    /// Linear RGBA tint color for the texture.
    pub texture_tint: LinearRgba,

    // --- Custom Shader Channel ---
    /// Optional custom shader source.
    pub shader_source: Option<UShaderSource>,
    /// Four general-purpose `Vec4` uniform parameters (`params[0]..params[3]`).
    pub custom_params: [Vec4; 4],
    /// Encoded blend mode (`0 = Replace`, `1 = AlphaBlend`, `2 = Multiply`, `3 = Add`).
    pub custom_blend_mode: f32,
    /// Overall opacity multiplier (`0.0..=1.0`).
    pub custom_opacity: f32,
    /// Whether time advances automatically.
    pub custom_animate: bool,
    /// Animation speed multiplier.
    pub custom_speed: f32,
    /// Static time phase offset.
    pub custom_time_offset: f32,

    // --- 3D PBR & Surface Lighting Channel ---
    /// Surface metallic factor (`0.0..=1.0`).
    pub pbr_metallic: f32,
    /// Surface perceptual roughness (`0.0..=1.0`).
    pub pbr_roughness: f32,
    /// HDR emissive contribution (`LinearRgba` as `Vec4`).
    pub pbr_emissive: Vec4,
    /// Dielectric specular reflectance (`0.0..=1.0`).
    pub pbr_reflectance: f32,
    /// Packed lighting bitmask (`double_sided`, `receive_shadows`, `cast_shadows`).
    pub pbr_lighting_flags: u32,
    /// `(bevel_width_px, bevel_strength, alpha_cutoff, 0.0)`.
    pub pbr_bevel_params: Vec4,
    /// Resolved 3D `AlphaMode`.
    pub pbr_alpha_mode: AlphaMode,
}

impl Default for VisualRenderPacket {
    fn default() -> Self {
        let (pbr_reflectance, pbr_lighting_flags, pbr_bevel_params, pbr_alpha_mode) =
            resolve_surface_lighting(None, None);
        Self {
            flags: VisualFlags::NONE,
            border_color: LinearRgba::NONE,
            border_width: 0.0,
            border_offset: 0.0,
            border_radius_override: None,
            shadow_color: LinearRgba::NONE,
            shadow_offset: Vec2::ZERO,
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            inner_glow_color: LinearRgba::NONE,
            inner_glow_blur: 0.0,
            inner_glow_spread: 0.0,
            grad_colors: [Vec4::ZERO; 8],
            grad_stops: [Vec4::ZERO; 2],
            grad_params: Vec4::ZERO,
            grad_center: Vec4::ZERO,
            texture: None,
            texture_tint: LinearRgba::WHITE,
            shader_source: None,
            custom_params: [Vec4::ZERO; 4],
            custom_blend_mode: 0.0,
            custom_opacity: 1.0,
            custom_animate: false,
            custom_speed: 1.0,
            custom_time_offset: 0.0,
            pbr_metallic: 0.0,
            pbr_roughness: 0.5,
            pbr_emissive: Vec4::ZERO,
            pbr_reflectance,
            pbr_lighting_flags,
            pbr_bevel_params,
            pbr_alpha_mode,
        }
    }
}

impl VisualRenderPacket {
    /// Writes [`UBorder`] data into the packet.
    pub fn apply_border(&mut self, border: &UBorder) {
        self.border_color = LinearRgba::from(border.color);
        self.border_width = border.width;
        self.border_offset = border.offset;
        self.border_radius_override =
            (border.radius != UCornerRadius::default()).then_some(border.radius);
        self.flags.set(
            VisualFlags::BORDER,
            border.width > 0.0 || border.offset != 0.0 || border.radius != UCornerRadius::default(),
        );
    }

    /// Clears the border channel from the packet.
    pub fn clear_border(&mut self) {
        self.flags.remove(VisualFlags::BORDER);
        self.border_color = LinearRgba::NONE;
        self.border_width = 0.0;
        self.border_offset = 0.0;
        self.border_radius_override = None;
    }

    /// Writes [`UShadow`] data into the packet (supporting both outer drop shadows and inner shadows).
    pub fn apply_shadow(&mut self, shadow: &UShadow, has_explicit_inner_glow: bool) {
        if shadow.is_inner {
            self.flags.remove(VisualFlags::OUTER_SHADOW);
            self.shadow_color = LinearRgba::NONE;
            self.shadow_offset = Vec2::ZERO;
            self.shadow_blur = 0.0;
            self.shadow_spread = 0.0;

            if !has_explicit_inner_glow {
                self.inner_glow_color = LinearRgba::from(shadow.color);
                self.inner_glow_blur = shadow.blur;
                self.inner_glow_spread = shadow.spread;
                self.flags.insert(VisualFlags::INNER_GLOW);
            }
        } else {
            self.shadow_color = LinearRgba::from(shadow.color);
            self.shadow_offset = shadow.offset;
            self.shadow_blur = shadow.blur;
            self.shadow_spread = shadow.spread;
            self.flags.set(
                VisualFlags::OUTER_SHADOW,
                shadow.blur > 0.0 || shadow.spread > 0.0,
            );
            if !has_explicit_inner_glow {
                self.flags.remove(VisualFlags::INNER_GLOW);
                self.inner_glow_color = LinearRgba::NONE;
                self.inner_glow_blur = 0.0;
                self.inner_glow_spread = 0.0;
            }
        }
    }

    /// Clears the shadow channel from the packet.
    pub fn clear_shadow(&mut self, explicit_inner_glow: Option<&UInnerGlow>) {
        self.flags.remove(VisualFlags::OUTER_SHADOW);
        self.shadow_color = LinearRgba::NONE;
        self.shadow_offset = Vec2::ZERO;
        self.shadow_blur = 0.0;
        self.shadow_spread = 0.0;

        if let Some(ig) = explicit_inner_glow {
            self.apply_inner_glow(ig);
        } else {
            self.clear_inner_glow(None);
        }
    }

    /// Writes [`UInnerGlow`] data into the packet.
    pub fn apply_inner_glow(&mut self, glow: &UInnerGlow) {
        self.inner_glow_color = LinearRgba::from(glow.color);
        self.inner_glow_blur = glow.blur;
        self.inner_glow_spread = glow.spread;
        self.flags.insert(VisualFlags::INNER_GLOW);
    }

    /// Clears the inner glow channel (falling back to inner `UShadow` if present).
    pub fn clear_inner_glow(&mut self, fallback_shadow: Option<&UShadow>) {
        if let Some(s) = fallback_shadow
            && s.is_inner
        {
            self.inner_glow_color = LinearRgba::from(s.color);
            self.inner_glow_blur = s.blur;
            self.inner_glow_spread = s.spread;
            self.flags.insert(VisualFlags::INNER_GLOW);
        } else {
            self.flags.remove(VisualFlags::INNER_GLOW);
            self.inner_glow_color = LinearRgba::NONE;
            self.inner_glow_blur = 0.0;
            self.inner_glow_spread = 0.0;
        }
    }

    /// Writes [`UGradient`] data into the packet.
    pub fn apply_gradient(&mut self, grad: &UGradient) {
        let (grad_type, param_y, center) = match grad.kind {
            UGradientKind::Linear { angle } => (1.0, angle, Vec2::ZERO),
            UGradientKind::Radial { center, radius } => (2.0, radius, center),
            UGradientKind::Angular { center } => (3.0, 0.0, center),
        };
        let count = grad.stops.len().min(8);
        let mut colors = [Vec4::ZERO; 8];
        let mut positions = [0.0f32; 8];
        for (i, stop) in grad.stops.iter().take(8).enumerate() {
            colors[i] = LinearRgba::from(stop.color).to_vec4();
            positions[i] = stop.position;
        }
        let stops = [
            Vec4::new(positions[0], positions[1], positions[2], positions[3]),
            Vec4::new(positions[4], positions[5], positions[6], positions[7]),
        ];
        let interp_mode = match grad.interpolation {
            UGradientInterpolation::Smooth => 0.0,
            UGradientInterpolation::Stepped => 1.0,
        };
        self.grad_colors = colors;
        self.grad_stops = stops;
        self.grad_params = Vec4::new(grad_type, param_y, count as f32, interp_mode);
        self.grad_center = Vec4::new(center.x, center.y, grad.offset, 0.0);
        self.flags.insert(VisualFlags::GRADIENT);
    }

    /// Clears the gradient channel from the packet.
    pub fn clear_gradient(&mut self) {
        self.flags.remove(VisualFlags::GRADIENT);
        self.grad_colors = [Vec4::ZERO; 8];
        self.grad_stops = [Vec4::ZERO; 2];
        self.grad_params = Vec4::ZERO;
        self.grad_center = Vec4::ZERO;
    }

    /// Writes [`UImage`] data into the packet.
    pub fn apply_image(&mut self, image: &UImage) {
        self.texture = Some(image.texture.clone());
        self.texture_tint = LinearRgba::from(image.color);
        if let Some(radius) = image.radius {
            self.border_radius_override = Some(radius);
        }
        self.flags.insert(VisualFlags::TEXTURE);
    }

    /// Clears the image/texture channel from the packet.
    pub fn clear_image(&mut self) {
        self.flags.remove(VisualFlags::TEXTURE);
        self.texture = None;
        self.texture_tint = LinearRgba::WHITE;
    }

    /// Writes 3D [`UPbr`], [`UPbrSurface`], and [`ULightOccluder`] data into the packet.
    pub fn apply_pbr_lighting(
        &mut self,
        pbr_opt: Option<&UPbr>,
        surface_opt: Option<&UPbrSurface>,
        occluder_opt: Option<&ULightOccluder>,
    ) {
        if let Some(pbr) = pbr_opt {
            self.flags.insert(VisualFlags::PBR);
            self.pbr_metallic = pbr.metallic;
            self.pbr_roughness = pbr.roughness;
            self.pbr_emissive = pbr.emissive.to_vec4();
        } else {
            self.flags.remove(VisualFlags::PBR);
            self.pbr_metallic = 0.0;
            self.pbr_roughness = 0.5;
            self.pbr_emissive = Vec4::ZERO;
        }
        let (refl, flags, bevel, alpha_mode) = resolve_surface_lighting(surface_opt, occluder_opt);
        self.pbr_reflectance = refl;
        self.pbr_lighting_flags = flags;
        self.pbr_bevel_params = bevel;
        self.pbr_alpha_mode = alpha_mode;
    }

    /// Writes [`UShader`] data into the packet.
    pub fn apply_shader(&mut self, shader: &UShader) {
        self.shader_source = Some(shader.source.clone());
        self.custom_params = shader.params;
        self.custom_blend_mode = shader.blend_mode.as_f32();
        self.custom_opacity = shader.opacity;
        self.custom_animate = shader.animate;
        self.custom_speed = shader.speed;
        self.custom_time_offset = shader.time_offset;
        self.flags.insert(VisualFlags::CUSTOM_SHADER);
    }

    /// Clears the custom shader channel from the packet.
    pub fn clear_shader(&mut self) {
        self.flags.remove(VisualFlags::CUSTOM_SHADER);
        self.shader_source = None;
        self.custom_params = [Vec4::ZERO; 4];
        self.custom_blend_mode = 0.0;
        self.custom_opacity = 1.0;
        self.custom_animate = false;
        self.custom_speed = 1.0;
        self.custom_time_offset = 0.0;
    }

    /// Resolves the effective corner radius combining `UNode::border_radius` with optional border override.
    #[inline]
    pub fn effective_corner_radius(&self, node_radius: UCornerRadius) -> UCornerRadius {
        if node_radius != UCornerRadius::default() {
            node_radius
        } else {
            self.border_radius_override.unwrap_or(node_radius)
        }
    }
}

/// Public helper for any built-in or third-party visual component hook to notify the UI engine
/// that an entity's visual state changed and needs render synchronization.
pub fn notify_visual_packet_dirty(world: &mut DeferredWorld, entity: Entity) {
    if !world.contains_resource::<crate::internal::MaterialPool>() {
        return;
    }
    if let Some(mut queue) = world.get_resource_mut::<UiInvalidateRequestQueue>() {
        queue.request(
            entity,
            UiLayoutInvalidation {
                render_self: true,
                ..default()
            },
        );
    }
}

// ============================================================================
// Component Lifecycle Hooks (Zero-polling insertion & removal handlers)
// ============================================================================

pub(crate) fn on_border_insert(mut world: DeferredWorld, ctx: HookContext) {
    let Some(border) = world.get::<UBorder>(ctx.entity).cloned() else {
        return;
    };
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_border(&border);
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_border_remove(mut world: DeferredWorld, ctx: HookContext) {
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.clear_border();
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_shadow_insert(mut world: DeferredWorld, ctx: HookContext) {
    let Some(shadow) = world.get::<UShadow>(ctx.entity).copied() else {
        return;
    };
    let has_inner_glow = world.get::<UInnerGlow>(ctx.entity).is_some();
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_shadow(&shadow, has_inner_glow);
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_shadow_remove(mut world: DeferredWorld, ctx: HookContext) {
    let explicit_ig = world.get::<UInnerGlow>(ctx.entity).copied();
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.clear_shadow(explicit_ig.as_ref());
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_inner_glow_insert(mut world: DeferredWorld, ctx: HookContext) {
    let Some(glow) = world.get::<UInnerGlow>(ctx.entity).copied() else {
        return;
    };
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_inner_glow(&glow);
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_inner_glow_remove(mut world: DeferredWorld, ctx: HookContext) {
    let fallback_shadow = world.get::<UShadow>(ctx.entity).copied();
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.clear_inner_glow(fallback_shadow.as_ref());
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_gradient_insert(mut world: DeferredWorld, ctx: HookContext) {
    let Some(grad) = world.get::<UGradient>(ctx.entity).cloned() else {
        return;
    };
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_gradient(&grad);
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_gradient_remove(mut world: DeferredWorld, ctx: HookContext) {
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.clear_gradient();
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_image_insert(mut world: DeferredWorld, ctx: HookContext) {
    let Some(image) = world.get::<UImage>(ctx.entity).cloned() else {
        return;
    };
    if let Some(mut node) = world.get_mut::<UNode>(ctx.entity) {
        if image.width != UVal::Auto && node.width == UVal::Auto {
            node.width = image.width;
        }
        if image.height != UVal::Auto && node.height == UVal::Auto {
            node.height = image.height;
        }
        if let Some(radius) = image.radius
            && node.border_radius == UCornerRadius::default()
        {
            node.border_radius = radius;
        }
    }
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_image(&image);
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_image_remove(mut world: DeferredWorld, ctx: HookContext) {
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.clear_image();
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_pbr_insert(mut world: DeferredWorld, ctx: HookContext) {
    let pbr = world.get::<UPbr>(ctx.entity).cloned();
    let surf = world.get::<UPbrSurface>(ctx.entity).cloned();
    let occl = world.get::<ULightOccluder>(ctx.entity).cloned();
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_pbr_lighting(pbr.as_ref(), surf.as_ref(), occl.as_ref());
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_pbr_remove(mut world: DeferredWorld, ctx: HookContext) {
    let surf = world.get::<UPbrSurface>(ctx.entity).cloned();
    let occl = world.get::<ULightOccluder>(ctx.entity).cloned();
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_pbr_lighting(None, surf.as_ref(), occl.as_ref());
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_pbr_surface_remove(mut world: DeferredWorld, ctx: HookContext) {
    let pbr = world.get::<UPbr>(ctx.entity).cloned();
    let occl = world.get::<ULightOccluder>(ctx.entity).cloned();
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_pbr_lighting(pbr.as_ref(), None, occl.as_ref());
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_occluder_remove(mut world: DeferredWorld, ctx: HookContext) {
    let pbr = world.get::<UPbr>(ctx.entity).cloned();
    let surf = world.get::<UPbrSurface>(ctx.entity).cloned();
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_pbr_lighting(pbr.as_ref(), surf.as_ref(), None);
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_shader_insert(mut world: DeferredWorld, ctx: HookContext) {
    let Some(shader) = world.get::<UShader>(ctx.entity).cloned() else {
        return;
    };
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.apply_shader(&shader);
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

pub(crate) fn on_shader_remove(mut world: DeferredWorld, ctx: HookContext) {
    if let Some(mut packet) = world.get_mut::<VisualRenderPacket>(ctx.entity) {
        packet.clear_shader();
    }
    notify_visual_packet_dirty(&mut world, ctx.entity);
}

// ============================================================================
// Modular Push Systems for In-Place Component Mutations (`Changed<T>`)
// ============================================================================

/// Synchronizes in-place [`UBorder`] mutations into [`VisualRenderPacket`].
pub fn sync_border_visual_packet(
    mut query: Query<
        (&UBorder, &mut VisualRenderPacket),
        Or<(Changed<UBorder>, Added<VisualRenderPacket>)>,
    >,
) {
    for (border, mut packet) in &mut query {
        packet.apply_border(border);
    }
}

/// Synchronizes in-place [`UShadow`] mutations into [`VisualRenderPacket`].
pub fn sync_shadow_visual_packet(
    mut query: Query<
        (&UShadow, Option<&UInnerGlow>, &mut VisualRenderPacket),
        Or<(Changed<UShadow>, Added<VisualRenderPacket>)>,
    >,
) {
    for (shadow, inner_glow_opt, mut packet) in &mut query {
        packet.apply_shadow(shadow, inner_glow_opt.is_some());
    }
}

/// Synchronizes in-place [`UInnerGlow`] mutations into [`VisualRenderPacket`].
pub fn sync_inner_glow_visual_packet(
    mut query: Query<
        (&UInnerGlow, &mut VisualRenderPacket),
        Or<(Changed<UInnerGlow>, Added<VisualRenderPacket>)>,
    >,
) {
    for (glow, mut packet) in &mut query {
        packet.apply_inner_glow(glow);
    }
}

/// Synchronizes in-place [`UGradient`] mutations into [`VisualRenderPacket`].
pub fn sync_gradient_visual_packet(
    mut query: Query<
        (&UGradient, &mut VisualRenderPacket),
        Or<(Changed<UGradient>, Added<VisualRenderPacket>)>,
    >,
) {
    for (grad, mut packet) in &mut query {
        packet.apply_gradient(grad);
    }
}

/// Synchronizes in-place [`UImage`] mutations into [`VisualRenderPacket`] and [`UNode`].
pub fn sync_image_visual_packet(
    mut query: Query<
        (&UImage, &mut UNode, &mut VisualRenderPacket),
        Or<(Changed<UImage>, Added<VisualRenderPacket>)>,
    >,
) {
    for (image, mut node, mut packet) in &mut query {
        if image.width != UVal::Auto && node.width == UVal::Auto {
            node.width = image.width;
        }
        if image.height != UVal::Auto && node.height == UVal::Auto {
            node.height = image.height;
        }
        if let Some(radius) = image.radius
            && node.border_radius == UCornerRadius::default()
        {
            node.border_radius = radius;
        }
        packet.apply_image(image);
    }
}

/// Synchronizes in-place [`UPbr`], [`UPbrSurface`], and [`ULightOccluder`] mutations into [`VisualRenderPacket`].
pub fn sync_pbr_visual_packet(
    mut query: Query<
        (
            Option<&UPbr>,
            Option<&UPbrSurface>,
            Option<&ULightOccluder>,
            &mut VisualRenderPacket,
        ),
        Or<(
            Changed<UPbr>,
            Changed<UPbrSurface>,
            Changed<ULightOccluder>,
            Added<VisualRenderPacket>,
        )>,
    >,
) {
    for (pbr_opt, surface_opt, occluder_opt, mut packet) in &mut query {
        if pbr_opt.is_some() || surface_opt.is_some() || occluder_opt.is_some() {
            packet.apply_pbr_lighting(pbr_opt, surface_opt, occluder_opt);
        }
    }
}

/// Synchronizes in-place [`UShader`] mutations into [`VisualRenderPacket`].
pub fn sync_shader_visual_packet(
    mut query: Query<
        (&UShader, &mut VisualRenderPacket),
        Or<(Changed<UShader>, Added<VisualRenderPacket>)>,
    >,
) {
    for (shader, mut packet) in &mut query {
        packet.apply_shader(shader);
    }
}

/// Modular plugin that installs the decoupled visual effect push systems (`UBorder`, `UShadow`,
/// `UInnerGlow`, `UGradient`, `UImage`, `UShader`, `UPbr`, `UPbrSurface`, `ULightOccluder`).
///
/// External crates can follow this exact pattern to register custom visual effect plugins
/// that write into [`VisualRenderPacket`] without modifying core `UNode` systems.
pub struct UnivisVisualEffectsPlugin;

impl Plugin for UnivisVisualEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<VisualFlags>()
            .register_type::<VisualRenderPacket>()
            .add_systems(
                UiSettlementSchedule,
                (
                    sync_border_visual_packet,
                    sync_shadow_visual_packet,
                    sync_inner_glow_visual_packet,
                    sync_gradient_visual_packet,
                    sync_image_visual_packet,
                    sync_shader_visual_packet,
                    sync_pbr_visual_packet,
                )
                    .in_set(UnivisPostUpdateSet::ExternalPrepare),
            );
    }
}
