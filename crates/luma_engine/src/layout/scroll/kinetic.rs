//! Kinetic momentum drag scrolling for touch and pointer interactions.
//!
//! Provides [`UScrollKineticDrag`], enabling fluid touch-style content dragging
//! with physics momentum, flick velocity, and exponential friction decay.

use bevy::prelude::*;

use crate::layout::query::{ComputedSize, UiPickingContext};
use crate::layout::scroll::container::UScrollContainer;
use crate::layout::scroll::pointer::ScrollPointerContext;
use crate::layout::scroll::scrollbar::UScrollbarThumb;
use crate::layout::univis_node::UNode;

/// Enables kinetic momentum drag scrolling when attached to a [`UScrollContainer`].
///
/// Users can click and drag directly on the container's content area. On release,
/// the scroll container retains velocity and glides with exponential friction deceleration.
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct UScrollKineticDrag {
    /// Whether kinetic dragging is enabled (default: true).
    pub enabled: bool,
    /// Friction factor that decelerates momentum after pointer release (higher = stops faster, default: 8.0).
    pub friction: f32,
    /// Maximum allowed kinetic velocity in logical pixels per second (default: 5000.0).
    pub max_velocity: f32,
    /// Velocity threshold below which kinetic motion stops completely (default: 4.0).
    pub min_velocity_threshold: f32,
    /// Current kinetic velocity vector (logical pixels per second).
    pub velocity: Vec2,
    /// Whether the container is currently being dragged.
    pub is_dragging: bool,
    /// Last observed pointer position in the container's local logical space.
    pub last_pointer_pos: Vec2,
    /// Smoothed instantaneous velocity vector during dragging.
    pub drag_velocity: Vec2,
}

impl Default for UScrollKineticDrag {
    fn default() -> Self {
        Self {
            enabled: true,
            friction: 8.0,
            max_velocity: 5000.0,
            min_velocity_threshold: 4.0,
            velocity: Vec2::ZERO,
            is_dragging: false,
            last_pointer_pos: Vec2::ZERO,
            drag_velocity: Vec2::ZERO,
        }
    }
}

impl UScrollKineticDrag {
    /// Creates a new kinetic drag configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets custom friction deceleration factor.
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction.max(0.1);
        self
    }

    /// Sets maximum allowed momentum velocity.
    pub fn with_max_velocity(mut self, max_velocity: f32) -> Self {
        self.max_velocity = max_velocity.max(100.0);
        self
    }

    /// Stops all kinetic momentum immediately.
    pub fn stop(&mut self) {
        self.velocity = Vec2::ZERO;
        self.drag_velocity = Vec2::ZERO;
        self.is_dragging = false;
    }
}

/// Reads pointer drag on scroll containers, tracks velocity, and applies kinetic momentum.
pub fn handle_kinetic_drag(
    time: Option<Res<Time>>,
    mouse_button: Option<Res<ButtonInput<MouseButton>>>,
    thumbs: Query<&UScrollbarThumb>,
    pointer: ScrollPointerContext,
    mut containers: Query<(
        Entity,
        &mut UScrollContainer,
        &mut UScrollKineticDrag,
        &GlobalTransform,
        &ComputedSize,
        &UNode,
        Option<&UiPickingContext>,
    )>,
) {
    let dt = time.map(|t| t.delta_secs().min(0.05)).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    // If any thumb is currently dragging, do not initiate container kinetic drag.
    let any_thumb_dragging = thumbs.iter().any(|t| t.is_dragging);

    let is_pressed = mouse_button
        .as_ref()
        .is_some_and(|mb| mb.pressed(MouseButton::Left));
    let just_pressed = mouse_button
        .as_ref()
        .is_some_and(|mb| mb.just_pressed(MouseButton::Left));
    let just_released = mouse_button
        .as_ref()
        .is_some_and(|mb| mb.just_released(MouseButton::Left));

    let active_windows = pointer.active_cursor_windows();

    let drag_winner = if just_pressed && !any_thumb_dragging && !active_windows.is_empty() {
        pointer.pick_unoccluded_hit(
            containers
                .iter()
                .filter(|(_, _, kinetic, ..)| kinetic.enabled)
                .map(|(entity, _, _, gt, computed, node, picking_ctx)| {
                    (entity, node, gt, computed, picking_ctx)
                }),
            &active_windows,
        )
    } else {
        None
    };

    for (entity, mut container, mut kinetic, gt, _computed, _node, picking_ctx) in
        containers.iter_mut()
    {
        if !kinetic.enabled {
            continue;
        }

        // 1. Release drag
        if (just_released || !is_pressed) && kinetic.is_dragging {
            kinetic.is_dragging = false;
            kinetic.velocity = kinetic.drag_velocity.clamp_length_max(kinetic.max_velocity);
        }

        // 2. Drag initiation on topmost unoccluded container
        let started_this_frame = if let Some(hit) = drag_winner
            && entity == hit.entity
        {
            kinetic.is_dragging = true;
            kinetic.last_pointer_pos = hit.local_hit_logical;
            kinetic.velocity = Vec2::ZERO;
            kinetic.drag_velocity = Vec2::ZERO;
            true
        } else {
            false
        };

        // 3. Active drag tracking (in node-local logical coordinates so 2D/3D/scaled UI match)
        if !started_this_frame
            && is_pressed
            && kinetic.is_dragging
            && !active_windows.is_empty()
            && let Some(local_cursor) = pointer.project_cursor_to_node_logical(
                entity,
                gt,
                picking_ctx,
                None,
                &active_windows,
            )
        {
            let delta = local_cursor - kinetic.last_pointer_pos;
            kinetic.last_pointer_pos = local_cursor;

            // Dragging UP (+Y in local UI plane) pulls content UP -> reveals bottom -> increases scroll offset
            let scroll_delta_y = if container.vertical { delta.y } else { 0.0 };
            // Dragging LEFT (-X in local UI plane) pulls content LEFT -> reveals right -> increases scroll offset
            let scroll_delta_x = if container.horizontal { -delta.x } else { 0.0 };

            let target = container.scroll_offset + Vec2::new(scroll_delta_x, scroll_delta_y);
            container.jump_to(target);

            // Sample instantaneous velocity
            if dt > 0.001 {
                let sample = Vec2::new(scroll_delta_x, scroll_delta_y) / dt;
                let blend = (18.0 * dt).min(1.0);
                kinetic.drag_velocity = kinetic.drag_velocity.lerp(sample, blend);
            }
        }

        // 4. Kinetic momentum glide & friction decay
        if !kinetic.is_dragging && kinetic.velocity.length_squared() > 0.0 {
            container.scroll_by(kinetic.velocity * dt);

            let decay = (-kinetic.friction * dt).exp();
            kinetic.velocity *= decay;

            if kinetic.velocity.length() < kinetic.min_velocity_threshold {
                kinetic.velocity = Vec2::ZERO;
            }

            // Snap momentum when reaching vertical bounds
            if container.is_at_top() && kinetic.velocity.y < 0.0 {
                kinetic.velocity.y = 0.0;
            }
            if container.is_at_bottom() && kinetic.velocity.y > 0.0 {
                kinetic.velocity.y = 0.0;
            }

            // Snap momentum when reaching horizontal bounds
            if container.scroll_offset.x <= 0.01 && kinetic.velocity.x < 0.0 {
                kinetic.velocity.x = 0.0;
            }
            if container.max_offset.x > 0.0
                && (container.scroll_offset.x - container.max_offset.x).abs() <= 0.5
                && kinetic.velocity.x > 0.0
            {
                kinetic.velocity.x = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinetic_drag_defaults() {
        let kd = UScrollKineticDrag::default();
        assert!(kd.enabled);
        assert_eq!(kd.friction, 8.0);
        assert_eq!(kd.max_velocity, 5000.0);
        assert_eq!(kd.velocity, Vec2::ZERO);
        assert!(!kd.is_dragging);
    }

    #[test]
    fn test_kinetic_friction_decay() {
        let mut kd = UScrollKineticDrag::default().with_friction(10.0);
        kd.velocity = Vec2::new(0.0, 1000.0);

        let dt = 0.1;
        let decay = (-kd.friction * dt).exp();
        kd.velocity *= decay;

        // e^(-1) ≈ 0.367879 -> 1000 * 0.367879 ≈ 367.88
        assert!((kd.velocity.y - 367.88).abs() < 1.0);
    }

    #[test]
    fn test_kinetic_stop() {
        let mut kd = UScrollKineticDrag::default();
        kd.velocity = Vec2::new(200.0, 500.0);
        kd.drag_velocity = Vec2::new(100.0, 300.0);
        kd.is_dragging = true;

        kd.stop();
        assert_eq!(kd.velocity, Vec2::ZERO);
        assert_eq!(kd.drag_velocity, Vec2::ZERO);
        assert!(!kd.is_dragging);
    }

    fn spawn_kinetic_test_camera(
        app: &mut App,
        target_window: Entity,
        perspective: bool,
        translation: Vec3,
    ) -> Entity {
        use bevy::camera::{
            CameraProjection, ComputedCameraValues, RenderTarget, RenderTargetInfo, Viewport,
        };
        use bevy::window::WindowRef;

        let mut camera = Camera::default();
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(800, 600),
                scale_factor: 1.0,
            }),
            clip_from_view: if perspective {
                PerspectiveProjection {
                    fov: core::f32::consts::FRAC_PI_2,
                    aspect_ratio: 800.0 / 600.0,
                    near: 0.1,
                    ..default()
                }
                .get_clip_from_view()
            } else {
                OrthographicProjection {
                    area: Rect::new(-400.0, -300.0, 400.0, 300.0),
                    ..OrthographicProjection::default_2d()
                }
                .get_clip_from_view()
            },
            ..default()
        };
        camera.viewport = Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(800, 600),
            ..default()
        });

        app.world_mut()
            .spawn((
                camera,
                RenderTarget::Window(WindowRef::Entity(target_window)),
                GlobalTransform::from(Transform::from_translation(translation)),
            ))
            .id()
    }

    #[test]
    fn test_kinetic_drag_respects_visibility_clip_occlusion_and_overlap() {
        use crate::layout::layout_system::UiSpace;
        use crate::layout::univis_node::UClip;
        use bevy::window::PrimaryWindow;

        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_systems(Update, handle_kinetic_drag);

        let mut win = Window::default();
        win.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        let win_e = app.world_mut().spawn((win, PrimaryWindow)).id();
        let cam_2d = spawn_kinetic_test_camera(&mut app, win_e, false, Vec3::new(0.0, 0.0, 100.0));

        // Hidden ancestor blocks child container kinetic drag.
        let hidden_parent = app.world_mut().spawn(Visibility::Hidden).id();
        let hidden_container = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UScrollKineticDrag::default(),
                UNode::default(),
                ComputedSize {
                    width: 300.0,
                    height: 300.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 10.0,
                    ..default()
                },
                ChildOf(hidden_parent),
            ))
            .id();

        // Clipping ancestor excludes cursor at (0, 0) when clip rect is centered at (200, 0).
        let clip_parent = app
            .world_mut()
            .spawn((
                UNode::default(),
                UClip::enabled(true),
                ComputedSize {
                    width: 100.0,
                    height: 100.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(200.0, 0.0, 0.0)),
            ))
            .id();
        let clipped_container = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UScrollKineticDrag::default(),
                UNode::default(),
                ComputedSize {
                    width: 300.0,
                    height: 300.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(100.0, 0.0, 0.0)),
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    clip_ancestor: Some(clip_parent),
                    local_depth_key: 9.0,
                    ..default()
                },
                ChildOf(clip_parent),
            ))
            .id();

        // Visible lower & upper containers plus an opaque modal blocker above both.
        let lower = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UScrollKineticDrag::default(),
                UNode::default(),
                ComputedSize {
                    width: 300.0,
                    height: 300.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 1.0,
                    ..default()
                },
            ))
            .id();

        let upper = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UScrollKineticDrag::default(),
                UNode::default(),
                ComputedSize {
                    width: 300.0,
                    height: 300.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 2.0,
                    ..default()
                },
            ))
            .id();

        let blocker = app
            .world_mut()
            .spawn((
                UNode {
                    background_color: Color::WHITE,
                    ..default()
                },
                ComputedSize {
                    width: 120.0,
                    height: 120.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    space: UiSpace::Screen,
                    local_depth_key: 5.0,
                    ..default()
                },
            ))
            .id();

        // Click while blocker is over center: nothing initiates drag.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        for entity in [hidden_container, clipped_container, lower, upper] {
            assert!(
                !app.world().get::<UScrollKineticDrag>(entity).unwrap().is_dragging,
                "Container {:?} must not initiate drag when blocked/hidden/clipped",
                entity
            );
        }

        // Release and hide blocker -> now upper (depth 2.0) must win over lower (depth 1.0).
        {
            let mut mouse = app
                .world_mut()
                .resource_mut::<ButtonInput<MouseButton>>();
            mouse.release(MouseButton::Left);
            mouse.clear();
        }
        app.world_mut()
            .entity_mut(blocker)
            .insert(Visibility::Hidden);
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        assert!(
            app.world().get::<UScrollKineticDrag>(upper).unwrap().is_dragging,
            "Upper visible unoccluded container should initiate kinetic drag"
        );
        assert!(
            !app.world().get::<UScrollKineticDrag>(lower).unwrap().is_dragging,
            "Lower overlapped container must not initiate kinetic drag"
        );
        assert!(
            !app.world()
                .get::<UScrollKineticDrag>(hidden_container)
                .unwrap()
                .is_dragging
        );
        assert!(
            !app.world()
                .get::<UScrollKineticDrag>(clipped_container)
                .unwrap()
                .is_dragging
        );

        // Move cursor UP 60px and continue holding -> only upper scrolls by +60.0 logical px.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear_just_pressed(MouseButton::Left);
        app.world_mut()
            .get_mut::<Window>(win_e)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(400.0, 240.0)));
        app.update();

        assert!(
            (app.world().get::<UScrollContainer>(upper).unwrap().scroll_offset.y - 60.0).abs()
                < 1e-3
        );
        assert_eq!(
            app.world().get::<UScrollContainer>(lower).unwrap().scroll_offset.y,
            0.0
        );
    }

    #[test]
    fn test_kinetic_drag_supports_secondary_window_and_3d_perspective_scaled_panel() {
        use crate::layout::layout_system::UiSpace;
        use bevy::window::PrimaryWindow;

        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_systems(Update, handle_kinetic_drag);

        // Primary window has no cursor; secondary window holds cursor.
        let win_a = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
        let mut win_b = Window::default();
        win_b.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        let win_b_e = app.world_mut().spawn(win_b).id();

        let _cam_a = spawn_kinetic_test_camera(&mut app, win_a, false, Vec3::new(0.0, 0.0, 100.0));
        let cam_b_3d =
            spawn_kinetic_test_camera(&mut app, win_b_e, true, Vec3::new(0.0, 0.0, 2.0));

        // World3d panel at z=0: world_scale=0.01 -> 200x200 logical px = 2.0x2.0 world units.
        let panel_3d = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UScrollKineticDrag::default(),
                UNode::default(),
                ComputedSize {
                    width: 200.0,
                    height: 200.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_b_3d),
                    space: UiSpace::World3d,
                    world_scale: 0.01,
                    ..default()
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        assert!(
            app.world()
                .get::<UScrollKineticDrag>(panel_3d)
                .unwrap()
                .is_dragging
        );

        // Perspective fov=90 deg at z=2.0 -> half-height = 2.0 world units = 300 screen px.
        // Moving cursor UP 100 screen px (300 -> 200) shifts hit +2/3 world units = +66.6667 logical px.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear_just_pressed(MouseButton::Left);
        app.world_mut()
            .get_mut::<Window>(win_b_e)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(400.0, 200.0)));
        app.update();

        let offset_y = app
            .world()
            .get::<UScrollContainer>(panel_3d)
            .unwrap()
            .scroll_offset
            .y;
        assert!(
            (offset_y - 200.0 / 3.0).abs() < 0.25,
            "Expected ~66.67 logical px scroll in World3d perspective panel, got {offset_y}"
        );
    }
}
