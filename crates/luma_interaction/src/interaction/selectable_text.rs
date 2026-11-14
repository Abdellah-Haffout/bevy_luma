//! Interactive selection and copy support for [`USelectableText`] labels.

use bevy::camera::RenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::picking::events::Pointer as PickingPointer;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::text::ComputedTextBlock;
use bevy::window::PrimaryWindow;
use luma_engine::layout::{
    geometry::ComputedSize,
    layout_system::ResolvedRootUi,
    query::UiPickingContext,
    render::clip_sync::{
        hit_world_to_local_logical, project_screen_point_to_node, resolve_node_world_scale,
        resolve_primary_window_entity,
    },
    text::{
        UCopyText, USelectableText, UTextLabel, UTextLabelLayoutCache, selectable_effective_text,
        selectable_text_extracted_copy, text_byte_index_from_local_point,
        text_word_range_from_local_point,
    },
    univis_node::UNode,
};

use crate::interaction::focus::UFocusState;
use crate::interaction::text_input::{UTextInput, clipboard::set_clipboard_text};

const DOUBLE_CLICK_MAX_INTERVAL_SECS: f64 = 0.35;

/// Tracks active drag-selection and double-click state across [`USelectableText`] labels.
#[derive(Resource, Default, Debug)]
pub struct USelectableTextState {
    /// Currently active selectable text label entity for keyboard shortcuts (`Ctrl+C`, `Ctrl+A`, `Escape`).
    pub active_entity: Option<Entity>,
    /// Fixed byte offset anchor during pointer drag selection.
    pub drag_anchor: Option<usize>,
    /// Timestamp of the last primary press for double-click word detection.
    pub last_click_secs: f64,
    /// Entity clicked at `last_click_secs`.
    pub last_click_entity: Option<Entity>,
}

fn pointer_location_to_local_logical(
    entity: Entity,
    pointer: &PickingPointer,
    picking_ctx: Option<&UiPickingContext>,
    gt: &GlobalTransform,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
    windows: &Query<(Entity, Option<&PrimaryWindow>)>,
    cameras: &Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
) -> Option<Vec2> {
    let preferred_camera = picking_ctx.and_then(|ctx| ctx.camera_entity);
    let world_scale = resolve_node_world_scale(entity, picking_ctx, parents_query, root_query);
    let primary_window =
        resolve_primary_window_entity(windows.iter().map(|(e, primary)| (e, primary.is_some())));
    project_screen_point_to_node(
        pointer.position,
        Some(&pointer.target),
        None,
        primary_window,
        preferred_camera,
        gt,
        world_scale,
        false,
        cameras,
    )
    .map(|(_, _, _, local_logical, _)| local_logical)
}

/// Observer initiating click, Shift-extend, or double-click word selection on [`USelectableText`].
pub fn on_selectable_text_pointer_press(
    trigger: On<PointerPress>,
    time: Option<Res<Time>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut state: ResMut<USelectableTextState>,
    mut query: Query<(
        Entity,
        &mut USelectableText,
        &UTextLabel,
        &UNode,
        &ComputedSize,
        &UTextLabelLayoutCache,
        &ComputedTextBlock,
        &GlobalTransform,
        Option<&UiPickingContext>,
    )>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    windows: Query<(Entity, Option<&PrimaryWindow>)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }

    let entity = trigger.entity;
    let Ok((
        _,
        selectable_ref,
        label,
        node,
        computed_size,
        layout_cache,
        computed,
        gt,
        picking_ctx,
    )) = query.get(entity)
    else {
        return;
    };

    if !selectable_ref.enabled {
        return;
    }

    let world_scale = resolve_node_world_scale(entity, picking_ctx, &parents_query, &root_query);
    let Some(local_logical) = trigger
        .event()
        .hit
        .position
        .map(|hit_world| hit_world_to_local_logical(hit_world, gt, world_scale))
        .or_else(|| {
            pointer_location_to_local_logical(
                entity,
                &trigger.pointer,
                picking_ctx,
                gt,
                &parents_query,
                &root_query,
                &windows,
                &cameras,
            )
        })
    else {
        return;
    };

    let text_len = selectable_effective_text(label, layout_cache).len();
    let hit_idx = text_byte_index_from_local_point(
        label,
        node,
        computed_size,
        layout_cache,
        computed,
        local_logical,
    )
    .min(text_len);

    let now = time.as_ref().map_or(0.0, |t| t.elapsed_secs_f64());
    let delta = now - state.last_click_secs;
    let is_double_click = state.last_click_entity == Some(entity)
        && delta > 0.0
        && delta < DOUBLE_CLICK_MAX_INTERVAL_SECS;

    let is_shift = keyboard
        .as_ref()
        .is_some_and(|k| k.pressed(KeyCode::ShiftLeft) || k.pressed(KeyCode::ShiftRight));

    let word_range = if is_double_click {
        Some(text_word_range_from_local_point(
            label,
            node,
            computed_size,
            layout_cache,
            computed,
            local_logical,
        ))
    } else {
        None
    };

    let prev_entity = state.active_entity;
    state.last_click_secs = now;
    state.last_click_entity = Some(entity);
    state.active_entity = Some(entity);

    if let Some(prev) = prev_entity
        && prev != entity
        && let Ok((_, mut prev_selectable, ..)) = query.get_mut(prev)
    {
        prev_selectable.clear_selection();
        state.drag_anchor = None;
    }

    let Ok((_, mut selectable, ..)) = query.get_mut(entity) else {
        return;
    };

    if let Some((word_start, word_end)) = word_range {
        if word_start < word_end {
            selectable.selection = Some((word_start, word_end));
        } else {
            selectable.clear_selection();
        }
        state.drag_anchor = None;
    } else if is_shift && prev_entity == Some(entity) {
        let anchor = state
            .drag_anchor
            .or_else(|| selectable.selection.map(|(s, _)| s))
            .unwrap_or(hit_idx)
            .min(text_len);
        if anchor != hit_idx {
            selectable.selection = Some((anchor.min(hit_idx), anchor.max(hit_idx)));
        } else {
            selectable.clear_selection();
        }
        state.drag_anchor = Some(anchor);
    } else {
        selectable.clear_selection();
        state.drag_anchor = Some(hit_idx);
    }
}

/// Observer updating [`USelectableText`] selection while dragging with the primary pointer button.
pub fn on_selectable_text_pointer_drag(
    trigger: On<PointerDrag>,
    state: Res<USelectableTextState>,
    mut query: Query<(
        Entity,
        &mut USelectableText,
        &UTextLabel,
        &UNode,
        &ComputedSize,
        &UTextLabelLayoutCache,
        &ComputedTextBlock,
        &GlobalTransform,
        Option<&UiPickingContext>,
    )>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    windows: Query<(Entity, Option<&PrimaryWindow>)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }

    let entity = trigger.entity;
    if state.active_entity != Some(entity) {
        return;
    }
    let Some(anchor) = state.drag_anchor else {
        return;
    };

    let Ok((
        _,
        mut selectable,
        label,
        node,
        computed_size,
        layout_cache,
        computed,
        gt,
        picking_ctx,
    )) = query.get_mut(entity)
    else {
        return;
    };

    if !selectable.enabled {
        return;
    }

    let Some(local_logical) = pointer_location_to_local_logical(
        entity,
        &trigger.pointer,
        picking_ctx,
        gt,
        &parents_query,
        &root_query,
        &windows,
        &cameras,
    ) else {
        return;
    };

    let text_len = selectable_effective_text(label, layout_cache).len();
    let drag_idx = text_byte_index_from_local_point(
        label,
        node,
        computed_size,
        layout_cache,
        computed,
        local_logical,
    )
    .min(text_len);
    let clamped_anchor = anchor.min(text_len);

    if clamped_anchor != drag_idx {
        selectable.selection = Some((clamped_anchor.min(drag_idx), clamped_anchor.max(drag_idx)));
    } else {
        selectable.clear_selection();
    }
}

/// Handles `Ctrl/Cmd+C`, `Ctrl/Cmd+A`, and `Escape` for the active [`USelectableText`] label.
pub fn selectable_text_keyboard_system(
    mut commands: Commands,
    focus_state: Option<Res<UFocusState>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    keyboard_events: Option<MessageReader<KeyboardInput>>,
    mut state: ResMut<USelectableTextState>,
    text_inputs: Query<(), With<UTextInput>>,
    mut query: Query<(
        Entity,
        &mut USelectableText,
        &UTextLabel,
        &UTextLabelLayoutCache,
    )>,
) {
    let (Some(keyboard), Some(mut keyboard_events)) = (keyboard, keyboard_events) else {
        return;
    };

    let text_input_focused = focus_state
        .as_ref()
        .and_then(|fs| fs.focused)
        .is_some_and(|e| text_inputs.contains(e));
    if text_input_focused {
        return;
    }

    let ctrl = keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight)
        || keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight);

    for ev in keyboard_events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }

        if ctrl && matches!(ev.key_code, KeyCode::KeyC | KeyCode::Insert) {
            let target = state
                .active_entity
                .filter(|e| query.get(*e).is_ok_and(|(_, s, ..)| s.enabled && s.is_selected()))
                .or_else(|| {
                    query
                        .iter()
                        .find(|(_, s, ..)| s.enabled && s.is_selected())
                        .map(|(e, ..)| e)
                });
            if let Some(target_entity) = target
                && let Ok((_, selectable, label, cache)) = query.get(target_entity)
                && let Some(text) = selectable_text_extracted_copy(label, cache, selectable)
            {
                commands.trigger(UCopyText { text });
            }
        } else if ctrl && ev.key_code == KeyCode::KeyA {
            if let Some(active) = state.active_entity
                && let Ok((_, mut selectable, label, cache)) = query.get_mut(active)
                && selectable.enabled
            {
                let len = selectable_effective_text(label, cache).len();
                if len > 0 {
                    selectable.select_all(len);
                } else {
                    selectable.clear_selection();
                }
            }
        } else if ev.key_code == KeyCode::Escape {
            if let Some(active) = state.active_entity
                && let Ok((_, mut selectable, ..)) = query.get_mut(active)
            {
                selectable.clear_selection();
            } else {
                for (_, mut selectable, ..) in query.iter_mut() {
                    if selectable.is_selected() {
                        selectable.clear_selection();
                    }
                }
            }
            state.drag_anchor = None;
        }
    }
}

/// Default global observer writing [`UCopyText`] payloads to the system clipboard.
pub fn on_copy_text_to_clipboard(trigger: On<UCopyText>) {
    let _ = set_clipboard_text(&trigger.event().text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::LumaInteractionPlugin;
    use crate::interaction::focus::{FocusModality, UFocusable, set_focus};
    use crate::interaction::picking::univis_picking_backend;
    use crate::interaction::text_input::UTextInputCaret;
    use bevy::camera::{
        CameraProjection, ComputedCameraValues, NormalizedRenderTarget, RenderTarget,
        RenderTargetInfo,
    };
    use bevy::input::keyboard::Key;
    use bevy::picking::backend::{HitData, PointerHits};
    use bevy::picking::events::{Pointer, PointerDrag, PointerPress};
    use bevy::picking::pointer::{Location, PointerId, PointerLocation, PointerMap};
    use bevy::text::Font;
    use bevy::time::TimeUpdateStrategy;
    use bevy::window::WindowRef;
    use luma_engine::LumaEnginePlugin;
    use luma_engine::layout::geometry::UVal;
    use luma_engine::layout::layout_system::{URootUi, UiCameraRef, UiSpace};
    use luma_engine::layout::text::compute_text_selection_rects;
    use std::time::Duration;

    #[derive(Resource, Default)]
    struct RecordedCopyEvents(Vec<String>);

    fn build_selectable_test_app() -> (App, Entity, Entity, Entity, Handle<Font>) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            LumaEnginePlugin,
            LumaInteractionPlugin,
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<Image>();
        app.init_asset::<TextureAtlasLayout>();
        app.init_asset::<bevy::sprite_render::ColorMaterial>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_message::<KeyboardInput>();
        app.init_resource::<PointerMap>();
        app.add_message::<PointerHits>();
        app.init_resource::<RecordedCopyEvents>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));

        app.add_observer(
            |trigger: On<UCopyText>, mut recorded: ResMut<RecordedCopyEvents>| {
                recorded.0.push(trigger.event().text.clone());
            },
        );

        let primary_win = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let secondary_win = app.world_mut().spawn(Window::default()).id();

        let mut camera = Camera::default();
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(800, 600),
                scale_factor: 1.0,
            }),
            clip_from_view: PerspectiveProjection {
                fov: core::f32::consts::FRAC_PI_2,
                aspect_ratio: 800.0 / 600.0,
                near: 0.1,
                ..default()
            }
            .get_clip_from_view(),
            ..default()
        };
        let sec_camera = app
            .world_mut()
            .spawn((
                camera,
                RenderTarget::Window(WindowRef::Entity(secondary_win)),
                Transform::from_xyz(0.0, 0.0, 5.0),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 5.0)),
            ))
            .id();

        let font = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(
                include_bytes!(
                    "../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf"
                )
                .to_vec(),
            ));

        (app, primary_win, secondary_win, sec_camera, font)
    }

    fn world_to_secondary_viewport(
        app: &App,
        camera_entity: Entity,
        world_pos: Vec3,
    ) -> Vec2 {
        let camera = app.world().get::<Camera>(camera_entity).unwrap();
        let camera_gt = app.world().get::<GlobalTransform>(camera_entity).unwrap();
        camera.world_to_viewport(camera_gt, world_pos).unwrap()
    }

    #[test]
    fn press_and_drag_updates_selection_across_secondary_window_world3d() {
        let (mut app, primary_win, secondary_win, sec_camera, font) = build_selectable_test_app();
        let root = app
            .world_mut()
            .spawn((
                URootUi {
                    camera: UiCameraRef::Entity(sec_camera),
                    ..URootUi::world_3d(Vec2::new(400.0, 200.0))
                },
                UNode {
                    width: UVal::Px(400.0),
                    height: UVal::Px(200.0),
                    ..default()
                },
            ))
            .id();

        let label_a = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UTextLabel {
                    text: "Selectable World Text".into(),
                    font_size: 24.0,
                    font: font.clone(),
                    justify: Justify::Left,
                    ..default()
                },
                USelectableText::default(),
            ))
            .id();

        let label_b = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UTextLabel {
                    text: "Second Block".into(),
                    font_size: 20.0,
                    font,
                    ..default()
                },
                USelectableText::default(),
            ))
            .id();

        for _ in 0..4 {
            app.update();
        }

        let target: NormalizedRenderTarget = RenderTarget::Window(WindowRef::Entity(secondary_win))
            .normalize(Some(primary_win))
            .unwrap();
        let label_gt = *app.world().get::<GlobalTransform>(label_a).unwrap();
        let world_scale = app
            .world()
            .get::<UiPickingContext>(label_a)
            .map_or(URootUi::DEFAULT_METERS_PER_UNIT, |c| c.world_scale);
        let rects = {
            let w = app.world();
            compute_text_selection_rects(
                w.get::<UTextLabel>(label_a).unwrap(),
                w.get::<UNode>(label_a).unwrap(),
                w.get::<ComputedSize>(label_a).unwrap(),
                w.get::<UTextLabelLayoutCache>(label_a).unwrap(),
                w.get::<ComputedTextBlock>(label_a).unwrap(),
                (0, 10),
            )
        };
        assert_eq!(rects.len(), 1);

        let left_world = label_gt
            .to_matrix()
            .transform_point3(Vec3::new((rects[0].center.x - rects[0].size.x * 0.45) * world_scale, rects[0].center.y * world_scale, 0.0));
        let right_world = label_gt
            .to_matrix()
            .transform_point3(Vec3::new((rects[0].center.x + rects[0].size.x * 0.45) * world_scale, rects[0].center.y * world_scale, 0.0));

        let start_screen = world_to_secondary_viewport(&app, sec_camera, left_world);
        let end_screen = world_to_secondary_viewport(&app, sec_camera, right_world);

        app.world_mut().trigger(PointerPress {
            entity: label_a,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: target.clone(),
                    position: start_screen,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(sec_camera, 5.0, Some(left_world), Some(Vec3::Z)),
            count: 1,
        });
        app.world_mut().trigger(PointerDrag {
            entity: label_a,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: target.clone(),
                    position: end_screen,
                },
            ),
            button: PointerButton::Primary,
            distance: end_screen - start_screen,
            delta: end_screen - start_screen,
        });
        app.update();

        let copied = {
            let w = app.world();
            selectable_text_extracted_copy(
                w.get::<UTextLabel>(label_a).unwrap(),
                w.get::<UTextLabelLayoutCache>(label_a).unwrap(),
                w.get::<USelectableText>(label_a).unwrap(),
            )
            .expect("drag should create a non-empty selection")
        };
        assert!(copied.contains("Select") || copied.len() >= 4, "got {copied:?}");

        // Pressing a different selectable label should clear label_a's highlight.
        let b_gt = *app.world().get::<GlobalTransform>(label_b).unwrap();
        let b_screen = world_to_secondary_viewport(&app, sec_camera, b_gt.translation());
        app.world_mut().trigger(PointerPress {
            entity: label_b,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target,
                    position: b_screen,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(sec_camera, 5.0, Some(b_gt.translation()), Some(Vec3::Z)),
            count: 1,
        });
        app.update();
        assert!(!app.world().get::<USelectableText>(label_a).unwrap().is_selected());
    }

    #[test]
    fn double_click_ctrl_c_ctrl_a_escape_and_focus_priority() {
        let (mut app, primary_win, _, sec_camera, font) = build_selectable_test_app();
        // Use a Screen root bound to the secondary/primary camera to lay out the label.
        let root = app
            .world_mut()
            .spawn((
                URootUi {
                    camera: UiCameraRef::Entity(sec_camera),
                    ..URootUi::world_2d(Vec2::new(400.0, 200.0))
                },
                UNode {
                    width: UVal::Px(400.0),
                    height: UVal::Px(200.0),
                    ..default()
                },
            ))
            .id();

        let text = "alpha beta gamma";
        let label_entity = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UTextLabel {
                    text: text.into(),
                    font_size: 24.0,
                    font,
                    justify: Justify::Left,
                    ..default()
                },
                USelectableText::default(),
            ))
            .id();

        let input_entity = app
            .world_mut()
            .spawn((
                UTextInput::new("type"),
                UTextInputCaret::default(),
                UFocusable::default(),
            ))
            .id();

        for _ in 0..4 {
            app.update();
        }

        // Compute world position right at the center of word "beta" (bytes 6..10).
        let gt = *app.world().get::<GlobalTransform>(label_entity).unwrap();
        let beta_local = {
            let w = app.world();
            let rects = compute_text_selection_rects(
                w.get::<UTextLabel>(label_entity).unwrap(),
                w.get::<UNode>(label_entity).unwrap(),
                w.get::<ComputedSize>(label_entity).unwrap(),
                w.get::<UTextLabelLayoutCache>(label_entity).unwrap(),
                w.get::<ComputedTextBlock>(label_entity).unwrap(),
                (6, 10),
            );
            rects[0].center
        };
        let world_scale = app
            .world()
            .get::<UiPickingContext>(label_entity)
            .map_or(URootUi::DEFAULT_METERS_PER_UNIT, |c| c.world_scale);
        let beta_world = gt
            .to_matrix()
            .transform_point3((beta_local * world_scale).extend(0.0));
        let target: NormalizedRenderTarget = RenderTarget::Window(WindowRef::Primary)
            .normalize(Some(primary_win))
            .unwrap();
        let send_press = |app: &mut App| {
            app.world_mut().trigger(PointerPress {
                entity: label_entity,
                pointer: Pointer::new(
                    PointerId::Mouse,
                    Location {
                        target: target.clone(),
                        position: Vec2::ZERO,
                    },
                ),
                button: PointerButton::Primary,
                hit: HitData::new(sec_camera, 5.0, Some(beta_world), Some(Vec3::Z)),
                count: 1,
            });
        };

        // Two primary clicks separated by 100ms trigger double-click word selection.
        send_press(&mut app);
        app.update();
        send_press(&mut app);
        app.update();

        let word = {
            let w = app.world();
            selectable_text_extracted_copy(
                w.get::<UTextLabel>(label_entity).unwrap(),
                w.get::<UTextLabelLayoutCache>(label_entity).unwrap(),
                w.get::<USelectableText>(label_entity).unwrap(),
            )
        };
        assert_eq!(word.as_deref(), Some("beta"));

        // Ctrl+C triggers UCopyText { text: "beta" }.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyC,
            logical_key: Key::Character("c".into()),
            state: ButtonState::Pressed,
            text: Some("c".into()),
            repeat: false,
            window: primary_win,
        });
        app.update();
        assert_eq!(
            app.world().resource::<RecordedCopyEvents>().0,
            vec!["beta".to_string()]
        );

        // When a UTextInput holds focus, selectable_text_keyboard_system must not intercept Ctrl+C.
        app.world_mut().resource_scope(|world, mut focus_state: Mut<UFocusState>| {
            let mut cmds = world.commands();
            set_focus(&mut cmds, &mut focus_state, Some(input_entity), FocusModality::Pointer);
        });
        app.world_mut().flush();
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyC,
            logical_key: Key::Character("c".into()),
            state: ButtonState::Pressed,
            text: Some("c".into()),
            repeat: false,
            window: primary_win,
        });
        app.update();
        assert_eq!(app.world().resource::<RecordedCopyEvents>().0.len(), 1);

        // Release focus, press Ctrl+A then Escape.
        app.world_mut().resource_scope(|world, mut focus_state: Mut<UFocusState>| {
            let mut cmds = world.commands();
            set_focus(&mut cmds, &mut focus_state, None, FocusModality::Pointer);
        });
        app.world_mut().flush();
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character("a".into()),
            state: ButtonState::Pressed,
            text: Some("a".into()),
            repeat: false,
            window: primary_win,
        });
        app.update();
        assert_eq!(
            app.world().get::<USelectableText>(label_entity).unwrap().selection,
            Some((0, text.len()))
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ControlLeft);
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: primary_win,
        });
        app.update();
        assert!(!app.world().get::<USelectableText>(label_entity).unwrap().is_selected());
    }

    #[test]
    fn picking_backend_hits_u_selectable_text_without_u_interaction() {
        let (mut app, primary_win, secondary_win, sec_camera, font) = build_selectable_test_app();
        app.add_systems(Update, univis_picking_backend);

        let selectable_entity = app
            .world_mut()
            .spawn((
                UTextLabel {
                    text: "Pickable Selectable".into(),
                    font,
                    ..default()
                },
                USelectableText::default(),
                UNode::default(),
                ComputedSize {
                    width: 200.0,
                    height: 60.0,
                    ..default()
                },
                Transform::default(),
                GlobalTransform::default(),
                Visibility::Visible,
                UiPickingContext {
                    root_entity: None,
                    camera_entity: Some(sec_camera),
                    space: UiSpace::World2d,
                    clip_ancestor: None,
                    root_sort_key: 0.0,
                    local_depth_key: 0.0,
                    world_scale: 1.0,
                },
            ))
            .id();

        let target = RenderTarget::Window(WindowRef::Entity(secondary_win))
            .normalize(Some(primary_win))
            .unwrap();
        app.world_mut().spawn((
            PointerId::Mouse,
            PointerLocation::new(Location {
                target,
                position: Vec2::new(400.0, 300.0),
            }),
        ));

        app.update();

        let hits = app.world().resource::<Messages<PointerHits>>();
        let mut cursor = hits.get_cursor();
        let all_picks: Vec<Entity> = cursor
            .read(hits)
            .flat_map(|batch| batch.picks.iter().map(|(e, _)| *e))
            .collect();
        assert!(
            all_picks.contains(&selectable_entity),
            "expected univis_picking_backend to hit USelectableText without UInteraction; hits={all_picks:?}"
        );
    }
}
