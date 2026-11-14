use super::*;
use crate::layout::render::clip_sync::{
    ClipUniformTarget, MAX_CLIP_LAYERS, collect_clip_stack_for_entity, resolve_world_scale_for_entity,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct LocalClipRect {
    pub(crate) min: Vec2,
    pub(crate) max: Vec2,
}

impl ClipUniformTarget for UTextLabelSdfMaterial {
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

impl ClipUniformTarget for UTextLabelSdfMaterial3d {
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

pub(crate) fn clip_quad_to_rect(
    quad: TextGlyphQuad,
    clip_rect: LocalClipRect,
) -> Option<TextGlyphQuad> {
    let half = quad.size * 0.5;
    let original_min = quad.center - half;
    let original_max = quad.center + half;

    let clipped_min = Vec2::new(
        original_min.x.max(clip_rect.min.x),
        original_min.y.max(clip_rect.min.y),
    );
    let clipped_max = Vec2::new(
        original_max.x.min(clip_rect.max.x),
        original_max.y.min(clip_rect.max.y),
    );

    if clipped_min.x >= clipped_max.x || clipped_min.y >= clipped_max.y {
        return None;
    }

    let original_width = original_max.x - original_min.x;
    let original_height = original_max.y - original_min.y;
    if original_width <= 0.0 || original_height <= 0.0 {
        return None;
    }

    let left_t = (clipped_min.x - original_min.x) / original_width;
    let right_t = (clipped_max.x - original_min.x) / original_width;
    let bottom_t = (clipped_min.y - original_min.y) / original_height;
    let top_t = (clipped_max.y - original_min.y) / original_height;

    let uv_left = quad.uv_min.x + (quad.uv_max.x - quad.uv_min.x) * left_t;
    let uv_right = quad.uv_min.x + (quad.uv_max.x - quad.uv_min.x) * right_t;
    let uv_bottom = quad.uv_max.y + (quad.uv_min.y - quad.uv_max.y) * bottom_t;
    let uv_top = quad.uv_max.y + (quad.uv_min.y - quad.uv_max.y) * top_t;

    Some(TextGlyphQuad {
        center: (clipped_min + clipped_max) * 0.5,
        size: clipped_max - clipped_min,
        uv_min: Vec2::new(uv_left, uv_top),
        uv_max: Vec2::new(uv_right, uv_bottom),
        color: quad.color,
    })
}

pub(crate) fn label_content_clip_rect(
    label: &UTextLabel,
    node: &UNode,
    computed_size: &ComputedSize,
) -> Option<LocalClipRect> {
    if label.overflow == UTextOverflow::Visible {
        return None;
    }

    if computed_size.width <= 0.0 || computed_size.height <= 0.0 {
        return None;
    }

    let width = (computed_size.width - node.padding.width_sum()).max(0.0);
    let height = (computed_size.height - node.padding.height_sum()).max(0.0);

    Some(LocalClipRect {
        min: Vec2::new(-width * 0.5, -height * 0.5),
        max: Vec2::new(width * 0.5, height * 0.5),
    })
}

fn text_root_stack_for_entity(
    entity: Entity,
    parents_query: &Query<&ChildOf>,
    root_stack_query: &Query<&ResolvedRootStack>,
) -> ResolvedRootStack {
    let mut current = entity;

    loop {
        if let Ok(stack) = root_stack_query.get(current) {
            return *stack;
        }

        let Ok(parent) = parents_query.get(current) else {
            return ResolvedRootStack::default();
        };
        current = parent.parent();
    }
}

fn text_render_transform(world_scale: f32, depth_offset: f32) -> Transform {
    Transform {
        translation: Vec3::new(0.0, 0.0, depth_offset),
        scale: Vec3::splat(world_scale),
        ..default()
    }
}

pub(crate) fn sync_text_render_transforms(
    mut text_query: Query<
        (Entity, &mut Transform, Has<TextSelectionHighlightMarker>, Has<Mesh3d>),
        Or<(With<TextChildMarker>, With<TextSelectionHighlightMarker>)>,
    >,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    root_stack_query: Query<&ResolvedRootStack>,
    stacking_query: Query<&crate::layout::components::UiLocalStacking>,
) {
    for (entity, mut transform, is_highlight, is_mesh_3d) in text_query.iter_mut() {
        let world_scale = resolve_world_scale_for_entity(entity, &parents_query, &root_query);
        let root_stack = text_root_stack_for_entity(entity, &parents_query, &root_stack_query);
        let parent_stacking = parents_query
            .get(entity)
            .ok()
            .and_then(|parent| stacking_query.get(parent.parent()).ok());
        // Place selection highlight quads strictly behind glyph pages (offset) and in front of the host node background (0).
        let base_offset = root_stack.text_child_offset_for_stacking(parent_stacking);
        let depth_offset = if is_highlight {
            base_offset * 0.5
        } else {
            base_offset
        };
        // World3d Mesh3d glyph/highlight vertices are already pre-scaled into world units;
        // scaling the Transform again would shrink them by world_scale^2.
        let transform_scale = if is_mesh_3d { 1.0 } else { world_scale };
        let desired_transform = text_render_transform(transform_scale, depth_offset);
        if *transform != desired_transform {
            *transform = desired_transform;
        }
    }
}

pub(crate) fn sync_text_clipper_materials(
    text_query: Query<
        (
            Entity,
            Option<&MeshMaterial2d<UTextLabelSdfMaterial>>,
            Option<&MeshMaterial3d<UTextLabelSdfMaterial3d>>,
        ),
        Or<(With<TextChildMarker>, With<TextSelectionHighlightMarker>)>,
    >,
    parents_query: Query<&ChildOf>,
    clipper_query: Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
    root_query: Query<&ResolvedRootUi>,
    materials: Option<ResMut<Assets<UTextLabelSdfMaterial>>>,
    materials_3d: Option<ResMut<Assets<UTextLabelSdfMaterial3d>>>,
) {
    let mut materials = materials;
    let mut materials_3d = materials_3d;
    for (entity, material_handle_2d, material_handle_3d) in text_query.iter() {
        let clip_data = collect_clip_stack_for_entity(
            entity,
            None,
            &parents_query,
            &clipper_query,
            &root_query,
        );

        if let Some(mat_handle) = material_handle_2d
            && let Some(ref mut materials) = materials
            && materials.get(&mat_handle.0).is_some_and(|material| {
                material.edge_softness != DEFAULT_TEXT_EDGE_SOFTNESS
                    || !material.matches_clip_uniforms(&clip_data)
            })
            && let Some(mut material) = materials.get_mut(&mat_handle.0)
        {
            material.edge_softness = DEFAULT_TEXT_EDGE_SOFTNESS;
            material.apply_clip_uniforms(&clip_data);
        }

        if let Some(mat_handle) = material_handle_3d
            && let Some(ref mut materials_3d) = materials_3d
            && materials_3d.get(&mat_handle.0).is_some_and(|material| {
                material.edge_softness != DEFAULT_TEXT_EDGE_SOFTNESS
                    || !material.matches_clip_uniforms(&clip_data)
            })
            && let Some(mut material) = materials_3d.get_mut(&mat_handle.0)
        {
            material.edge_softness = DEFAULT_TEXT_EDGE_SOFTNESS;
            material.apply_clip_uniforms(&clip_data);
        }
    }
}
