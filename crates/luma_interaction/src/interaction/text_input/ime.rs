//! Platform IME handling (`Preedit`, `Commit`, `Enabled`, `Disabled`) and window
//! IME cursor-area synchronization for [`UTextInput`].

use bevy::camera::{NormalizedRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::window::{Ime, PrimaryWindow, Window};
use luma_engine::layout::{
    layout_system::ResolvedRootUi, query::UiPickingContext,
    render::clip_sync::resolve_primary_window_entity,
};

use super::{
    actions::insert_text,
    types::{
        ImePreeditState, TextInputMode, UTextInput, UTextInputCaret, UTextInputCaretMarker,
        UTextInputChanged,
    },
};
use crate::interaction::focus::{UFocusState, UFocusable};

/// Resolves the camera and target window entity associated with a [`UTextInput`].
pub(super) fn resolve_input_camera_and_window(
    entity: Entity,
    picking_ctx: Option<&UiPickingContext>,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
    cameras: &Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
    primary_window: Option<Entity>,
) -> (Option<Entity>, Option<Entity>) {
    let camera_entity = picking_ctx.and_then(|ctx| ctx.camera_entity).or_else(|| {
        let mut cursor = entity;
        loop {
            if let Ok(resolved) = root_query.get(cursor) {
                return resolved.camera_entity;
            }
            let Ok(child_of) = parents_query.get(cursor) else {
                return None;
            };
            cursor = child_of.parent();
        }
    });

    let window_entity = if let Some(cam) = camera_entity {
        cameras
            .get(cam)
            .ok()
            .and_then(|(_, _, _, rt_opt)| match rt_opt {
                None => primary_window,
                Some(rt) => match rt.normalize(primary_window) {
                    Some(NormalizedRenderTarget::Window(win_ref)) => Some(win_ref.entity()),
                    _ => None,
                },
            })
    } else {
        primary_window
    };

    (camera_entity, window_entity)
}

/// Processes Bevy [`Ime`] composition events for the focused [`UTextInput`] and keeps the
/// target window's `ime_enabled` and `ime_position` synchronized with the active caret.
pub fn text_input_ime_system(
    mut commands: Commands,
    focus_state: Option<Res<UFocusState>>,
    ime_events: Option<MessageReader<Ime>>,
    mut inputs: Query<(
        Entity,
        &mut UTextInput,
        &mut UTextInputCaret,
        &UFocusable,
        Option<&UiPickingContext>,
        Option<&GlobalTransform>,
    )>,
    mut windows: Query<(Entity, &mut Window, Option<&PrimaryWindow>)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
    caret_transforms: Query<&GlobalTransform, With<UTextInputCaretMarker>>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
) {
    let primary_window = resolve_primary_window_entity(
        windows
            .iter()
            .map(|(entity, _, primary)| (entity, primary.is_some())),
    );

    let focused_entity = focus_state.as_ref().and_then(|f| f.focused);
    let mut want_ime_window: Option<Entity> = None;
    let mut want_ime_camera: Option<Entity> = None;
    let mut caret_world_pos: Option<Vec3> = None;

    if let Some(entity) = focused_entity
        && let Ok((_, mut input, mut caret, focusable, picking_ctx, input_gt)) =
            inputs.get_mut(entity)
        && focusable.enabled
        && input.enabled
    {
        let (camera_entity, target_window) = resolve_input_camera_and_window(
            entity,
            picking_ctx,
            &parents_query,
            &root_query,
            &cameras,
            primary_window,
        );

        if let Some(mut reader) = ime_events {
            for ev in reader.read() {
                let ev_window = match ev {
                    Ime::Preedit { window, .. }
                    | Ime::Commit { window, .. }
                    | Ime::Enabled { window }
                    | Ime::Disabled { window } => *window,
                };
                if target_window.is_some_and(|w| w != ev_window) {
                    continue;
                }

                match ev {
                    Ime::Enabled { .. } => {
                        caret.ime_preedit = None;
                        caret.reset_blink();
                    }
                    Ime::Disabled { .. } => {
                        caret.ime_preedit = None;
                    }
                    Ime::Preedit { value, cursor, .. } => {
                        if input.mode == TextInputMode::Normal && !value.is_empty() {
                            caret.ime_preedit = Some(ImePreeditState {
                                value: value.clone(),
                                cursor: *cursor,
                            });
                        } else {
                            caret.ime_preedit = None;
                        }
                        caret.reset_blink();
                    }
                    Ime::Commit { value, .. } => {
                        caret.ime_preedit = None;
                        if insert_text(&mut input, value) {
                            commands.trigger(UTextInputChanged {
                                entity,
                                value: input.value.clone(),
                            });
                        }
                        caret.reset_blink();
                    }
                }
            }
        }

        if input.mode != TextInputMode::Password {
            want_ime_window = target_window;
            want_ime_camera = camera_entity;
            caret_world_pos = caret
                .caret_entity
                .and_then(|ce| caret_transforms.get(ce).ok())
                .map(|gt| gt.translation())
                .or_else(|| input_gt.map(|gt| gt.translation()));
        }
    } else if let Some(mut reader) = ime_events {
        // Drain stale IME messages when no input holds focus so they aren't replayed on later focus.
        for _ in reader.read() {}
    }

    let ime_screen_pos = want_ime_camera
        .and_then(|cam_entity| cameras.get(cam_entity).ok())
        .zip(caret_world_pos)
        .and_then(|((_, camera, cam_gt, _), world_pos)| {
            camera.world_to_viewport(cam_gt, world_pos).ok()
        });

    for (win_entity, mut window, _) in windows.iter_mut() {
        let should_enable = want_ime_window == Some(win_entity);
        if window.ime_enabled != should_enable {
            window.ime_enabled = should_enable;
        }
        if should_enable
            && let Some(pos) = ime_screen_pos
            && window.ime_position.distance(pos) > 0.5
        {
            window.ime_position = pos;
        }
    }
}
