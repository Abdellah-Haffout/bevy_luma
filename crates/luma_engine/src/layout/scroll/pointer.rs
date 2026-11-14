use std::cmp::Ordering;

use bevy::camera::RenderTarget;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::layout::layout_system::ResolvedRootUi;
use crate::layout::query::{ComputedSize, UiPickingContext};
use crate::layout::render::clip_sync::{
    camera_matches_window, collect_active_cursor_windows, is_entity_visible_in_hierarchy,
    is_world_point_clipped_by_ancestors, ray_hit_for_camera, resolve_node_world_scale,
    resolve_primary_window_entity, sd_node_shape,
};
use crate::layout::univis_node::{UBorder, UClip, UNode};

/// Ranked hit candidate used to choose the topmost scroll target or occluder.
#[derive(Clone, Copy, Debug)]
pub(super) struct ScrollHitCandidate {
    pub entity: Entity,
    pub camera_order: f32,
    pub root_sort_key: f32,
    pub local_depth_key: f32,
    pub hit_distance: f32,
    pub hit_by_cursor: bool,
}

impl ScrollHitCandidate {
    /// Compares two candidates by camera order, root sort, local depth, and nearer ray distance.
    pub fn rank_cmp(&self, other: &Self) -> Ordering {
        self.camera_order
            .total_cmp(&other.camera_order)
            .then_with(|| self.root_sort_key.total_cmp(&other.root_sort_key))
            .then_with(|| self.local_depth_key.total_cmp(&other.local_depth_key))
            .then_with(|| other.hit_distance.total_cmp(&self.hit_distance))
    }
}

/// Result of projecting a window cursor ray onto a scroll UI node plane.
#[derive(Clone, Copy, Debug)]
pub(super) struct ScrollRayHit {
    pub entity: Entity,
    pub window_entity: Entity,
    pub camera_entity: Entity,
    pub camera_order: f32,
    pub hit_world: Vec3,
    pub local_hit_logical: Vec2,
    pub hit_distance: f32,
}

/// Shared read-only ECS context for scroll pointer resolution, ray hit-testing, and occlusion.
#[doc(hidden)]
#[derive(SystemParam)]
pub struct ScrollPointerContext<'w, 's> {
    pub(super) windows: Query<'w, 's, (Entity, &'static Window, Option<&'static PrimaryWindow>)>,
    pub(super) cameras: Query<
        'w,
        's,
        (
            Entity,
            &'static Camera,
            &'static GlobalTransform,
            Option<&'static RenderTarget>,
        ),
    >,
    pub(super) visibility_query: Query<'w, 's, &'static Visibility>,
    pub(super) parents_query: Query<'w, 's, &'static ChildOf>,
    pub(super) clipper_query: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static ComputedSize,
            &'static UNode,
            &'static UClip,
        ),
    >,
    pub(super) root_query: Query<'w, 's, &'static ResolvedRootUi>,
    pub(super) occluders: Query<
        'w,
        's,
        (
            Entity,
            &'static UNode,
            Option<&'static UBorder>,
            &'static GlobalTransform,
            &'static ComputedSize,
            Option<&'static UiPickingContext>,
        ),
    >,
}

impl<'w, 's> ScrollPointerContext<'w, 's> {
    /// Returns `true` when `entity` and all of its `ChildOf` ancestors are visible.
    pub(super) fn is_visible(&self, entity: Entity) -> bool {
        is_entity_visible_in_hierarchy(entity, &self.visibility_query, &self.parents_query)
    }

    /// Returns `true` when `entity` is a descendant of `ancestor`.
    pub(super) fn is_descendant_of(&self, entity: Entity, ancestor: Entity) -> bool {
        let mut current = entity;
        while let Ok(parent) = self.parents_query.get(current) {
            let parent_entity = parent.parent();
            if parent_entity == ancestor {
                return true;
            }
            current = parent_entity;
        }
        false
    }

    /// Resolves the primary window entity, falling back to the sole window when unambiguous.
    pub(super) fn primary_window(&self) -> Option<Entity> {
        resolve_primary_window_entity(self.windows.iter().map(|(e, _, p)| (e, p.is_some())))
    }

    /// Resolves `(window_entity, primary_window, cursor_pos)` for a targeted event window.
    pub(super) fn resolve_window(
        &self,
        target_window: Entity,
    ) -> (Option<Entity>, Option<Entity>, Option<Vec2>) {
        let primary_window = self.primary_window();

        let resolved = if target_window != Entity::PLACEHOLDER {
            self.windows
                .get(target_window)
                .ok()
                .map(|(entity, window, _)| (entity, window.cursor_position()))
        } else {
            None
        }
        .or_else(|| {
            primary_window.and_then(|entity| {
                self.windows
                    .get(entity)
                    .ok()
                    .map(|(win_entity, window, _)| (win_entity, window.cursor_position()))
            })
        });

        match resolved {
            Some((win_entity, cursor)) => (Some(win_entity), primary_window, cursor),
            None => (None, primary_window, None),
        }
    }

    /// Collects all windows currently reporting a cursor position, ordered primary-first.
    pub(super) fn active_cursor_windows(&self) -> Vec<(Entity, Vec2, Option<Entity>)> {
        collect_active_cursor_windows(&self.windows)
    }

    /// Resolves the effective `camera.order` from an optional [`UiPickingContext`].
    pub(super) fn camera_order(&self, picking_ctx: Option<&UiPickingContext>) -> f32 {
        picking_ctx
            .and_then(|ctx| ctx.camera_entity)
            .and_then(|cam_entity| self.cameras.get(cam_entity).ok())
            .map_or(0.0, |(_, cam, _, _)| cam.order as f32)
    }

    /// Builds a [`ScrollHitCandidate`] for depth comparison.
    pub(super) fn candidate_rank(
        &self,
        entity: Entity,
        camera_order: f32,
        hit_distance: f32,
        hit_by_cursor: bool,
        global_transform: &GlobalTransform,
        picking_ctx: Option<&UiPickingContext>,
    ) -> ScrollHitCandidate {
        ScrollHitCandidate {
            entity,
            camera_order,
            root_sort_key: picking_ctx.map_or(0.0, |ctx| ctx.root_sort_key),
            local_depth_key: picking_ctx
                .map_or_else(|| global_transform.translation().z, |ctx| ctx.local_depth_key),
            hit_distance,
            hit_by_cursor,
        }
    }

    /// Ray-tests `cursor_pos` on `window_entity` against `entity`'s shape and ancestor `UClip`s.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn hit_test_node(
        &self,
        entity: Entity,
        node: &UNode,
        global_transform: &GlobalTransform,
        computed: &ComputedSize,
        picking_ctx: Option<&UiPickingContext>,
        window_entity: Entity,
        primary_window: Option<Entity>,
        cursor_pos: Vec2,
    ) -> Option<ScrollRayHit> {
        let world_scale = self.resolve_world_scale(entity, picking_ctx);
        let safe_scale = safe_world_scale(world_scale);
        let half_size = Vec2::new(computed.width, computed.height) * 0.5 * world_scale;
        if half_size.x <= 0.0 || half_size.y <= 0.0 {
            return None;
        }
        let radius = node_radius_vec(node) * world_scale;

        let try_camera = |cam_entity: Entity, camera: &Camera, cam_gt: &GlobalTransform| {
            let (camera_order, hit_world, local_hit, hit_distance) =
                ray_hit_for_camera(camera, cam_gt, cursor_pos, global_transform, true)?;
            if sd_node_shape(local_hit, half_size, radius, node.shape_mode) > 0.0 {
                return None;
            }
            Some(ScrollRayHit {
                entity,
                window_entity,
                camera_entity: cam_entity,
                camera_order,
                hit_world,
                local_hit_logical: local_hit / safe_scale,
                hit_distance,
            })
        };

        let hit = if let Some(cam_entity) = picking_ctx.and_then(|ctx| ctx.camera_entity) {
            let (_, camera, cam_gt, rt_opt) = self.cameras.get(cam_entity).ok()?;
            if !camera_matches_window(rt_opt, window_entity, primary_window) {
                return None;
            }
            try_camera(cam_entity, camera, cam_gt)?
        } else {
            let mut matching_cameras: Vec<_> = self
                .cameras
                .iter()
                .filter(|(_, cam, _, rt_opt)| {
                    cam.is_active && camera_matches_window(*rt_opt, window_entity, primary_window)
                })
                .collect();
            matching_cameras.sort_by(|left, right| right.1.order.cmp(&left.1.order));

            matching_cameras
                .into_iter()
                .find_map(|(cam_entity, cam, cam_gt, _)| try_camera(cam_entity, cam, cam_gt))?
        };

        if is_world_point_clipped_by_ancestors(
            entity,
            hit.hit_world,
            world_scale,
            picking_ctx.and_then(|ctx| ctx.clip_ancestor),
            &self.parents_query,
            &self.clipper_query,
        ) {
            return None;
        }

        Some(hit)
    }

    /// Hit-tests `entity` across all active cursor windows and returns the best hit & rank.
    pub(super) fn hit_test_node_any_window(
        &self,
        entity: Entity,
        node: &UNode,
        global_transform: &GlobalTransform,
        computed: &ComputedSize,
        picking_ctx: Option<&UiPickingContext>,
        active_windows: &[(Entity, Vec2, Option<Entity>)],
    ) -> Option<(ScrollRayHit, ScrollHitCandidate)> {
        let mut best: Option<(ScrollRayHit, ScrollHitCandidate)> = None;
        for &(win_entity, cursor_pos, primary_window) in active_windows {
            let Some(hit) = self.hit_test_node(
                entity,
                node,
                global_transform,
                computed,
                picking_ctx,
                win_entity,
                primary_window,
                cursor_pos,
            ) else {
                continue;
            };
            let rank = self.candidate_rank(
                entity,
                hit.camera_order,
                hit.hit_distance,
                true,
                global_transform,
                picking_ctx,
            );
            if best
                .as_ref()
                .is_none_or(|(_, best_rank)| rank.rank_cmp(best_rank) == Ordering::Greater)
            {
                best = Some((hit, rank));
            }
        }
        best
    }

    /// Selects the highest-ranked hit among `candidates` across `active_windows` and rejects occluded hits.
    pub(super) fn pick_unoccluded_hit<'a, I>(
        &self,
        candidates: I,
        active_windows: &[(Entity, Vec2, Option<Entity>)],
    ) -> Option<ScrollRayHit>
    where
        I: IntoIterator<
            Item = (
                Entity,
                &'a UNode,
                &'a GlobalTransform,
                &'a ComputedSize,
                Option<&'a UiPickingContext>,
            ),
        >,
    {
        let mut best: Option<(ScrollRayHit, ScrollHitCandidate)> = None;

        for (entity, node, gt, computed, picking_ctx) in candidates {
            if !self.is_visible(entity) {
                continue;
            }
            let Some((hit, candidate)) =
                self.hit_test_node_any_window(entity, node, gt, computed, picking_ctx, active_windows)
            else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, best_rank)| candidate.rank_cmp(best_rank) == Ordering::Greater)
            {
                best = Some((hit, candidate));
            }
        }

        let (hit, candidate) = best?;
        let &(_, cursor_pos, primary_window) = active_windows
            .iter()
            .find(|&&(win_e, _, _)| win_e == hit.window_entity)?;
        if self.is_hit_occluded(&candidate, hit.window_entity, primary_window, cursor_pos) {
            return None;
        }
        Some(hit)
    }

    /// Projects the active window cursor onto `entity`'s plane in logical UI pixel space.
    ///
    /// Unlike [`Self::hit_test_node`], this skips bounds and clip checks so active pointer
    /// drags remain continuous even when the cursor temporarily leaves the element rect.
    pub(super) fn project_cursor_to_node_logical(
        &self,
        entity: Entity,
        global_transform: &GlobalTransform,
        picking_ctx: Option<&UiPickingContext>,
        preferred_camera: Option<Entity>,
        active_windows: &[(Entity, Vec2, Option<Entity>)],
    ) -> Option<Vec2> {
        let safe_scale = safe_world_scale(self.resolve_world_scale(entity, picking_ctx));
        let cam_entity_opt = preferred_camera.or_else(|| picking_ctx.and_then(|ctx| ctx.camera_entity));

        if let Some(cam_entity) = cam_entity_opt {
            let (_, camera, cam_gt, rt_opt) = self.cameras.get(cam_entity).ok()?;
            if !camera.is_active {
                return None;
            }
            let &(_, cursor_pos, _) = active_windows
                .iter()
                .find(|&&(win_entity, _, primary_window)| {
                    camera_matches_window(rt_opt, win_entity, primary_window)
                })?;
            let (_, _, local_hit, _) =
                ray_hit_for_camera(camera, cam_gt, cursor_pos, global_transform, false)?;
            return Some(local_hit / safe_scale);
        }

        // Fall back to camera viewport matching first, then any camera on an active window.
        for require_viewport_rect in [true, false] {
            for &(win_entity, cursor_pos, primary_window) in active_windows {
                let mut matching_cameras: Vec<_> = self
                    .cameras
                    .iter()
                    .filter(|(_, cam, _, rt_opt)| {
                        cam.is_active
                            && camera_matches_window(*rt_opt, win_entity, primary_window)
                    })
                    .collect();
                matching_cameras.sort_by(|left, right| right.1.order.cmp(&left.1.order));

                if let Some(local_hit) =
                    matching_cameras.into_iter().find_map(|(_, cam, cam_gt, _)| {
                        ray_hit_for_camera(
                            cam,
                            cam_gt,
                            cursor_pos,
                            global_transform,
                            require_viewport_rect,
                        )
                        .map(|(_, _, local_hit, _)| local_hit)
                    })
                {
                    return Some(local_hit / safe_scale);
                }
            }
        }

        None
    }

    /// Returns `true` when any solid, visible, non-descendant UI node outranks `winner` at `cursor_pos`.
    pub(super) fn is_hit_occluded(
        &self,
        winner: &ScrollHitCandidate,
        window_entity: Entity,
        primary_window: Option<Entity>,
        cursor_pos: Vec2,
    ) -> bool {
        self.occluders.iter().any(
            |(occ_entity, occ_node, occ_border, occ_gt, occ_size, occ_picking)| {
                if occ_entity == winner.entity
                    || !is_solid_occluder_node(occ_node, occ_border)
                    || self.is_descendant_of(occ_entity, winner.entity)
                    || !self.is_visible(occ_entity)
                {
                    return false;
                }

                let Some(hit) = self.hit_test_node(
                    occ_entity,
                    occ_node,
                    occ_gt,
                    occ_size,
                    occ_picking,
                    window_entity,
                    primary_window,
                    cursor_pos,
                ) else {
                    return false;
                };

                let occ_rank = self.candidate_rank(
                    occ_entity,
                    hit.camera_order,
                    hit.hit_distance,
                    true,
                    occ_gt,
                    occ_picking,
                );
                occ_rank.rank_cmp(winner) == Ordering::Greater
            },
        )
    }

    fn resolve_world_scale(&self, entity: Entity, picking_ctx: Option<&UiPickingContext>) -> f32 {
        resolve_node_world_scale(entity, picking_ctx, &self.parents_query, &self.root_query)
    }
}

fn safe_world_scale(world_scale: f32) -> f32 {
    if world_scale.abs() > f32::EPSILON {
        world_scale
    } else {
        1.0
    }
}

fn node_radius_vec(node: &UNode) -> Vec4 {
    Vec4::new(
        node.border_radius.top_right,
        node.border_radius.bottom_right,
        node.border_radius.top_left,
        node.border_radius.bottom_left,
    )
}

fn is_solid_occluder_node(node: &UNode, border: Option<&UBorder>) -> bool {
    node.background_color.alpha() > 0.001
        || border.is_some_and(|b| b.width > 0.0 && b.color.alpha() > 0.001)
}
