//! Visual scrollbar primitives: tracks, thumbs, auto-hide fading, and drag control.
//!
//! Provides [`UScrollbarTrack`], [`UScrollbarThumb`], and [`UScrollbarFade`].
//! These components are unopinionated ECS primitives that can be attached to any
//! [`UNode`] to build fully custom, hardware-accelerated
//! scrollbars with proportional thumb sizing, synchronous drag-to-scroll, and track click-to-jump.

use bevy::prelude::*;

use crate::layout::query::{ComputedSize, UiPickingContext};
use crate::layout::scroll::container::UScrollContainer;
use crate::layout::scroll::pointer::ScrollPointerContext;
use crate::layout::univis_node::UNode;

/// Scrollbar orientation axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
pub enum UScrollbarAxis {
    /// Vertical scrollbar (glides along Y axis).
    #[default]
    Vertical,
    /// Horizontal scrollbar (glides along X axis).
    Horizontal,
}

/// Marks an entity as a scrollbar track.
///
/// Clicking on the track can immediately jump or page the associated [`UScrollContainer`].
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
#[require(UNode)]
pub struct UScrollbarTrack {
    /// The target scroll container entity.
    pub container: Entity,
    /// Orientation axis of the track.
    pub axis: UScrollbarAxis,
    /// If true, clicking on the track jumps the scroll offset to the clicked position.
    pub click_to_jump: bool,
    /// If true, hides the track when the container content does not overflow.
    pub hide_when_not_scrollable: bool,
}

impl UScrollbarTrack {
    /// Creates a vertical scrollbar track targeting the given container.
    pub fn vertical(container: Entity) -> Self {
        Self {
            container,
            axis: UScrollbarAxis::Vertical,
            click_to_jump: true,
            hide_when_not_scrollable: false,
        }
    }

    /// Creates a horizontal scrollbar track targeting the given container.
    pub fn horizontal(container: Entity) -> Self {
        Self {
            container,
            axis: UScrollbarAxis::Horizontal,
            click_to_jump: true,
            hide_when_not_scrollable: false,
        }
    }

    /// Enables or disables click-to-jump behavior.
    pub fn with_click_to_jump(mut self, click_to_jump: bool) -> Self {
        self.click_to_jump = click_to_jump;
        self
    }

    /// Enables or disables automatic hiding when content does not overflow.
    pub fn with_hide_when_not_scrollable(mut self, hide: bool) -> Self {
        self.hide_when_not_scrollable = hide;
        self
    }
}

/// Marks an entity as a scrollbar thumb slider.
///
/// Typically spawned as a child of a [`UScrollbarTrack`]. Its size automatically
/// reflects the viewport-to-content ratio, and dragging it smoothly scrolls
/// the target [`UScrollContainer`].
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
#[require(UNode)]
pub struct UScrollbarThumb {
    /// The target scroll container entity.
    pub container: Entity,
    /// Explicit track entity if not the immediate parent.
    pub track: Option<Entity>,
    /// Movement axis.
    pub axis: UScrollbarAxis,
    /// Minimum size of the thumb along its scroll axis in logical pixels (default: 20.0).
    pub min_size: f32,
    /// If true, automatically sizes the thumb proportional to viewport/content ratio.
    pub auto_size: bool,
    /// If true, hides the thumb when the container is not scrollable.
    pub hide_when_not_scrollable: bool,
    /// Cached base translation set by the downward layout solver.
    pub base_translation: Vec2,
    /// Whether base translation has been initialized.
    pub initialized: bool,
    /// Whether the thumb is currently being dragged by a pointer.
    pub is_dragging: bool,
    /// Pointer position along the movement axis when dragging began.
    pub drag_start_cursor: f32,
    /// Container scroll offset along the movement axis when dragging began.
    pub drag_start_offset: f32,
}

impl UScrollbarThumb {
    /// Creates a vertical scrollbar thumb targeting the given container.
    pub fn vertical(container: Entity) -> Self {
        Self {
            container,
            track: None,
            axis: UScrollbarAxis::Vertical,
            min_size: 20.0,
            auto_size: true,
            hide_when_not_scrollable: false,
            base_translation: Vec2::ZERO,
            initialized: false,
            is_dragging: false,
            drag_start_cursor: 0.0,
            drag_start_offset: 0.0,
        }
    }

    /// Creates a horizontal scrollbar thumb targeting the given container.
    pub fn horizontal(container: Entity) -> Self {
        Self {
            container,
            track: None,
            axis: UScrollbarAxis::Horizontal,
            min_size: 20.0,
            auto_size: true,
            hide_when_not_scrollable: false,
            base_translation: Vec2::ZERO,
            initialized: false,
            is_dragging: false,
            drag_start_cursor: 0.0,
            drag_start_offset: 0.0,
        }
    }

    /// Configures minimum thumb size along its axis.
    pub fn with_min_size(mut self, min_size: f32) -> Self {
        self.min_size = min_size.max(4.0);
        self
    }

    /// Explicitly sets the track entity.
    pub fn with_track(mut self, track: Entity) -> Self {
        self.track = Some(track);
        self
    }

    /// Enables or disables automatic proportional sizing.
    pub fn with_auto_size(mut self, auto_size: bool) -> Self {
        self.auto_size = auto_size;
        self
    }

    /// Enables or disables auto-hiding when not scrollable.
    pub fn with_hide_when_not_scrollable(mut self, hide: bool) -> Self {
        self.hide_when_not_scrollable = hide;
        self
    }
}

/// Adds automatic fade-out behavior to a scrollbar track or thumb when idle.
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct UScrollbarFade {
    /// Idle duration in seconds before fade-out begins (default: 1.2s).
    pub idle_timeout: f32,
    /// Duration of the fade-out animation in seconds (default: 0.35s).
    pub fade_duration: f32,
    /// Idle timer counting up towards `idle_timeout`.
    pub idle_timer: f32,
    /// Current opacity multiplier (0.0 = transparent, 1.0 = fully visible).
    pub current_alpha: f32,
    /// Target opacity multiplier.
    pub target_alpha: f32,
    /// Cached base background alpha for restoration.
    pub base_alpha: f32,
    /// Whether base alpha was initialized.
    pub initialized: bool,
}

impl Default for UScrollbarFade {
    fn default() -> Self {
        Self {
            idle_timeout: 1.2,
            fade_duration: 0.35,
            idle_timer: 0.0,
            current_alpha: 1.0,
            target_alpha: 1.0,
            base_alpha: 1.0,
            initialized: false,
        }
    }
}

impl UScrollbarFade {
    /// Creates a new fade config with the specified idle timeout and fade duration.
    pub fn new(idle_timeout: f32, fade_duration: f32) -> Self {
        Self {
            idle_timeout,
            fade_duration,
            ..Default::default()
        }
    }
}

/// Calculates proportional thumb size based on track size and container viewport/content ratio.
pub fn calculate_thumb_size(
    track_size: f32,
    viewport_size: f32,
    max_offset: f32,
    min_size: f32,
) -> f32 {
    if track_size <= 0.0 || viewport_size <= 0.0 {
        return min_size;
    }
    let content_size = viewport_size + max_offset;
    if content_size <= viewport_size || max_offset <= 0.0 {
        return track_size;
    }
    let ratio = (viewport_size / content_size).clamp(0.0, 1.0);
    (track_size * ratio).clamp(min_size, track_size)
}

/// Synchronizes scrollbar thumb sizes, positions, and visibility.
#[allow(clippy::type_complexity)]
pub fn sync_scrollbar_thumbs(
    containers: Query<(&UScrollContainer, &ComputedSize)>,
    tracks: Query<(&UScrollbarTrack, &ComputedSize)>,
    all_computed: Query<&ComputedSize>,
    parents: Query<&ChildOf>,
    mut thumbs: Query<(
        Entity,
        &mut UScrollbarThumb,
        &mut UNode,
        &ComputedSize,
        &mut Transform,
        Option<&mut Visibility>,
    )>,
) {
    for (_entity, thumb, mut node, thumb_computed, mut transform, mut vis_opt) in thumbs.iter_mut()
    {
        let Ok((container, container_computed)) = containers.get(thumb.container) else {
            continue;
        };

        let track_entity = thumb
            .track
            .or_else(|| parents.get(_entity).ok().map(|p| p.parent()));

        let track_size = track_entity
            .and_then(|te| all_computed.get(te).ok())
            .map(|c| match thumb.axis {
                UScrollbarAxis::Vertical => c.height,
                UScrollbarAxis::Horizontal => c.width,
            })
            .unwrap_or(0.0);

        let (viewport_size, max_offset, progress) = match thumb.axis {
            UScrollbarAxis::Vertical => (
                container_computed.height,
                container.max_offset.y,
                container.progress_y(),
            ),
            UScrollbarAxis::Horizontal => (
                container_computed.width,
                container.max_offset.x,
                container.progress_x(),
            ),
        };

        let is_scrollable = max_offset > 0.0;

        // Auto hide when not scrollable
        if thumb.hide_when_not_scrollable {
            let target_vis = if is_scrollable {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if let Some(vis) = vis_opt.as_deref_mut().filter(|v| **v != target_vis) {
                *vis = target_vis;
            }
        }

        if !is_scrollable {
            continue;
        }

        // Proportional sizing
        let target_thumb_size =
            calculate_thumb_size(track_size, viewport_size, max_offset, thumb.min_size);

        if thumb.auto_size && track_size > 0.0 {
            match thumb.axis {
                UScrollbarAxis::Vertical => {
                    let current_px = match node.height {
                        crate::layout::geometry::UVal::Px(px) => px,
                        _ => -1.0,
                    };
                    if (current_px - target_thumb_size).abs() > 0.5 {
                        node.height = crate::layout::geometry::UVal::Px(target_thumb_size);
                    }
                }
                UScrollbarAxis::Horizontal => {
                    let current_px = match node.width {
                        crate::layout::geometry::UVal::Px(px) => px,
                        _ => -1.0,
                    };
                    if (current_px - target_thumb_size).abs() > 0.5 {
                        node.width = crate::layout::geometry::UVal::Px(target_thumb_size);
                    }
                }
            }
        }

        // Travel range and position
        let actual_thumb_size = match thumb.axis {
            UScrollbarAxis::Vertical => {
                if thumb_computed.height > 0.0 {
                    thumb_computed.height
                } else {
                    target_thumb_size
                }
            }
            UScrollbarAxis::Horizontal => {
                if thumb_computed.width > 0.0 {
                    thumb_computed.width
                } else {
                    target_thumb_size
                }
            }
        };

        let travel = (track_size - actual_thumb_size).max(0.0);

        match thumb.axis {
            UScrollbarAxis::Vertical => {
                // Moving down decreases local Y
                transform.translation.y = thumb.base_translation.y - (progress * travel);
            }
            UScrollbarAxis::Horizontal => {
                // Moving right increases local X
                transform.translation.x = thumb.base_translation.x + (progress * travel);
            }
        }
    }

    // Also update track visibility if hide_when_not_scrollable
    for (track, computed) in tracks.iter() {
        let _ = (track, computed);
    }
}

/// Reads mouse clicks and drag events to control scrollbar thumbs and tracks.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_scrollbar_drag(
    mouse_button: Option<Res<ButtonInput<MouseButton>>>,
    pointer: ScrollPointerContext,
    mut containers: Query<&mut UScrollContainer>,
    tracks: Query<(
        Entity,
        &UScrollbarTrack,
        &GlobalTransform,
        &ComputedSize,
        &UNode,
        Option<&UiPickingContext>,
    )>,
    all_nodes: Query<(&GlobalTransform, &ComputedSize, Option<&UiPickingContext>)>,
    mut thumbs: Query<(
        Entity,
        &mut UScrollbarThumb,
        &GlobalTransform,
        &ComputedSize,
        &UNode,
        Option<&UiPickingContext>,
    )>,
) {
    let Some(mouse_button) = mouse_button else {
        return;
    };

    let just_pressed = mouse_button.just_pressed(MouseButton::Left);
    let is_pressed = mouse_button.pressed(MouseButton::Left);
    let just_released = mouse_button.just_released(MouseButton::Left);

    // 1. Drag release
    if just_released || !is_pressed {
        for (_e, mut thumb, _gt, _computed, _node, _picking) in thumbs.iter_mut() {
            if thumb.is_dragging {
                thumb.is_dragging = false;
            }
        }
    }

    let active_windows = pointer.active_cursor_windows();
    if active_windows.is_empty() {
        return;
    }

    let resolve_anchor = |thumb_entity: Entity, thumb: &UScrollbarThumb| {
        let track_entity = thumb
            .track
            .or_else(|| pointer.parents_query.get(thumb_entity).ok().map(|p| p.parent()));
        let anchor = track_entity.and_then(|te| {
            all_nodes
                .get(te)
                .ok()
                .map(|(gt, computed, picking)| (te, gt, computed, picking))
        });
        (track_entity, anchor)
    };

    // 2. Click detection on topmost visible, unclipped, unoccluded thumbs and tracks
    let mut started_thumb_entity: Option<Entity> = None;

    if just_pressed {
        let thumb_winner = pointer.pick_unoccluded_hit(
            thumbs
                .iter()
                .filter(|(_, thumb, ..)| containers.get(thumb.container).is_ok())
                .map(|(entity, _, gt, computed, node, picking_ctx)| {
                    (entity, node, gt, computed, picking_ctx)
                }),
            &active_windows,
        );

        if let Some(hit) = thumb_winner {
            if let Ok((_, mut thumb, _gt, _computed, _node, _picking)) = thumbs.get_mut(hit.entity)
                && let Ok(container) = containers.get(thumb.container)
            {
                // Anchor drag deltas to the stationary track plane when available so thumb
                // repositioning during sync does not fight cursor deltas.
                let (_, anchor) = resolve_anchor(hit.entity, &thumb);
                let start_local = anchor
                    .and_then(|(anchor_entity, anchor_gt, _computed, anchor_picking)| {
                        pointer.project_cursor_to_node_logical(
                            anchor_entity,
                            anchor_gt,
                            anchor_picking,
                            Some(hit.camera_entity),
                            &active_windows,
                        )
                    })
                    .unwrap_or(hit.local_hit_logical);

                thumb.is_dragging = true;
                thumb.drag_start_cursor = match thumb.axis {
                    UScrollbarAxis::Vertical => start_local.y,
                    UScrollbarAxis::Horizontal => start_local.x,
                };
                thumb.drag_start_offset = match thumb.axis {
                    UScrollbarAxis::Vertical => container.scroll_offset.y,
                    UScrollbarAxis::Horizontal => container.scroll_offset.x,
                };
                started_thumb_entity = Some(hit.entity);
            }
        } else if let Some(hit) = pointer.pick_unoccluded_hit(
            tracks
                .iter()
                .filter(|(_, track, ..)| {
                    track.click_to_jump && containers.get(track.container).is_ok()
                })
                .map(|(entity, _, gt, computed, node, picking_ctx)| {
                    (entity, node, gt, computed, picking_ctx)
                }),
            &active_windows,
        ) && let Ok((_, track, _gt, computed, _node, _picking)) = tracks.get(hit.entity)
            && let Ok(mut container) = containers.get_mut(track.container)
        {
            match track.axis {
                UScrollbarAxis::Vertical => {
                    if computed.height > 0.0 && container.max_offset.y > 0.0 {
                        let half_h = computed.height * 0.5;
                        let click_from_top =
                            (half_h - hit.local_hit_logical.y).clamp(0.0, computed.height);
                        let fraction = click_from_top / computed.height;
                        let target_y = fraction * container.max_offset.y;
                        let cur_x = container.scroll_offset.x;
                        container.jump_to(Vec2::new(cur_x, target_y));
                    }
                }
                UScrollbarAxis::Horizontal => {
                    if computed.width > 0.0 && container.max_offset.x > 0.0 {
                        let half_w = computed.width * 0.5;
                        let click_from_left =
                            (hit.local_hit_logical.x + half_w).clamp(0.0, computed.width);
                        let fraction = click_from_left / computed.width;
                        let target_x = fraction * container.max_offset.x;
                        let cur_y = container.scroll_offset.y;
                        container.jump_to(Vec2::new(target_x, cur_y));
                    }
                }
            }
        }
    }

    // 3. Active drag handling
    if is_pressed {
        for (thumb_entity, thumb, gt, thumb_computed, _node, picking_ctx) in thumbs.iter() {
            if !thumb.is_dragging || started_thumb_entity == Some(thumb_entity) {
                continue;
            }

            let Ok(mut container) = containers.get_mut(thumb.container) else {
                continue;
            };

            let (track_entity, anchor) = resolve_anchor(thumb_entity, &thumb);
            let track_size = track_entity
                .and_then(|te| all_nodes.get(te).ok())
                .map(|(_, c, _)| match thumb.axis {
                    UScrollbarAxis::Vertical => c.height,
                    UScrollbarAxis::Horizontal => c.width,
                })
                .unwrap_or(0.0);

            let projected = if let Some((anchor_entity, anchor_gt, _computed, anchor_picking)) = anchor
            {
                pointer.project_cursor_to_node_logical(
                    anchor_entity,
                    anchor_gt,
                    anchor_picking,
                    None,
                    &active_windows,
                )
            } else {
                pointer.project_cursor_to_node_logical(
                    thumb_entity,
                    gt,
                    picking_ctx,
                    None,
                    &active_windows,
                )
            };

            let Some(local_cursor) = projected else {
                continue;
            };

            match thumb.axis {
                UScrollbarAxis::Vertical => {
                    let travel = (track_size - thumb_computed.height).max(1.0);
                    // Moving cursor down (local.y < drag_start_cursor) increases scroll offset
                    let cursor_delta = local_cursor.y - thumb.drag_start_cursor;
                    let scroll_delta = -cursor_delta * (container.max_offset.y / travel);
                    let new_offset = thumb.drag_start_offset + scroll_delta;
                    let cur_x = container.scroll_offset.x;
                    container.jump_to(Vec2::new(cur_x, new_offset));
                }
                UScrollbarAxis::Horizontal => {
                    let travel = (track_size - thumb_computed.width).max(1.0);
                    // Moving cursor right increases scroll offset
                    let cursor_delta = local_cursor.x - thumb.drag_start_cursor;
                    let scroll_delta = cursor_delta * (container.max_offset.x / travel);
                    let new_offset = thumb.drag_start_offset + scroll_delta;
                    let cur_y = container.scroll_offset.y;
                    container.jump_to(Vec2::new(new_offset, cur_y));
                }
            }
        }
    }
}

/// Fades out scrollbar elements when idle, and fades them in on scroll or hover.
pub fn handle_scrollbar_fade(
    time: Option<Res<Time>>,
    containers: Query<(Entity, &UScrollContainer)>,
    thumbs: Query<&UScrollbarThumb>,
    mut fade_nodes: Query<(&mut UScrollbarFade, &mut UNode)>,
) {
    let dt = time.map(|t| t.delta_secs().min(0.1)).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    // Check if any thumb is currently dragging
    let any_dragging = thumbs.iter().any(|t| t.is_dragging);

    // Check if any container is hovered or active
    let any_container_active = containers.iter().any(|(_e, c)| c.is_hovered);

    for (mut fade, mut node) in fade_nodes.iter_mut() {
        if !fade.initialized {
            fade.base_alpha = node.background_color.alpha();
            fade.initialized = true;
        }

        if any_dragging || any_container_active {
            fade.idle_timer = 0.0;
            fade.target_alpha = 1.0;
        } else {
            fade.idle_timer += dt;
            if fade.idle_timer >= fade.idle_timeout {
                fade.target_alpha = 0.0;
            }
        }

        // Interpolate current alpha
        if (fade.current_alpha - fade.target_alpha).abs() > 0.005 {
            let blend = (dt / fade.fade_duration.max(0.01)).min(1.0);
            fade.current_alpha += (fade.target_alpha - fade.current_alpha) * blend;
            let effective_alpha = fade.base_alpha * fade.current_alpha;
            node.background_color = node.background_color.with_alpha(effective_alpha);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_thumb_size() {
        // Track 500, Viewport 500, Max offset 1000 -> Content 1500 -> Ratio 1/3 -> 500 * (1/3) = 166.67
        let size = calculate_thumb_size(500.0, 500.0, 1000.0, 20.0);
        assert!((size - 166.67).abs() < 1.0);

        // Huge content clamped to min_size
        let small_size = calculate_thumb_size(500.0, 500.0, 50000.0, 24.0);
        assert_eq!(small_size, 24.0);

        // Content fits in viewport -> fills track
        let full_size = calculate_thumb_size(500.0, 500.0, 0.0, 20.0);
        assert_eq!(full_size, 500.0);
    }

    #[test]
    fn test_scrollbar_track_constructors() {
        let e = Entity::from_raw_u32(42).unwrap();
        let track = UScrollbarTrack::vertical(e).with_click_to_jump(false);
        assert_eq!(track.axis, UScrollbarAxis::Vertical);
        assert!(!track.click_to_jump);

        let h_track = UScrollbarTrack::horizontal(e).with_hide_when_not_scrollable(true);
        assert_eq!(h_track.axis, UScrollbarAxis::Horizontal);
        assert!(h_track.hide_when_not_scrollable);
    }

    #[test]
    fn test_scrollbar_thumb_constructors() {
        let e = Entity::from_raw_u32(42).unwrap();
        let thumb = UScrollbarThumb::vertical(e).with_min_size(32.0);
        assert_eq!(thumb.axis, UScrollbarAxis::Vertical);
        assert_eq!(thumb.min_size, 32.0);
        assert!(thumb.auto_size);
        assert!(!thumb.is_dragging);
    }

    #[test]
    fn test_drag_delta_conversion_math() {
        let track_size = 500.0;
        let thumb_size = 100.0;
        let max_offset = 1200.0;
        let travel = track_size - thumb_size; // 400.0

        // If cursor moves down by 100px:
        let cursor_delta = -100.0;
        let scroll_delta = -cursor_delta * (max_offset / travel); // 100 * (1200 / 400) = 300.0
        assert_eq!(scroll_delta, 300.0);

        // Full travel (400px down) scrolls to max_offset (1200px)
        let full_cursor_delta = -400.0;
        let full_scroll_delta = -full_cursor_delta * (max_offset / travel);
        assert_eq!(full_scroll_delta, 1200.0);
    }

    #[test]
    fn test_fade_interpolation() {
        let mut fade = UScrollbarFade::new(1.0, 0.5);
        fade.current_alpha = 1.0;
        fade.target_alpha = 0.0;

        let dt = 0.25;
        let blend = (dt / fade.fade_duration).min(1.0);
        fade.current_alpha += (fade.target_alpha - fade.current_alpha) * blend;
        assert_eq!(fade.current_alpha, 0.5);
    }

    fn spawn_scrollbar_test_camera(
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
    fn test_scrollbar_drag_respects_visibility_clip_occlusion_and_overlap() {
        use crate::layout::univis_node::UClip;
        use bevy::window::PrimaryWindow;

        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_systems(Update, handle_scrollbar_drag);

        let mut win = Window::default();
        win.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        let win_e = app.world_mut().spawn((win, PrimaryWindow)).id();
        let cam_2d =
            spawn_scrollbar_test_camera(&mut app, win_e, false, Vec3::new(0.0, 0.0, 100.0));

        let target_container = app
            .world_mut()
            .spawn(UScrollContainer {
                max_offset: Vec2::new(0.0, 600.0),
                ..UScrollContainer::vertical()
            })
            .id();

        // Hidden track & thumb
        let hidden_parent = app.world_mut().spawn(Visibility::Hidden).id();
        let hidden_track = app
            .world_mut()
            .spawn((
                UScrollbarTrack::vertical(target_container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 200.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 9.0,
                    ..default()
                },
                ChildOf(hidden_parent),
            ))
            .id();
        let hidden_thumb = app
            .world_mut()
            .spawn((
                UScrollbarThumb::vertical(target_container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 50.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 10.0,
                    ..default()
                },
                ChildOf(hidden_track),
            ))
            .id();

        // Clipped track outside UClip rect
        let clip_parent = app
            .world_mut()
            .spawn((
                UNode::default(),
                UClip::enabled(true),
                ComputedSize {
                    width: 40.0,
                    height: 40.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(150.0, 0.0, 0.0)),
            ))
            .id();
        let clipped_track = app
            .world_mut()
            .spawn((
                UScrollbarTrack::vertical(target_container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 200.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    clip_ancestor: Some(clip_parent),
                    local_depth_key: 8.0,
                    ..default()
                },
                ChildOf(clip_parent),
            ))
            .id();

        // Two overlapping tracks + thumbs, blocked initially by an opaque modal
        let lower_track = app
            .world_mut()
            .spawn((
                UScrollbarTrack::vertical(target_container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 200.0,
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
        let lower_thumb = app
            .world_mut()
            .spawn((
                UScrollbarThumb::vertical(target_container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 50.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 2.0,
                    ..default()
                },
                ChildOf(lower_track),
            ))
            .id();

        let upper_track = app
            .world_mut()
            .spawn((
                UScrollbarTrack::vertical(target_container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 200.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 3.0,
                    ..default()
                },
            ))
            .id();
        let upper_thumb = app
            .world_mut()
            .spawn((
                UScrollbarThumb::vertical(target_container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 50.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 4.0,
                    ..default()
                },
                ChildOf(upper_track),
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
                    width: 100.0,
                    height: 100.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_2d),
                    local_depth_key: 20.0,
                    ..default()
                },
            ))
            .id();

        // While occluded: neither thumbs nor tracks activate
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        assert!(!app.world().get::<UScrollbarThumb>(hidden_thumb).unwrap().is_dragging);
        assert!(!app.world().get::<UScrollbarThumb>(lower_thumb).unwrap().is_dragging);
        assert!(!app.world().get::<UScrollbarThumb>(upper_thumb).unwrap().is_dragging);
        assert_eq!(
            app.world()
                .get::<UScrollContainer>(target_container)
                .unwrap()
                .target_offset
                .y,
            0.0
        );

        // Hide blocker and press again: only upper thumb initiates drag; track does not jump
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

        assert!(app.world().get::<UScrollbarThumb>(upper_thumb).unwrap().is_dragging);
        assert!(!app.world().get::<UScrollbarThumb>(lower_thumb).unwrap().is_dragging);
        assert!(!app.world().get::<UScrollbarThumb>(hidden_thumb).unwrap().is_dragging);
        assert_eq!(
            app.world()
                .get::<UScrollContainer>(target_container)
                .unwrap()
                .target_offset
                .y,
            0.0
        );

        // Despawn upper_thumb + lower_thumb while keeping upper_track/clipped_track:
        // Wait until release, then click track bottom-ish? Click center -> jump to 0.5 * 600 = 300.0
        let _ = clipped_track;
        app.world_mut().entity_mut(upper_thumb).despawn();
        app.world_mut().entity_mut(lower_thumb).despawn();
        {
            let mut mouse = app
                .world_mut()
                .resource_mut::<ButtonInput<MouseButton>>();
            mouse.release(MouseButton::Left);
            mouse.clear();
        }
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        assert!(
            (app.world()
                .get::<UScrollContainer>(target_container)
                .unwrap()
                .target_offset
                .y
                - 300.0)
                .abs()
                < 1e-3,
            "Unoccluded track click-to-jump should jump to 50%"
        );
    }

    #[test]
    fn test_scrollbar_drag_and_jump_across_secondary_window_3d_perspective_and_thumb_sync() {
        use crate::layout::layout_system::UiSpace;
        use bevy::window::PrimaryWindow;

        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_systems(Update, handle_scrollbar_drag);

        // Primary window has no cursor; secondary window owns the 3D camera & cursor.
        let win_a = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
        let mut win_b = Window::default();
        win_b.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        let win_b_e = app.world_mut().spawn(win_b).id();

        let _cam_a = spawn_scrollbar_test_camera(&mut app, win_a, false, Vec3::new(0.0, 0.0, 100.0));
        let cam_b_3d =
            spawn_scrollbar_test_camera(&mut app, win_b_e, true, Vec3::new(0.0, 0.0, 2.0));

        let container = app
            .world_mut()
            .spawn(UScrollContainer {
                max_offset: Vec2::new(0.0, 600.0),
                ..UScrollContainer::vertical()
            })
            .id();

        // World3d scale 0.01: track 20x200 logical = 0.2x2.0 world units at z=0.
        let track = app
            .world_mut()
            .spawn((
                UScrollbarTrack::vertical(container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 200.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::IDENTITY,
                UiPickingContext {
                    camera_entity: Some(cam_b_3d),
                    space: UiSpace::World3d,
                    world_scale: 0.01,
                    local_depth_key: 1.0,
                    ..default()
                },
            ))
            .id();

        // Thumb 20x50 logical placed at top of track (+0.75 world units = screen y 187.5).
        let thumb = app
            .world_mut()
            .spawn((
                UScrollbarThumb::vertical(container),
                UNode::default(),
                ComputedSize {
                    width: 20.0,
                    height: 50.0,
                    local_pos: Vec2::ZERO,
                },
                GlobalTransform::from(Transform::from_xyz(0.0, 0.75, 0.0)),
                UiPickingContext {
                    camera_entity: Some(cam_b_3d),
                    space: UiSpace::World3d,
                    world_scale: 0.01,
                    local_depth_key: 2.0,
                    ..default()
                },
                ChildOf(track),
            ))
            .id();

        // 1. Track click-to-jump at midpoint (400, 300) misses thumb [150..225], hits track center.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        assert!(!app.world().get::<UScrollbarThumb>(thumb).unwrap().is_dragging);
        assert!(
            (app.world()
                .get::<UScrollContainer>(container)
                .unwrap()
                .target_offset
                .y
                - 300.0)
                .abs()
                < 1e-3
        );

        // Reset scroll & mouse, then drag thumb across two frames while thumb moves.
        {
            let mut c = app.world_mut().get_mut::<UScrollContainer>(container).unwrap();
            c.scroll_offset = Vec2::ZERO;
            c.target_offset = Vec2::ZERO;
        }
        {
            let mut mouse = app
                .world_mut()
                .resource_mut::<ButtonInput<MouseButton>>();
            mouse.release(MouseButton::Left);
            mouse.clear();
        }
        app.world_mut()
            .get_mut::<Window>(win_b_e)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(400.0, 187.5)));
        app.update();

        // Frame 1: click thumb center at screen y=187.5 (world y=+0.75).
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().get::<UScrollbarThumb>(thumb).unwrap().is_dragging);

        // Frame 2: drag down 150 screen px (y=337.5, -1.0 world = -100 logical px).
        // Travel = 200 - 50 = 150, max_offset = 600 -> offset_y = 100 * 4 = 400.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear_just_pressed(MouseButton::Left);
        app.world_mut()
            .get_mut::<Window>(win_b_e)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(400.0, 337.5)));
        app.update();

        let offset_after_f2 = app
            .world()
            .get::<UScrollContainer>(container)
            .unwrap()
            .target_offset
            .y;
        assert!(
            (offset_after_f2 - 400.0).abs() < 0.5,
            "Expected 400.0 after -100px 3D drag, got {offset_after_f2}"
        );

        // Frame 3: simulate sync_scrollbar_thumbs moving the thumb down, then drag -37.5px more (y=375.0).
        // Total delta = -125 logical px -> offset_y = 125 * 4 = 500 without thumb-feedback drift.
        *app.world_mut().get_mut::<GlobalTransform>(thumb).unwrap() =
            GlobalTransform::from(Transform::from_xyz(0.0, -0.25, 0.0));
        app.world_mut()
            .get_mut::<Window>(win_b_e)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(400.0, 375.0)));
        app.update();

        let offset_after_f3 = app
            .world()
            .get::<UScrollContainer>(container)
            .unwrap()
            .target_offset
            .y;
        assert!(
            (offset_after_f3 - 500.0).abs() < 0.5,
            "Expected 500.0 after synced thumb drag, got {offset_after_f3}"
        );
    }
}
