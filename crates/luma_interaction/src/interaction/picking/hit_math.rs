use bevy::picking::pointer::Location;
use bevy::prelude::{Camera, Dir3, Entity, GlobalTransform, Query, Vec2, Vec3};

#[derive(Clone, Copy)]
pub(super) struct CachedPointerRay {
    pub(super) order: f32,
    pub(super) origin: Vec3,
    pub(super) direction: Vec3,
}

pub(super) fn pointer_ray_for_camera(
    location: &Location,
    camera_entity: Entity,
    cameras: &Query<(Entity, &Camera, &GlobalTransform)>,
) -> Option<CachedPointerRay> {
    let Ok((_, camera, camera_transform)) = cameras.get(camera_entity) else {
        return None;
    };

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
    let plane_origin = global_transform.translation();
    let z_axis = global_transform.affine().matrix3.z_axis;
    let plane_normal = Dir3::new(z_axis.into())
        .map(|d| d.as_vec3())
        .unwrap_or(Vec3::Z);
    let denominator = ray.direction.dot(plane_normal);

    if denominator.abs() <= f32::EPSILON {
        return None;
    }

    let distance = (plane_origin - ray.origin).dot(plane_normal) / denominator;
    if distance < 0.0 {
        return None;
    }

    let world_hit = ray.origin + ray.direction * distance;
    let local_hit = global_transform
        .to_matrix()
        .inverse()
        .transform_point3(world_hit)
        .truncate();

    Some((world_hit, local_hit, plane_normal, distance))
}
