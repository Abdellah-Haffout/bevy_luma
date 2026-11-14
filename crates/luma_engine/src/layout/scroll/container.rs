//! Scroll container and viewport management for Univis UI.
//!
//! Provides [`UScrollContainer`] and [`UScrollContent`] for smooth mouse-wheel
//! driven scrolling, automatic extent calculation, and hardware-accelerated
//! rounded SDF clipping via [`UClip`].

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::layout::query::{ComputedSize, UiPickingContext};
use crate::layout::scroll::pointer::{ScrollHitCandidate, ScrollPointerContext};
use crate::layout::univis_node::{UClip, UNode};

/// Configures a scrollable viewport container.
///
/// When attached to an entity with a [`UNode`], overflowing child content marked
/// with [`UScrollContent`] (or the first child node) glides smoothly using mouse
/// wheel input or programmatic target offsets.
///
/// Attaching `UScrollContainer` automatically requires [`UNode`] and [`UClip`] (enabled by default).
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
#[require(UNode, UClip { enabled: true })]
pub struct UScrollContainer {
    /// Whether vertical scrolling is enabled (default: true).
    pub vertical: bool,
    /// Whether horizontal scrolling is enabled (default: false).
    pub horizontal: bool,
    /// Current rendered scroll offset in logical pixels (x = horizontal, y = vertical).
    pub scroll_offset: Vec2,
    /// Target scroll offset for smooth interpolation.
    pub target_offset: Vec2,
    /// Maximum allowed scroll offset computed from (content_size - container_size).
    pub max_offset: Vec2,
    /// Scroll sensitivity factor per mouse wheel tick (default: 40.0 pixels per notch).
    pub sensitivity: f32,
    /// Smoothing responsiveness factor (higher = faster glide, default: 18.0).
    pub smoothness: f32,
    /// Explicit hover flag for custom pointer integrations.
    pub is_hovered: bool,
}

impl Default for UScrollContainer {
    fn default() -> Self {
        Self {
            vertical: true,
            horizontal: false,
            scroll_offset: Vec2::ZERO,
            target_offset: Vec2::ZERO,
            max_offset: Vec2::ZERO,
            sensitivity: 40.0,
            smoothness: 18.0,
            is_hovered: false,
        }
    }
}

impl UScrollContainer {
    /// Creates a vertical scroll container.
    pub fn vertical() -> Self {
        Self {
            vertical: true,
            horizontal: false,
            ..Default::default()
        }
    }

    /// Creates a horizontal scroll container.
    pub fn horizontal() -> Self {
        Self {
            vertical: false,
            horizontal: true,
            ..Default::default()
        }
    }

    /// Creates a bidirectional scroll container (both vertical and horizontal).
    pub fn both() -> Self {
        Self {
            vertical: true,
            horizontal: true,
            ..Default::default()
        }
    }

    /// Sets custom scroll sensitivity (pixels per wheel notch).
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = sensitivity;
        self
    }

    /// Sets custom smoothing factor (higher = snappier glide).
    pub fn with_smoothness(mut self, smoothness: f32) -> Self {
        self.smoothness = smoothness;
        self
    }

    /// Sets the target scroll offset immediately, clamped to `max_offset`.
    pub fn scroll_to(&mut self, target: Vec2) {
        let x = if self.horizontal {
            target.x.clamp(0.0, self.max_offset.x)
        } else {
            0.0
        };
        let y = if self.vertical {
            target.y.clamp(0.0, self.max_offset.y)
        } else {
            0.0
        };
        self.target_offset = Vec2::new(x, y);
    }

    /// Adjusts the target scroll offset by a relative delta.
    pub fn scroll_by(&mut self, delta: Vec2) {
        self.scroll_to(self.target_offset + delta);
    }

    /// Instantly snaps to the target offset without smooth interpolation.
    pub fn jump_to(&mut self, target: Vec2) {
        self.scroll_to(target);
        self.scroll_offset = self.target_offset;
    }

    /// Returns true if vertical scrolling is at the top (offset == 0.0).
    pub fn is_at_top(&self) -> bool {
        self.scroll_offset.y <= 0.01
    }

    /// Returns true if vertical scrolling reached the bottom extent.
    pub fn is_at_bottom(&self) -> bool {
        self.max_offset.y > 0.0 && (self.scroll_offset.y - self.max_offset.y).abs() <= 0.5
    }

    /// Returns normalized vertical scroll progress in 0.0..=1.0.
    pub fn progress_y(&self) -> f32 {
        if self.max_offset.y > 0.0 {
            (self.scroll_offset.y / self.max_offset.y).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Returns normalized horizontal scroll progress in 0.0..=1.0.
    pub fn progress_x(&self) -> f32 {
        if self.max_offset.x > 0.0 {
            (self.scroll_offset.x / self.max_offset.x).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Marks an entity as the scrollable content inside a [`UScrollContainer`].
///
/// When attached to a child of a `UScrollContainer`, its layout position is preserved
/// as `base_translation`, and its rendered transform is smoothly offset by the container's
/// scroll offset.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq)]
#[reflect(Component)]
pub struct UScrollContent {
    /// Cached base translation established by the downward layout solver.
    pub base_translation: Vec2,
    /// Whether the base translation has been initialized.
    pub initialized: bool,
}

impl UScrollContent {
    /// Creates a new `UScrollContent` marker.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Automatically ensures [`UClip`] is enabled when [`UScrollContainer`] is added.
pub fn init_scroll_containers(
    mut commands: Commands,
    query: Query<(Entity, Option<&UClip>), Added<UScrollContainer>>,
) {
    for (entity, clip) in query.iter() {
        if clip.is_none_or(|c| !c.enabled) {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_insert(UClip { enabled: true });
            }
        }
    }
}

/// Automatically attaches [`UScrollContent`] to the first child of a [`UScrollContainer`] if missing.
pub fn auto_init_scroll_content(
    mut commands: Commands,
    containers: Query<(Entity, &Children), With<UScrollContainer>>,
    all_content: Query<(), With<UScrollContent>>,
    unode_query: Query<(), With<UNode>>,
) {
    for (_container_entity, children) in containers.iter() {
        let has_content_child = children.iter().any(|c| all_content.contains(c));
        if !has_content_child {
            let Some(&first_child) = children.first().filter(|&&c| unode_query.contains(c)) else {
                continue;
            };
            if let Ok(mut cmd) = commands.get_entity(first_child) {
                cmd.try_insert(UScrollContent::default());
            }
        }
    }
}

/// Synchronizes container maximum scroll extents from container and content computed sizes.
pub fn sync_scroll_extents(
    mut containers: Query<(Entity, &mut UScrollContainer, &ComputedSize)>,
    content_query: Query<(&UScrollContent, &ComputedSize, &ChildOf)>,
) {
    for (_content, content_size, parent) in content_query.iter() {
        if let Ok((_entity, mut container, container_size)) = containers.get_mut(parent.parent()) {
            let overflow_y = (content_size.height - container_size.height).max(0.0);
            let overflow_x = (content_size.width - container_size.width).max(0.0);
            container.max_offset = Vec2::new(overflow_x, overflow_y);

            // Clamp offsets when extents change
            container.target_offset = container
                .target_offset
                .clamp(Vec2::ZERO, container.max_offset);
            container.scroll_offset = container
                .scroll_offset
                .clamp(Vec2::ZERO, container.max_offset);
        }
    }
}

/// Reads mouse wheel input and updates target scroll offsets on the topmost visible, unoccluded scroll container.
pub fn handle_mouse_wheel_scroll(
    mouse_wheel: Option<MessageReader<MouseWheel>>,
    mut containers: Query<(
        Entity,
        &mut UScrollContainer,
        &UNode,
        &GlobalTransform,
        &ComputedSize,
        Option<&UiPickingContext>,
    )>,
    pointer: ScrollPointerContext,
) {
    let Some(mut mouse_wheel) = mouse_wheel else {
        return;
    };

    if mouse_wheel.is_empty() {
        return;
    }

    for ev in mouse_wheel.read() {
        let (window_entity_opt, primary_window, cursor_pos_opt) =
            pointer.resolve_window(ev.window);

        let mut best_candidate: Option<ScrollHitCandidate> = None;

        for (entity, container, node, global_transform, computed, picking_ctx) in containers.iter()
        {
            if !pointer.is_visible(entity) {
                continue;
            }

            let cursor_hit = if let (Some(window_entity), Some(cursor_pos)) =
                (window_entity_opt, cursor_pos_opt)
            {
                pointer.hit_test_node(
                    entity,
                    node,
                    global_transform,
                    computed,
                    picking_ctx,
                    window_entity,
                    primary_window,
                    cursor_pos,
                )
            } else {
                None
            };

            let candidate = if let Some(hit) = cursor_hit {
                pointer.candidate_rank(
                    entity,
                    hit.camera_order,
                    hit.hit_distance,
                    true,
                    global_transform,
                    picking_ctx,
                )
            } else if container.is_hovered {
                let cam_order = pointer.camera_order(picking_ctx);
                pointer.candidate_rank(
                    entity,
                    cam_order,
                    0.0,
                    false,
                    global_transform,
                    picking_ctx,
                )
            } else {
                continue;
            };

            if best_candidate
                .as_ref()
                .is_none_or(|best| candidate.rank_cmp(best) == std::cmp::Ordering::Greater)
            {
                best_candidate = Some(candidate);
            }
        }

        let Some(winner) = best_candidate else {
            continue;
        };

        if winner.hit_by_cursor
            && let (Some(window_entity), Some(cursor_pos)) = (window_entity_opt, cursor_pos_opt)
            && pointer.is_hit_occluded(&winner, window_entity, primary_window, cursor_pos)
        {
            continue;
        }

        let Ok((_, mut container, _, _, _, _)) = containers.get_mut(winner.entity) else {
            continue;
        };

        let delta = match ev.unit {
            MouseScrollUnit::Line => Vec2::new(ev.x, ev.y) * container.sensitivity,
            MouseScrollUnit::Pixel => Vec2::new(ev.x, ev.y),
        };

        if container.vertical {
            // Scrolling wheel down (ev.y < 0) reveals lower content: target offset increases
            container.target_offset.y =
                (container.target_offset.y - delta.y).clamp(0.0, container.max_offset.y);
        }

        if container.horizontal {
            // For horizontal-only containers, allow standard vertical mouse wheel notches to scroll horizontally
            let h_delta = if !container.vertical && ev.x.abs() < 0.001 {
                delta.y
            } else {
                delta.x
            };
            container.target_offset.x =
                (container.target_offset.x - h_delta).clamp(0.0, container.max_offset.x);
        }
    }
}

/// Smoothly interpolates scroll offset towards target offset and updates the content transform.
pub fn apply_scroll_transitions(
    time: Option<Res<Time>>,
    mut containers: Query<&mut UScrollContainer>,
    mut content_query: Query<(&UScrollContent, &mut Transform, &ChildOf)>,
) {
    let dt = time.map(|t| t.delta_secs().min(0.05)).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    for (content, mut transform, parent) in content_query.iter_mut() {
        let Ok(mut container) = containers.get_mut(parent.parent()) else {
            continue;
        };

        if !content.initialized {
            continue;
        }

        // Smooth glide towards target offset
        let diff = container.target_offset - container.scroll_offset;
        if diff.length_squared() < 0.0001 {
            container.scroll_offset = container.target_offset;
        } else {
            let blend = 1.0 - (-container.smoothness * dt).exp();
            container.scroll_offset += diff * blend;
        }

        let effective_x = if container.horizontal {
            container.scroll_offset.x
        } else {
            0.0
        };
        let effective_y = if container.vertical {
            container.scroll_offset.y
        } else {
            0.0
        };

        transform.translation.x = content.base_translation.x - effective_x;
        transform.translation.y = content.base_translation.y + effective_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::RenderTarget;
    use bevy::window::PrimaryWindow;

    #[test]
    fn test_scroll_container_constructors() {
        let v = UScrollContainer::vertical();
        assert!(v.vertical);
        assert!(!v.horizontal);
        assert_eq!(v.sensitivity, 40.0);

        let h = UScrollContainer::horizontal();
        assert!(!h.vertical);
        assert!(h.horizontal);

        let b = UScrollContainer::both();
        assert!(b.vertical);
        assert!(b.horizontal);
    }

    #[test]
    fn test_scroll_to_and_by_clamps() {
        let mut sc = UScrollContainer::vertical();
        sc.max_offset = Vec2::new(100.0, 500.0);

        sc.scroll_to(Vec2::new(50.0, 200.0));
        assert_eq!(sc.target_offset, Vec2::new(0.0, 200.0)); // horizontal disabled

        sc.scroll_by(Vec2::new(0.0, 400.0));
        assert_eq!(sc.target_offset.y, 500.0); // clamped to max

        sc.scroll_by(Vec2::new(0.0, -600.0));
        assert_eq!(sc.target_offset.y, 0.0); // clamped to 0
    }

    #[test]
    fn test_scroll_progress() {
        let mut sc = UScrollContainer::vertical();
        sc.max_offset.y = 200.0;
        assert!(sc.is_at_top());
        assert_eq!(sc.progress_y(), 0.0);

        sc.scroll_offset.y = 100.0;
        assert_eq!(sc.progress_y(), 0.5);

        sc.scroll_offset.y = 200.0;
        assert!(sc.is_at_bottom());
        assert_eq!(sc.progress_y(), 1.0);
    }

    #[test]
    fn test_scroll_progress_horizontal() {
        let mut sc = UScrollContainer::horizontal();
        sc.max_offset.x = 400.0;
        assert_eq!(sc.progress_x(), 0.0);

        sc.scroll_offset.x = 200.0;
        assert_eq!(sc.progress_x(), 0.5);

        sc.scroll_offset.x = 400.0;
        assert_eq!(sc.progress_x(), 1.0);
    }

    #[test]
    fn test_scroll_smooth_gliding() {
        let mut sc = UScrollContainer::vertical().with_smoothness(20.0);
        sc.max_offset.y = 400.0;
        sc.scroll_to(Vec2::new(0.0, 200.0));

        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            let diff = sc.target_offset - sc.scroll_offset;
            let blend = 1.0 - (-sc.smoothness * dt).exp();
            sc.scroll_offset += diff * blend;
        }

        assert!(
            (sc.scroll_offset.y - 200.0).abs() < 1.0,
            "Scroll offset failed to glide to target: offset = {}",
            sc.scroll_offset.y
        );
    }

    fn spawn_scroll_test_camera(
        app: &mut App,
        target_window: Entity,
        perspective: bool,
        translation: Vec3,
    ) -> Entity {
        use bevy::camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo, Viewport};
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

    fn scroll_test_picking_context(
        camera_entity: Entity,
        root_sort_key: f32,
        local_depth_key: f32,
        clip_ancestor: Option<Entity>,
    ) -> UiPickingContext {
        UiPickingContext {
            root_entity: Some(Entity::PLACEHOLDER),
            camera_entity: Some(camera_entity),
            clip_ancestor,
            root_sort_key,
            local_depth_key,
            ..default()
        }
    }

    #[test]
    fn mouse_wheel_scrolls_only_topmost_visible_unoccluded_container_and_respects_window_and_3d() {
        use crate::layout::geometry::UVal;
        use crate::layout::layout_system::UiSpace;

        let mut app = App::new();
        app.add_message::<MouseWheel>();
        app.add_systems(Update, handle_mouse_wheel_scroll);

        let mut win_a = Window::default();
        win_a.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        let window_a = app.world_mut().spawn((win_a, PrimaryWindow)).id();

        let mut win_b = Window::default();
        win_b.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        let window_b = app.world_mut().spawn(win_b).id();

        let cam_2d = spawn_scroll_test_camera(&mut app, window_a, false, Vec3::new(0.0, 0.0, 100.0));
        let cam_3d = spawn_scroll_test_camera(&mut app, window_b, true, Vec3::new(0.0, 0.0, 5.0));

        // 1. Lower and upper overlapping containers on window_a.
        let lower = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UNode {
                    width: UVal::Px(300.0),
                    height: UVal::Px(300.0),
                    ..default()
                },
                ComputedSize {
                    width: 300.0,
                    height: 300.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
                scroll_test_picking_context(cam_2d, 0.0, 0.001, None),
            ))
            .id();

        let upper = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UNode {
                    width: UVal::Px(300.0),
                    height: UVal::Px(300.0),
                    ..default()
                },
                ComputedSize {
                    width: 300.0,
                    height: 300.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 1.0)),
                scroll_test_picking_context(cam_2d, 1.0, 0.001, None),
            ))
            .id();

        // Transparent fullscreen HUD wrapper on highest root must NOT block scrolling (overlapping_roots pattern).
        let _transparent_hud = app
            .world_mut()
            .spawn((
                UNode {
                    background_color: Color::NONE,
                    ..default()
                },
                ComputedSize {
                    width: 800.0,
                    height: 600.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 5.0)),
                scroll_test_picking_context(cam_2d, 10.0, 0.001, None),
            ))
            .id();

        // Solid descendant inside `upper` must NOT block scrolling `upper`.
        let _inner_content_card = app
            .world_mut()
            .spawn((
                ChildOf(upper),
                UNode {
                    background_color: Color::WHITE,
                    ..default()
                },
                ComputedSize {
                    width: 280.0,
                    height: 280.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 1.002)),
                scroll_test_picking_context(cam_2d, 1.0, 0.005, None),
            ))
            .id();

        // Hidden and clipped containers above `upper` must be ignored.
        let _hidden_top = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                Visibility::Hidden,
                ComputedSize {
                    width: 300.0,
                    height: 300.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 2.0)),
                scroll_test_picking_context(cam_2d, 2.0, 0.001, None),
            ))
            .id();

        let clipper = app
            .world_mut()
            .spawn((
                UNode::default(),
                ComputedSize {
                    width: 50.0,
                    height: 50.0,
                    local_pos: Vec2::ZERO,
                },
                UClip::enabled(true),
                GlobalTransform::from(Transform::from_xyz(200.0, 0.0, 3.0)),
            ))
            .id();
        let clipped_top = app
            .world_mut()
            .spawn((
                ChildOf(clipper),
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                ComputedSize {
                    width: 500.0,
                    height: 500.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 3.0)),
                scroll_test_picking_context(cam_2d, 3.0, 0.001, Some(clipper)),
            ))
            .id();

        // Window B has a tilted 3D scroll container under a perspective camera.
        let world3d_container = app
            .world_mut()
            .spawn((
                UScrollContainer {
                    max_offset: Vec2::new(0.0, 500.0),
                    ..UScrollContainer::vertical()
                },
                UNode::default(),
                ComputedSize {
                    width: 4.0,
                    height: 4.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(
                    Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_y(0.25)),
                ),
                UiPickingContext {
                    space: UiSpace::World3d,
                    camera_entity: Some(cam_3d),
                    ..default()
                },
            ))
            .id();

        // Scroll on window_a -> only `upper` should scroll.
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: window_a,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
        app.update();

        assert_eq!(app.world().entity(upper).get::<UScrollContainer>().unwrap().target_offset.y, 40.0);
        assert_eq!(app.world().entity(lower).get::<UScrollContainer>().unwrap().target_offset.y, 0.0);
        assert_eq!(
            app.world().entity(clipped_top).get::<UScrollContainer>().unwrap().target_offset.y,
            0.0
        );
        assert_eq!(
            app.world()
                .entity(world3d_container)
                .get::<UScrollContainer>()
                .unwrap()
                .target_offset
                .y,
            0.0
        );

        // Spawning a solid non-descendant panel above `upper` blocks scrolling `upper`.
        let solid_modal = app
            .world_mut()
            .spawn((
                UNode {
                    background_color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                    ..default()
                },
                ComputedSize {
                    width: 400.0,
                    height: 400.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 4.0)),
                scroll_test_picking_context(cam_2d, 5.0, 0.001, None),
            ))
            .id();

        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: window_a,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
        app.update();
        assert_eq!(
            app.world().entity(upper).get::<UScrollContainer>().unwrap().target_offset.y,
            40.0,
            "solid non-descendant panel above scroll container should block wheel scroll"
        );

        // Hiding the modal restores scrolling on `upper`, and scrolling window_b scrolls 3D container.
        *app.world_mut().entity_mut(solid_modal).get_mut::<Visibility>().unwrap() =
            Visibility::Hidden;
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: window_b,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
        app.update();
        assert_eq!(
            app.world()
                .entity(world3d_container)
                .get::<UScrollContainer>()
                .unwrap()
                .target_offset
                .y,
            40.0
        );
        assert_eq!(app.world().entity(upper).get::<UScrollContainer>().unwrap().target_offset.y, 40.0);
    }
}
