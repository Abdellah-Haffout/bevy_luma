use bevy::camera::RenderTarget;
use bevy::picking::pointer::Location;
use bevy::prelude::{Camera, Entity, GlobalTransform, Query, Vec2, Vec3};
use luma_engine::layout::render::clip_sync::camera_matches_pointer_target;

#[derive(Clone, Copy)]
pub(super) struct CachedPointerRay {
    pub(super) order: f32,
    pub(super) origin: Vec3,
    pub(super) direction: Vec3,
}

pub(super) fn pointer_ray_for_camera(
    location: &Location,
    camera_entity: Entity,
    cameras: &Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
    primary_window: Option<Entity>,
    has_any_window: bool,
) -> Option<CachedPointerRay> {
    let Ok((_, camera, camera_transform, render_target)) = cameras.get(camera_entity) else {
        return None;
    };

    if !camera.is_active
        || !camera_matches_pointer_target(
            render_target,
            &location.target,
            primary_window,
            has_any_window,
        )
    {
        return None;
    }

    if !camera
        .logical_viewport_rect()
        .is_some_and(|rect| rect.contains(location.position))
    {
        return None;
    }

    let ray = camera
        .viewport_to_world(camera_transform, location.position)
        .ok()?;
    Some(CachedPointerRay {
        order: camera.order as f32,
        origin: ray.origin,
        direction: ray.direction.as_vec3(),
    })
}

pub(super) fn intersect_ray_with_node_plane(
    ray: &CachedPointerRay,
    global_transform: &GlobalTransform,
) -> Option<(Vec3, Vec2, Vec3, f32)> {
    luma_engine::layout::render::clip_sync::intersect_ray_with_node_plane(
        ray.origin,
        ray.direction,
        global_transform,
    )
}
