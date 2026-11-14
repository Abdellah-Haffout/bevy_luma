use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::window::WindowRef;

use super::{URootUi, UiCameraRef, UiCanvasSize, UiSpace};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RootResolutionIssue {
    #[default]
    None,
    AutoMissingCamera,
    AutoAmbiguousCamera,
    ExplicitCameraMissing,
    ExplicitCameraInactive,
    ViewportSizeUnavailable,
}

impl RootResolutionIssue {
    fn reason(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::AutoMissingCamera => {
                Some("no active compatible camera matched `UiCameraRef::Auto`")
            }
            Self::AutoAmbiguousCamera => {
                Some("multiple active compatible cameras matched `UiCameraRef::Auto`")
            }
            Self::ExplicitCameraMissing => {
                Some("the configured camera entity is missing or does not contain `Camera`")
            }
            Self::ExplicitCameraInactive => Some("the configured camera entity is inactive"),
            Self::ViewportSizeUnavailable => {
                Some("logical viewport size was not available from the resolved camera")
            }
        }
    }

    fn action(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::AutoMissingCamera => {
                Some("spawn one active compatible camera or bind `UiCameraRef::Entity` explicitly")
            }
            Self::AutoAmbiguousCamera => {
                Some("bind `UiCameraRef::Entity` explicitly for this root")
            }
            Self::ExplicitCameraMissing => Some("update the root to point at a live camera entity"),
            Self::ExplicitCameraInactive => {
                Some("activate the target camera or bind a different active camera")
            }
            Self::ViewportSizeUnavailable => {
                Some("wait for the camera viewport to initialize or provide a fixed canvas size")
            }
        }
    }
}

pub(super) fn normalize_meters_per_unit(value: f32) -> f32 {
    if value.is_finite() && value > f32::EPSILON {
        value
    } else {
        URootUi::DEFAULT_METERS_PER_UNIT
    }
}

#[cfg(test)]
pub(super) fn resolve_root_camera(
    camera_ref: UiCameraRef,
    space: UiSpace,
    cameras: &Query<(
        Entity,
        &Camera,
        Option<&RenderTarget>,
        Has<Camera2d>,
        Has<Camera3d>,
    )>,
) -> (Option<Entity>, RootResolutionIssue) {
    resolve_root_camera_for_primary_window(camera_ref, space, cameras, None)
}

pub(super) fn resolve_root_camera_for_primary_window(
    camera_ref: UiCameraRef,
    space: UiSpace,
    cameras: &Query<(
        Entity,
        &Camera,
        Option<&RenderTarget>,
        Has<Camera2d>,
        Has<Camera3d>,
    )>,
    primary_window: Option<Entity>,
) -> (Option<Entity>, RootResolutionIssue) {
    match camera_ref {
        UiCameraRef::Entity(entity) => match cameras.get(entity) {
            Ok((_, camera, _, _, _)) if camera.is_active => {
                (Some(entity), RootResolutionIssue::None)
            }
            Ok(_) => (None, RootResolutionIssue::ExplicitCameraInactive),
            Err(_) => (None, RootResolutionIssue::ExplicitCameraMissing),
        },
        UiCameraRef::Auto => {
            let candidates: Vec<(Entity, bool, bool)> = cameras
                .iter()
                .filter_map(|(entity, camera, render_target, is_2d, is_3d)| {
                    // Only cameras targeting the primary window participate in `Auto` resolution;
                    // secondary-window and offscreen cameras must be targeted via `UiCameraRef::Entity`.
                    let targets_primary_window = match render_target {
                        None | Some(RenderTarget::Window(WindowRef::Primary)) => true,
                        Some(RenderTarget::Window(WindowRef::Entity(win))) => {
                            primary_window == Some(*win)
                        }
                        _ => false,
                    };
                    (camera.is_active && targets_primary_window).then_some((entity, is_2d, is_3d))
                })
                .collect();

            match candidates.as_slice() {
                [] => (None, RootResolutionIssue::AutoMissingCamera),
                [(entity, _, _)] => (Some(*entity), RootResolutionIssue::None),
                many => {
                    // Disambiguate cleanly when a scene mixes a single 2D camera and 3D camera(s),
                    // or a single 3D camera and 2D HUD camera(s) on window render targets.
                    let prefer_2d = matches!(space, UiSpace::Screen | UiSpace::World2d);
                    let mut preferred = many.iter().copied().filter(|(_, is_2d, is_3d)| {
                        if prefer_2d {
                            *is_2d && !*is_3d
                        } else {
                            *is_3d && !*is_2d
                        }
                    });
                    let first_pref = preferred.next();
                    let second_pref = preferred.next();
                    let others_are_complementary = many.iter().all(|(_, is_2d, is_3d)| {
                        if prefer_2d {
                            (*is_2d && !*is_3d) || (*is_3d && !*is_2d)
                        } else {
                            (*is_3d && !*is_2d) || (*is_2d && !*is_3d)
                        }
                    });

                    if let (Some((entity, _, _)), None) = (first_pref, second_pref)
                        && others_are_complementary
                    {
                        (Some(entity), RootResolutionIssue::None)
                    } else {
                        (None, RootResolutionIssue::AutoAmbiguousCamera)
                    }
                }
            }
        }
    }
}

pub(super) fn resolve_root_canvas_size(
    canvas: UiCanvasSize,
    camera_entity: Option<Entity>,
    cameras: &Query<(Entity, &Camera)>,
    windows: &Query<&Window>,
    issue: &mut RootResolutionIssue,
) -> Vec2 {
    match canvas {
        UiCanvasSize::Fixed(size) => size,
        UiCanvasSize::FitContent { min, max } => clamp_canvas_size(min.max(Vec2::ZERO), min, max),
        UiCanvasSize::Viewport => resolve_viewport_size(camera_entity, cameras, windows, issue),
        UiCanvasSize::Virtual { reference, mode } => {
            let viewport = resolve_viewport_size(camera_entity, cameras, windows, issue);
            let scale = mode.compute_scale_factor(viewport, reference);
            viewport / scale
        }
    }
}

fn resolve_viewport_size(
    camera_entity: Option<Entity>,
    cameras: &Query<(Entity, &Camera)>,
    windows: &Query<&Window>,
    issue: &mut RootResolutionIssue,
) -> Vec2 {
    if let Some(camera_entity) = camera_entity {
        if let Ok((_, camera)) = cameras.get(camera_entity)
            && let Some(size) = camera.logical_viewport_size()
        {
            return size;
        }

        if *issue == RootResolutionIssue::None {
            *issue = RootResolutionIssue::ViewportSizeUnavailable;
        }
    }

    if let Ok(window) = windows.single() {
        Vec2::new(window.width(), window.height())
    } else {
        Vec2::new(800.0, 600.0)
    }
}

pub(super) fn clamp_canvas_size(size: Vec2, min: Vec2, max: Option<Vec2>) -> Vec2 {
    let mut clamped = Vec2::new(size.x.max(min.x), size.y.max(min.y));

    if let Some(max) = max {
        clamped.x = clamped.x.min(max.x);
        clamped.y = clamped.y.min(max.y);
    }

    clamped
}

pub(super) fn emit_root_resolution_warning(
    entity: Entity,
    space: UiSpace,
    issue: RootResolutionIssue,
) {
    let Some(reason) = issue.reason() else {
        return;
    };
    let action = issue.action().unwrap_or("inspect the root configuration");

    bevy::log::warn!(
        "[layout/root_resolution] root={:?} space={:?} reason={} action={}",
        entity,
        space,
        reason,
        action
    );
}
