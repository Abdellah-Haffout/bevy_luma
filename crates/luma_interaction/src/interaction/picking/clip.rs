use bevy::prelude::{ChildOf, Entity, GlobalTransform, Query, Vec3};

use luma_engine::layout::query::{ComputedSize, UiPickingContext};
use luma_engine::layout::render::clip_sync::is_world_point_clipped_by_ancestors;
use luma_engine::layout::univis_node::{UClip, UNode};

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn is_clipped_by_ancestors(
    start_entity: Entity,
    cursor_world_pos: Vec3,
    parents_query: &Query<&ChildOf>,
    clipper_query: &Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
) -> bool {
    is_world_point_clipped_by_ancestors(
        start_entity,
        cursor_world_pos,
        1.0,
        None,
        parents_query,
        clipper_query,
    )
}

pub(super) fn is_clipped_by_picking_context(
    picking_context: Option<&UiPickingContext>,
    start_entity: Entity,
    cursor_world_pos: Vec3,
    parents_query: &Query<&ChildOf>,
    clipper_query: &Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
) -> bool {
    is_world_point_clipped_by_ancestors(
        start_entity,
        cursor_world_pos,
        picking_context.map_or(1.0, |context| context.world_scale),
        picking_context.and_then(|context| context.clip_ancestor),
        parents_query,
        clipper_query,
    )
}
