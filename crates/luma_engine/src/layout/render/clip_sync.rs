//! Hardware SDF clipping synchronization and shared geometry helpers.
//!
//! Resolves up to [`MAX_CLIP_LAYERS`] nested [`UClip`] ancestor containers using
//! their inverse `GlobalTransform` matrices so rotated, scaled, and 3D world-space
//! UI elements share exact clip math across rendering, picking, and scrolling.

use bevy::camera::{NormalizedRenderTarget, RenderTarget};
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::layout::components::CachedUiContext;
use crate::layout::geometry::ComputedSize;
use crate::layout::layout_system::ResolvedRootUi;
use crate::layout::query::UiPickingContext;
use crate::layout::render::material::UNodeMaterial;
use crate::layout::render::material_3d::UNodeMaterial3d;
use crate::layout::univis_node::{UClip, UNode};
use crate::prelude::UShapeMode;

/// Maximum number of nested [`UClip`] ancestor layers evaluated per shader pass.
pub const MAX_CLIP_LAYERS: usize = 4;

/// GPU-ready multi-layer clip uniform payload computed from up to [`MAX_CLIP_LAYERS`] ancestors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClipUniformData {
    /// Row 0 of each clipper's inverse `GlobalTransform` matrix (`[x, y, z, w]`).
    pub clip_inv_x: [Vec4; MAX_CLIP_LAYERS],
    /// Row 1 of each clipper's inverse `GlobalTransform` matrix (`[x, y, z, w]`).
    pub clip_inv_y: [Vec4; MAX_CLIP_LAYERS],
    /// `[half_width, half_height, active (1.0/0.0), shape_mode (0.0=round, 1.0=cut)]` per layer.
    pub clip_bounds: [Vec4; MAX_CLIP_LAYERS],
    /// Corner radii `(top_right, bottom_right, top_left, bottom_left)` per layer in clipper-local space.
    pub clip_radius: [Vec4; MAX_CLIP_LAYERS],
    /// Intersected world-XY center for simple unrotated 2D batching fallback.
    pub aabb_center: Vec2,
    /// Intersected world-XY size for simple unrotated 2D batching fallback.
    pub aabb_size: Vec2,
    /// Radius of the nearest unrotated clipper for simple 2D batching fallback.
    pub aabb_radius: Vec4,
    /// Number of active clip layers (`0..=MAX_CLIP_LAYERS`).
    pub layer_count: usize,
    /// True when clipping involves rotation, non-unit scale, cut corners, or multi-layer nesting.
    pub is_complex_for_batch: bool,
}

impl ClipUniformData {
    /// Returns `true` when at least one active clip layer applies.
    pub fn is_active(&self) -> bool {
        self.layer_count > 0
    }
}

/// Trait implemented by material assets that consume multi-layer shader clipping uniforms.
pub(crate) trait ClipUniformTarget {
    fn clip_fields(
        &self,
    ) -> (
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
    );

    fn clip_fields_mut(
        &mut self,
    ) -> (
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
    );

    fn matches_clip_uniforms(&self, data: &ClipUniformData) -> bool {
        let (inv_x, inv_y, bounds, radius) = self.clip_fields();
        *inv_x == data.clip_inv_x
            && *inv_y == data.clip_inv_y
            && *bounds == data.clip_bounds
            && *radius == data.clip_radius
    }

    fn apply_clip_uniforms(&mut self, data: &ClipUniformData) -> bool {
        if self.matches_clip_uniforms(data) {
            return false;
        }
        let (inv_x, inv_y, bounds, radius) = self.clip_fields_mut();
        *inv_x = data.clip_inv_x;
        *inv_y = data.clip_inv_y;
        *bounds = data.clip_bounds;
        *radius = data.clip_radius;
        true
    }
}

impl ClipUniformTarget for UNodeMaterial {
    fn clip_fields(
        &self,
    ) -> (
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
    ) {
        (
            &self.clip_inv_x,
            &self.clip_inv_y,
            &self.clip_bounds,
            &self.clip_radius,
        )
    }

    fn clip_fields_mut(
        &mut self,
    ) -> (
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
    ) {
        (
            &mut self.clip_inv_x,
            &mut self.clip_inv_y,
            &mut self.clip_bounds,
            &mut self.clip_radius,
        )
    }
}

impl ClipUniformTarget for UNodeMaterial3d {
    fn clip_fields(
        &self,
    ) -> (
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
        &[Vec4; MAX_CLIP_LAYERS],
    ) {
        (
            &self.clip_inv_x,
            &self.clip_inv_y,
            &self.clip_bounds,
            &self.clip_radius,
        )
    }

    fn clip_fields_mut(
        &mut self,
    ) -> (
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
        &mut [Vec4; MAX_CLIP_LAYERS],
    ) {
        (
            &mut self.clip_inv_x,
            &mut self.clip_inv_y,
            &mut self.clip_bounds,
            &mut self.clip_radius,
        )
    }
}

/// Evaluates the signed distance for a rounded box centered at the origin.
///
/// Matches shader corner ordering: `(top_right, bottom_right, top_left, bottom_left)`.
pub fn sd_rounded_box(p: Vec2, b: Vec2, r: Vec4) -> f32 {
    let (r_top, r_bottom) = if p.x > 0.0 { (r.x, r.y) } else { (r.z, r.w) };
    let radius = if p.y > 0.0 { r_top } else { r_bottom };
    let q = p.abs() - b + Vec2::splat(radius);
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - radius
}

/// Evaluates the signed distance for a cut/chamfered box centered at the origin.
pub fn sd_cut_box(p: Vec2, b: Vec2, r: Vec4) -> f32 {
    let (r_top, r_bottom) = if p.x > 0.0 { (r.x, r.y) } else { (r.z, r.w) };
    let radius = if p.y > 0.0 { r_top } else { r_bottom };
    let q = p.abs() - b;
    let d_box = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0);
    let d_cut =
        (p.x.abs() + p.y.abs() - (b.x + b.y - radius)) * std::f32::consts::FRAC_1_SQRT_2;
    d_box.max(d_cut)
}

/// Evaluates the signed distance for a UI node shape (`Round` or `Cut`).
pub fn sd_node_shape(p: Vec2, b: Vec2, r: Vec4, shape_mode: UShapeMode) -> f32 {
    match shape_mode {
        UShapeMode::Round => sd_rounded_box(p, b, r),
        UShapeMode::Cut => sd_cut_box(p, b, r),
    }
}

/// Intersects a 3D world ray with a UI node's local `z = 0` plane.
///
/// Returns `(world_hit, local_hit_xy, plane_normal, ray_distance)` when the ray hits
/// on or in front of `ray_origin` and the transform is invertible.
pub fn intersect_ray_with_node_plane(
    ray_origin: Vec3,
    ray_direction: Vec3,
    global_transform: &GlobalTransform,
) -> Option<(Vec3, Vec2, Vec3, f32)> {
    let plane_origin = global_transform.translation();
    let z_axis = global_transform.affine().matrix3.z_axis;
    let plane_normal = Dir3::new(z_axis.into())
        .map(|d| d.as_vec3())
        .unwrap_or(Vec3::Z);
    let denominator = ray_direction.dot(plane_normal);

    if denominator.abs() <= f32::EPSILON {
        return None;
    }

    let distance = (plane_origin - ray_origin).dot(plane_normal) / denominator;
    if distance < 0.0 {
        return None;
    }

    let matrix = global_transform.to_matrix();
    if matrix.determinant().abs() <= 1e-12 {
        return None;
    }

    let world_hit = ray_origin + ray_direction * distance;
    let local_hit = matrix.inverse().transform_point3(world_hit).truncate();
    Some((world_hit, local_hit, plane_normal, distance))
}

/// Returns `true` when `entity` is visible along its `ChildOf` hierarchy.
///
/// Walking `Visibility` up `ChildOf` avoids relying on unpropagated or previous-frame
/// [`InheritedVisibility`] during `PreUpdate` picking or headless tests without `VisibilityPlugin`.
pub fn is_entity_visible_in_hierarchy(
    entity: Entity,
    visibility_query: &Query<&Visibility>,
    parents_query: &Query<&ChildOf>,
) -> bool {
    let mut current = entity;
    loop {
        if let Ok(visibility) = visibility_query.get(current) {
            match visibility {
                Visibility::Hidden => return false,
                Visibility::Visible => return true,
                Visibility::Inherited => {}
            }
        }
        let Ok(parent) = parents_query.get(current) else {
            return true;
        };
        current = parent.parent();
    }
}

/// Resolves the UI-units-to-world scale multiplier for `entity` by walking to its [`ResolvedRootUi`].
pub fn resolve_world_scale_for_entity(
    start_entity: Entity,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
) -> f32 {
    let mut current = start_entity;
    loop {
        if let Ok(root) = root_query.get(current) {
            return root.ui_units_to_world_scale();
        }
        let Ok(parent) = parents_query.get(current) else {
            return 1.0;
        };
        current = parent.parent();
    }
}

/// Converts a world-space hit point on a UI node into node-local logical coordinates (`+Y` up, unscaled by `world_scale`).
pub fn hit_world_to_local_logical(
    hit_world: Vec3,
    global_transform: &GlobalTransform,
    world_scale: f32,
) -> Vec2 {
    let safe_scale = if world_scale.abs() > f32::EPSILON {
        world_scale
    } else {
        1.0
    };
    global_transform
        .to_matrix()
        .inverse()
        .transform_point3(hit_world)
        .truncate()
        / safe_scale
}

/// Resolves the positive logical-to-world scale multiplier for `entity`, preferring [`UiPickingContext`].
pub fn resolve_node_world_scale(
    entity: Entity,
    picking_ctx: Option<&UiPickingContext>,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
) -> f32 {
    let scale = picking_ctx.map_or_else(
        || resolve_world_scale_for_entity(entity, parents_query, root_query),
        |ctx| ctx.world_scale,
    );
    if scale.abs() > f32::EPSILON {
        scale
    } else {
        1.0
    }
}

/// Resolves the [`PrimaryWindow`] entity, or falls back to the sole window when unambiguous.
pub fn resolve_primary_window_entity<I>(windows: I) -> Option<Entity>
where
    I: IntoIterator<Item = (Entity, bool)>,
{
    let mut first: Option<Entity> = None;
    let mut count = 0usize;
    for (entity, is_primary) in windows {
        if is_primary {
            return Some(entity);
        }
        if count == 0 {
            first = Some(entity);
        }
        count += 1;
    }
    if count == 1 { first } else { None }
}

/// Returns `true` when `render_target` targets `window_entity`.
pub fn camera_matches_window(
    render_target: Option<&RenderTarget>,
    window_entity: Entity,
    primary_window: Option<Entity>,
) -> bool {
    match render_target {
        Some(rt) => matches!(
            rt.normalize(primary_window),
            Some(NormalizedRenderTarget::Window(win_ref)) if win_ref.entity() == window_entity
        ),
        None => primary_window == Some(window_entity),
    }
}

/// Returns `true` when `render_target` resolves to `normalized_target`.
pub fn camera_matches_normalized_target(
    render_target: Option<&RenderTarget>,
    normalized_target: &NormalizedRenderTarget,
    primary_window: Option<Entity>,
) -> bool {
    match render_target {
        Some(rt) => rt.normalize(primary_window).as_ref() == Some(normalized_target),
        None => matches!(
            normalized_target,
            NormalizedRenderTarget::Window(win_ref) if Some(win_ref.entity()) == primary_window
        ),
    }
}

/// Returns `true` when `render_target` matches a pointer's `normalized_target`,
/// including headless test setups with no windows and `NormalizedRenderTarget::None`.
pub fn camera_matches_pointer_target(
    render_target: Option<&RenderTarget>,
    normalized_target: &NormalizedRenderTarget,
    primary_window: Option<Entity>,
    has_any_window: bool,
) -> bool {
    if camera_matches_normalized_target(render_target, normalized_target, primary_window) {
        return true;
    }
    !has_any_window
        && matches!(normalized_target, NormalizedRenderTarget::None { .. })
        && matches!(
            render_target,
            None | Some(RenderTarget::Window(bevy::window::WindowRef::Primary))
        )
}

/// Projects `screen_pos` from `camera` onto `node_gt`'s plane, returning `(order, world_hit, local_hit, distance)`.
pub fn ray_hit_for_camera(
    camera: &Camera,
    cam_gt: &GlobalTransform,
    screen_pos: Vec2,
    node_gt: &GlobalTransform,
    require_viewport_rect: bool,
) -> Option<(f32, Vec3, Vec2, f32)> {
    if !camera.is_active {
        return None;
    }
    if require_viewport_rect
        && !camera
            .logical_viewport_rect()
            .is_some_and(|rect| rect.contains(screen_pos))
    {
        return None;
    }
    let ray = camera.viewport_to_world(cam_gt, screen_pos).ok()?;
    let (hit_world, local_hit, _, hit_distance) =
        intersect_ray_with_node_plane(ray.origin, ray.direction.as_vec3(), node_gt)?;
    Some((camera.order as f32, hit_world, local_hit, hit_distance))
}

/// Projects `screen_pos` onto `node_gt`'s plane using `preferred_camera` or sorted active cameras matching `target`/`window_entity`.
///
/// Returns `(camera_entity, camera_order, hit_world, local_hit_logical, hit_distance)`.
#[allow(clippy::too_many_arguments)]
pub fn project_screen_point_to_node(
    screen_pos: Vec2,
    normalized_target: Option<&NormalizedRenderTarget>,
    window_entity: Option<Entity>,
    primary_window: Option<Entity>,
    preferred_camera: Option<Entity>,
    node_gt: &GlobalTransform,
    world_scale: f32,
    require_viewport_rect: bool,
    cameras: &Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
) -> Option<(Entity, f32, Vec3, Vec2, f32)> {
    let safe_scale = if world_scale.abs() > f32::EPSILON {
        world_scale
    } else {
        1.0
    };
    let matches_target = |rt_opt: Option<&RenderTarget>| -> bool {
        if let Some(target) = normalized_target {
            camera_matches_normalized_target(rt_opt, target, primary_window)
        } else if let Some(win_entity) = window_entity {
            camera_matches_window(rt_opt, win_entity, primary_window)
        } else {
            true
        }
    };

    if let Some(cam_entity) = preferred_camera {
        let (_, camera, cam_gt, rt_opt) = cameras.get(cam_entity).ok()?;
        if !matches_target(rt_opt) {
            return None;
        }
        let (order, hit_world, local_hit, distance) =
            ray_hit_for_camera(camera, cam_gt, screen_pos, node_gt, require_viewport_rect)?;
        return Some((cam_entity, order, hit_world, local_hit / safe_scale, distance));
    }

    let mut matching_cameras: Vec<_> = cameras
        .iter()
        .filter(|(_, cam, _, rt_opt)| cam.is_active && matches_target(*rt_opt))
        .collect();
    matching_cameras.sort_by(|left, right| right.1.order.cmp(&left.1.order));

    matching_cameras
        .into_iter()
        .find_map(|(cam_entity, cam, cam_gt, _)| {
            let (order, hit_world, local_hit, distance) =
                ray_hit_for_camera(cam, cam_gt, screen_pos, node_gt, require_viewport_rect)?;
            Some((cam_entity, order, hit_world, local_hit / safe_scale, distance))
        })
}

/// Collects active window cursors as `(window_entity, cursor_pos, primary_window)` ordered primary-first.
pub fn collect_active_cursor_windows(
    windows: &Query<(Entity, &Window, Option<&PrimaryWindow>)>,
) -> Vec<(Entity, Vec2, Option<Entity>)> {
    let primary_window =
        resolve_primary_window_entity(windows.iter().map(|(e, _, p)| (e, p.is_some())));
    let mut active: Vec<_> = windows
        .iter()
        .filter_map(|(entity, window, _)| {
            window
                .cursor_position()
                .map(|cursor| (entity, cursor, primary_window))
        })
        .collect();
    active.sort_by_key(|(entity, _, _)| usize::from(Some(*entity) != primary_window));
    active
}

fn node_radius_vec(node: &UNode) -> Vec4 {
    Vec4::new(
        node.border_radius.top_right,
        node.border_radius.bottom_right,
        node.border_radius.top_left,
        node.border_radius.bottom_left,
    )
}

/// Returns `true` when `world_pos` lies outside the rounded/cut shape of a single clipper.
pub fn is_point_clipped_by_clipper(
    world_pos: Vec3,
    transform: &GlobalTransform,
    size: &ComputedSize,
    node: &UNode,
    world_scale: f32,
) -> bool {
    let matrix = transform.to_matrix();
    if matrix.determinant().abs() <= 1e-12 {
        return true;
    }

    let cursor_in_clipper_space = matrix.inverse().transform_point3(world_pos).truncate();
    let half_size = Vec2::new(size.width, size.height) * 0.5 * world_scale;
    let radius = node_radius_vec(node) * world_scale;
    sd_node_shape(cursor_in_clipper_space, half_size, radius, node.shape_mode) > 0.0
}

fn check_clip_chain_from(
    mut current: Entity,
    world_pos: Vec3,
    world_scale: f32,
    parents_query: &Query<&ChildOf>,
    clipper_query: &Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
) -> bool {
    loop {
        if let Ok((transform, size, node, clip)) = clipper_query.get(current)
            && clip.enabled
            && is_point_clipped_by_clipper(world_pos, transform, size, node, world_scale)
        {
            return true;
        }
        let Ok(parent) = parents_query.get(current) else {
            return false;
        };
        current = parent.get();
    }
}

/// Returns `true` when `world_pos` is clipped out by **any** enabled [`UClip`] ancestor.
///
/// Walks the full ancestor chain up to the root so nested outer+inner clippers are
/// all enforced, scaling bounds and radii by `world_scale`.
pub fn is_world_point_clipped_by_ancestors(
    start_entity: Entity,
    world_pos: Vec3,
    world_scale: f32,
    first_clip_ancestor: Option<Entity>,
    parents_query: &Query<&ChildOf>,
    clipper_query: &Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
) -> bool {
    let mut current = start_entity;
    let mut seen_first = false;

    while let Ok(parent) = parents_query.get(current) {
        current = parent.get();
        if first_clip_ancestor == Some(current) {
            seen_first = true;
        }
        if let Ok((transform, size, node, clip)) = clipper_query.get(current)
            && clip.enabled
            && is_point_clipped_by_clipper(world_pos, transform, size, node, world_scale)
        {
            return true;
        }
    }

    if !seen_first
        && let Some(clip_entity) = first_clip_ancestor
        && check_clip_chain_from(clip_entity, world_pos, world_scale, parents_query, clipper_query)
    {
        return true;
    }

    false
}

fn transform_is_pure_2d_translation(transform: &GlobalTransform) -> bool {
    let m = transform.affine().matrix3;
    (m.x_axis.x - 1.0).abs() <= 1e-4
        && m.x_axis.y.abs() <= 1e-4
        && m.x_axis.z.abs() <= 1e-4
        && m.y_axis.x.abs() <= 1e-4
        && (m.y_axis.y - 1.0).abs() <= 1e-4
        && m.y_axis.z.abs() <= 1e-4
        && (m.z_axis.z - 1.0).abs() <= 1e-4
}

/// Collects up to [`MAX_CLIP_LAYERS`] enabled [`UClip`] ancestors for `entity`.
///
/// Ancestors are ordered innermost-first (`slot 0` nearest ancestor).
pub fn collect_clip_stack_for_entity(
    start_entity: Entity,
    cached_world_scale: Option<f32>,
    parents_query: &Query<&ChildOf>,
    clipper_query: &Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
    root_query: &Query<&ResolvedRootUi>,
) -> ClipUniformData {
    let world_scale = cached_world_scale
        .unwrap_or_else(|| resolve_world_scale_for_entity(start_entity, parents_query, root_query));

    let mut data = ClipUniformData::default();
    let mut aabb_min = Vec2::splat(f32::NEG_INFINITY);
    let mut aabb_max = Vec2::splat(f32::INFINITY);
    let mut aabb_possible = true;
    let mut current = start_entity;

    while let Ok(parent) = parents_query.get(current) {
        current = parent.parent();
        let Ok((clip_gt, clip_computed, clip_node, clip)) = clipper_query.get(current) else {
            continue;
        };
        if !clip.enabled {
            continue;
        }

        let matrix = clip_gt.to_matrix();
        if matrix.determinant().abs() <= 1e-12 {
            // Zero-scaled / degenerate clipper leaves zero visible region.
            let idx = data.layer_count;
            data.clip_inv_x[idx] = Vec4::ZERO;
            data.clip_inv_y[idx] = Vec4::ZERO;
            data.clip_bounds[idx] = Vec4::new(-1.0, -1.0, 1.0, 0.0);
            data.clip_radius[idx] = Vec4::ZERO;
            data.layer_count += 1;
            data.is_complex_for_batch = true;
            aabb_possible = false;
            if data.layer_count >= MAX_CLIP_LAYERS {
                break;
            }
            continue;
        }

        let inv = matrix.inverse();
        let half_size = Vec2::new(clip_computed.width, clip_computed.height) * 0.5 * world_scale;
        let radius = node_radius_vec(clip_node) * world_scale;
        let shape_mode_f = match clip_node.shape_mode {
            UShapeMode::Round => 0.0,
            UShapeMode::Cut => 1.0,
        };

        let idx = data.layer_count;
        data.clip_inv_x[idx] = inv.row(0);
        data.clip_inv_y[idx] = inv.row(1);
        data.clip_bounds[idx] = Vec4::new(half_size.x, half_size.y, 1.0, shape_mode_f);
        data.clip_radius[idx] = radius;
        data.layer_count += 1;

        if aabb_possible && transform_is_pure_2d_translation(clip_gt) && shape_mode_f < 0.5 {
            let center = clip_gt.translation().truncate();
            let c_min = center - half_size;
            let c_max = center + half_size;
            aabb_min = aabb_min.max(c_min);
            aabb_max = aabb_max.min(c_max);
            if idx == 0 {
                data.aabb_radius = radius;
            } else {
                data.is_complex_for_batch = true;
            }
        } else {
            aabb_possible = false;
            data.is_complex_for_batch = true;
        }

        if data.layer_count >= MAX_CLIP_LAYERS {
            break;
        }
    }

    if data.layer_count > 0 && aabb_possible {
        if aabb_max.x >= aabb_min.x && aabb_max.y >= aabb_min.y {
            data.aabb_center = (aabb_min + aabb_max) * 0.5;
            data.aabb_size = aabb_max - aabb_min;
        } else {
            data.aabb_center = Vec2::ZERO;
            data.aabb_size = Vec2::ZERO;
            data.aabb_radius = Vec4::ZERO;
        }
    }

    data
}

/// Synchronizes clipping uniforms across both 2D (`UNodeMaterial`) and 3D (`UNodeMaterial3d`) UI nodes.
pub fn sync_node_clipper_materials(
    nodes_query: Query<(
        Entity,
        Option<&MeshMaterial2d<UNodeMaterial>>,
        Option<&MeshMaterial3d<UNodeMaterial3d>>,
        Option<&CachedUiContext>,
    )>,
    parents_query: Query<&ChildOf>,
    clipper_query: Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
    root_query: Query<&ResolvedRootUi>,
    materials_2d: Option<ResMut<Assets<UNodeMaterial>>>,
    materials_3d: Option<ResMut<Assets<UNodeMaterial3d>>>,
) {
    let mut materials_2d = materials_2d;
    let mut materials_3d = materials_3d;

    for (entity, mat2d_opt, mat3d_opt, cached_opt) in nodes_query.iter() {
        if mat2d_opt.is_none() && mat3d_opt.is_none() {
            continue;
        }

        let cached_scale = cached_opt
            .filter(|ctx| ctx.root_entity.is_some())
            .map(|ctx| ctx.ui_to_world_scale);
        let clip_data = collect_clip_stack_for_entity(
            entity,
            cached_scale,
            &parents_query,
            &clipper_query,
            &root_query,
        );

        if let Some(mat_handle) = mat2d_opt
            && let Some(ref mut mats) = materials_2d
            && mats
                .get(&mat_handle.0)
                .is_some_and(|material| !material.matches_clip_uniforms(&clip_data))
            && let Some(mut material) = mats.get_mut(&mat_handle.0)
        {
            material.apply_clip_uniforms(&clip_data);
        }

        if let Some(mat_handle) = mat3d_opt
            && let Some(ref mut mats) = materials_3d
            && mats
                .get(&mat_handle.0)
                .is_some_and(|material| !material.matches_clip_uniforms(&clip_data))
            && let Some(mut material) = mats.get_mut(&mat_handle.0)
        {
            material.apply_clip_uniforms(&clip_data);
        }
    }
}

/// Active clipping summary attached to nodes inside [`UClip`] containers for 2D batching.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq)]
#[reflect(Component)]
pub struct UActiveClip {
    /// World-space center of the intersected unrotated clip box.
    pub center: Vec2,
    /// Physical size of the intersected unrotated clip box in world units.
    pub size: Vec2,
    /// Corner radii of the primary clip box in world units.
    pub radius: Vec4,
    /// True when clipping is active on this node.
    pub use_clip: bool,
    /// True when rotation, non-unit scale, cut shapes, or multi-level nesting requires the material pipeline.
    pub is_complex: bool,
}

/// Propagates [`UActiveClip`] components for instanced batch extraction.
pub fn sync_node_active_clips(
    mut commands: Commands,
    mut existing_query: Query<(Entity, &mut UActiveClip)>,
    unode_query: Query<Entity, (With<UNode>, Without<UActiveClip>)>,
    cached_query: Query<&CachedUiContext>,
    parents_query: Query<&ChildOf>,
    clipper_query: Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
    root_query: Query<&ResolvedRootUi>,
) {
    for (entity, mut active_clip) in existing_query.iter_mut() {
        let cached_scale = cached_query
            .get(entity)
            .ok()
            .filter(|ctx| ctx.root_entity.is_some())
            .map(|ctx| ctx.ui_to_world_scale);
        let clip_data = collect_clip_stack_for_entity(
            entity,
            cached_scale,
            &parents_query,
            &clipper_query,
            &root_query,
        );
        let next = UActiveClip {
            center: clip_data.aabb_center,
            size: clip_data.aabb_size,
            radius: clip_data.aabb_radius,
            use_clip: clip_data.is_active(),
            is_complex: clip_data.is_complex_for_batch,
        };
        if *active_clip != next {
            *active_clip = next;
        }
    }

    for entity in unode_query.iter() {
        let cached_scale = cached_query
            .get(entity)
            .ok()
            .filter(|ctx| ctx.root_entity.is_some())
            .map(|ctx| ctx.ui_to_world_scale);
        let clip_data = collect_clip_stack_for_entity(
            entity,
            cached_scale,
            &parents_query,
            &clipper_query,
            &root_query,
        );
        if clip_data.is_active() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_insert(UActiveClip {
                    center: clip_data.aabb_center,
                    size: clip_data.aabb_size,
                    radius: clip_data.aabb_radius,
                    use_clip: true,
                    is_complex: clip_data.is_complex_for_batch,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::layout_system::{ResolvedRootUi, URootUi, UiCanvasSize, UiSpace};
    use luma_layout::geometry::UCornerRadius;

    #[test]
    fn test_active_clipper_for_node() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<UNodeMaterial>();

        let root = app
            .world_mut()
            .spawn((
                UNode::default(),
                ComputedSize {
                    width: 800.0,
                    height: 600.0,
                    ..default()
                },
                Transform::default(),
                GlobalTransform::from_translation(Vec3::ZERO),
            ))
            .id();

        let clipper = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UNode {
                    border_radius: UCornerRadius::all(8.0),
                    ..default()
                },
                ComputedSize {
                    width: 300.0,
                    height: 200.0,
                    ..default()
                },
                UClip { enabled: true },
                Transform::default(),
                GlobalTransform::from_translation(Vec3::new(100.0, 50.0, 0.0)),
            ))
            .id();

        let child = app
            .world_mut()
            .spawn((
                ChildOf(clipper),
                UNode::default(),
                ComputedSize {
                    width: 100.0,
                    height: 50.0,
                    ..default()
                },
                Transform::default(),
                GlobalTransform::from_translation(Vec3::new(120.0, 60.0, 0.0)),
            ))
            .id();

        let mut materials = app.world_mut().resource_mut::<Assets<UNodeMaterial>>();
        let mat_handle = materials.add(UNodeMaterial::default());
        app.world_mut()
            .entity_mut(child)
            .insert(MeshMaterial2d(mat_handle.clone()));

        app.add_systems(Update, sync_node_clipper_materials);
        app.update();

        let materials = app.world().resource::<Assets<UNodeMaterial>>();
        let material = materials.get(&mat_handle).unwrap();
        assert_eq!(material.clip_bounds[0], Vec4::new(150.0, 100.0, 1.0, 0.0));
        assert_eq!(material.clip_bounds[1], Vec4::ZERO);
        assert_eq!(material.clip_radius[0], Vec4::splat(8.0));

        // Verify world point (100, 50, 0) maps to clipper origin (0, 0)
        let wp = Vec4::new(100.0, 50.0, 0.0, 1.0);
        let local = Vec2::new(material.clip_inv_x[0].dot(wp), material.clip_inv_y[0].dot(wp));
        assert!(local.abs_diff_eq(Vec2::ZERO, 1e-5));
    }

    #[test]
    fn nested_and_rotated_3d_clippers_populate_both_2d_and_3d_materials() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<UNodeMaterial3d>();

        let root = app
            .world_mut()
            .spawn((
                URootUi::world_3d(Vec2::new(800.0, 600.0)),
                ResolvedRootUi {
                    root_entity: Entity::PLACEHOLDER,
                    space: UiSpace::World3d,
                    canvas: UiCanvasSize::Fixed(Vec2::new(800.0, 600.0)),
                    canvas_size: Vec2::new(800.0, 600.0),
                    camera_entity: None,
                    meters_per_unit: 0.01,
                    resolution_scale: 1.0,
                },
                UNode::default(),
                ComputedSize {
                    width: 800.0,
                    height: 600.0,
                    ..default()
                },
                GlobalTransform::IDENTITY,
            ))
            .id();
        app.world_mut()
            .entity_mut(root)
            .get_mut::<ResolvedRootUi>()
            .unwrap()
            .root_entity = root;

        let outer = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UNode {
                    shape_mode: UShapeMode::Cut,
                    border_radius: UCornerRadius::all(10.0),
                    ..default()
                },
                ComputedSize {
                    width: 400.0,
                    height: 400.0,
                    ..default()
                },
                UClip { enabled: true },
                GlobalTransform::from(Transform::from_xyz(1.0, 0.0, 2.0)),
            ))
            .id();

        let inner = app
            .world_mut()
            .spawn((
                ChildOf(outer),
                UNode::default(),
                ComputedSize {
                    width: 200.0,
                    height: 100.0,
                    ..default()
                },
                UClip { enabled: true },
                GlobalTransform::from(
                    Transform::from_xyz(1.0, 0.0, 2.0)
                        .with_rotation(Quat::from_rotation_z(core::f32::consts::FRAC_PI_2)),
                ),
            ))
            .id();

        let mut mats = app.world_mut().resource_mut::<Assets<UNodeMaterial3d>>();
        let handle = mats.add(UNodeMaterial3d {
            size: Vec2::new(100.0, 100.0),
            softness: 0.5,
            ..default()
        });

        let child = app
            .world_mut()
            .spawn((
                ChildOf(inner),
                UNode::default(),
                ComputedSize::default(),
                MeshMaterial3d(handle.clone()),
            ))
            .id();

        app.add_systems(Update, sync_node_clipper_materials);
        app.update();

        let mats = app.world().resource::<Assets<UNodeMaterial3d>>();
        let mat = mats.get(&handle).unwrap();
        // Slot 0 = inner rotated clipper (world_scale 0.01 -> half 1.0 x 0.5)
        assert!((mat.clip_bounds[0].x - 1.0).abs() < 1e-5);
        assert!((mat.clip_bounds[0].y - 0.5).abs() < 1e-5);
        assert_eq!(mat.clip_bounds[0].z, 1.0);
        // Slot 1 = outer cut-corner clipper (half 2.0 x 2.0, shape_mode = 1.0)
        assert!((mat.clip_bounds[1].x - 2.0).abs() < 1e-5);
        assert_eq!(mat.clip_bounds[1].z, 1.0);
        assert_eq!(mat.clip_bounds[1].w, 1.0);
        assert!(mat.clip_radius[1].abs_diff_eq(Vec4::splat(0.1), 1e-5));

        // World point shifted +0.75 along world Y maps into rotated inner clipper X (+0.75), well inside half_x=1.0
        let wp = Vec4::new(1.0, 0.75, 2.0, 1.0);
        let p_inner = Vec2::new(mat.clip_inv_x[0].dot(wp), mat.clip_inv_y[0].dot(wp));
        assert!(p_inner.abs_diff_eq(Vec2::new(0.75, 0.0), 1e-5));
        assert!(app.world().get::<ChildOf>(child).is_some());
    }

    #[test]
    fn test_sync_node_active_clips_propagation() {
        let mut app = App::new();

        let clipper = app
            .world_mut()
            .spawn((
                UNode {
                    border_radius: UCornerRadius::all(12.0),
                    ..default()
                },
                ComputedSize {
                    width: 400.0,
                    height: 300.0,
                    ..default()
                },
                UClip { enabled: true },
                Transform::default(),
                GlobalTransform::from_translation(Vec3::new(50.0, 50.0, 0.0)),
            ))
            .id();

        let child = app
            .world_mut()
            .spawn((
                ChildOf(clipper),
                UNode::default(),
                ComputedSize {
                    width: 100.0,
                    height: 100.0,
                    ..default()
                },
                Transform::default(),
                GlobalTransform::from_translation(Vec3::new(60.0, 60.0, 0.0)),
            ))
            .id();

        app.add_systems(Update, sync_node_active_clips);
        app.update();

        let active_clip = app.world().get::<UActiveClip>(child).unwrap();
        assert!(active_clip.use_clip);
        assert!(!active_clip.is_complex);
        assert_eq!(active_clip.center, Vec2::new(50.0, 50.0));
        assert_eq!(active_clip.size, Vec2::new(400.0, 300.0));
        assert_eq!(active_clip.radius, Vec4::splat(12.0));
    }
}
