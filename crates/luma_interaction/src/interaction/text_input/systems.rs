//! ECS systems for keyboard input, caret animation, and visual synchronization.

use bevy::camera::RenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::text::{ComputedTextBlock, LineBreak};
use bevy::window::PrimaryWindow;
use luma_engine::layout::{
    geometry::{ComputedSize, UCornerRadius, UVal},
    layout_system::ResolvedRootUi,
    query::UiPickingContext,
    render::clip_sync::{
        hit_world_to_local_logical, project_screen_point_to_node, resolve_node_world_scale,
        resolve_primary_window_entity,
    },
    text::{
        UTextLabel, UTextLabelLayoutCache, UTextOverflow, UTextSelectionRect, UTextWrapMode,
        compute_text_selection_rects, text_byte_index_from_local_point,
        text_caret_rect_from_byte_index, text_move_caret_line_edge, text_move_caret_vertical,
        text_word_range_from_local_point,
    },
    univis_node::{UClip, UNode, UPositionType, USelf, UZIndex},
};

use super::{
    actions::{
        char_offset_to_x, clear_selection, cursor_index_from_x_offset, delete_backward,
        delete_forward, delete_selection, delete_word_backward, delete_word_forward,
        display_byte_to_value_byte, display_text, insert_text, move_cursor_down, move_cursor_left,
        move_cursor_right, move_cursor_to_byte, move_cursor_to_end, move_cursor_to_start,
        move_cursor_up, move_word_left, move_word_right, select_all, select_word_at_cursor,
        selected_text, selection_bounds, value_byte_to_display_byte,
    },
    clipboard::{get_clipboard_text, set_clipboard_text},
    types::{
        TextInputMode, UTextInput, UTextInputCancelled, UTextInputCaret, UTextInputCaretMarker,
        UTextInputChanged, UTextInputSelectionMarker, UTextInputSubmit, UTextInputTextMarker,
        UTextInputViewportMarker,
    },
};
use crate::interaction::focus::{FocusLost, FocusModality, UFocusState, UFocusable, set_focus};

/// Keyboard processing system driving active text input fields.
pub fn text_input_keyboard_system(
    mut commands: Commands,
    focus_state: Option<Res<UFocusState>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    keyboard_events: Option<MessageReader<KeyboardInput>>,
    mut query: Query<(Entity, &mut UTextInput, &mut UTextInputCaret, &UFocusable)>,
    text_query: Query<(&UTextLabel, &UTextLabelLayoutCache, &ComputedTextBlock)>,
) {
    let (Some(focus_state), Some(keyboard), Some(mut keyboard_events)) =
        (focus_state, keyboard, keyboard_events)
    else {
        return;
    };
    let Some(focused_entity) = focus_state.focused else {
        return;
    };

    let Ok((entity, mut input, mut caret, focusable)) = query.get_mut(focused_entity) else {
        return;
    };

    if !focusable.enabled || !input.enabled {
        return;
    }

    let ctrl = keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight)
        || keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight);

    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    let parley_ready = caret
        .text_entity
        .and_then(|te| text_query.get(te).ok())
        .filter(|(_, cache, computed)| {
            !computed.buffer().is_empty() && cache.displayed_text == display_text(&input)
        });

    for ev in keyboard_events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }

        if ctrl {
            let changed = |commands: &mut Commands, input: &UTextInput| {
                commands.trigger(UTextInputChanged {
                    entity,
                    value: input.value.clone(),
                });
            };
            match ev.key_code {
                KeyCode::KeyA => select_all(&mut input),
                KeyCode::KeyC | KeyCode::Insert => {
                    if input.mode != TextInputMode::Password
                        && let Some(text) = selected_text(&input)
                    {
                        set_clipboard_text(&text);
                    }
                }
                KeyCode::KeyX => {
                    if input.mode != TextInputMode::Password
                        && let Some(text) = selected_text(&input)
                        && set_clipboard_text(&text)
                        && delete_selection(&mut input)
                    {
                        caret.ime_preedit = None;
                        changed(&mut commands, &input);
                    }
                }
                KeyCode::KeyV => {
                    if let Some(text) = get_clipboard_text()
                        && insert_text(&mut input, &text)
                    {
                        caret.ime_preedit = None;
                        changed(&mut commands, &input);
                    }
                }
                KeyCode::Backspace => {
                    if delete_word_backward(&mut input) {
                        caret.ime_preedit = None;
                        changed(&mut commands, &input);
                    }
                }
                KeyCode::Delete => {
                    if delete_word_forward(&mut input) {
                        caret.ime_preedit = None;
                        changed(&mut commands, &input);
                    }
                }
                KeyCode::ArrowLeft => move_word_left(&mut input, shift),
                KeyCode::ArrowRight => move_word_right(&mut input, shift),
                KeyCode::Home => move_cursor_to_start(&mut input, shift),
                KeyCode::End => move_cursor_to_end(&mut input, shift),
                KeyCode::Enter | KeyCode::NumpadEnter if input.multiline => {
                    caret.ime_preedit = None;
                    commands.trigger(UTextInputSubmit {
                        entity,
                        value: input.value.clone(),
                    });
                    if input.clear_on_submit {
                        input.value.clear();
                        input.cursor_position = 0;
                        input.selection = None;
                        changed(&mut commands, &input);
                    }
                }
                _ => continue,
            }
            caret.reset_blink();
            continue;
        }

        // Shift+Insert pastes
        if shift && ev.key_code == KeyCode::Insert {
            if let Some(text) = get_clipboard_text()
                && insert_text(&mut input, &text)
            {
                caret.ime_preedit = None;
                commands.trigger(UTextInputChanged {
                    entity,
                    value: input.value.clone(),
                });
            }
            caret.reset_blink();
            continue;
        }

        match ev.key_code {
            KeyCode::Backspace => {
                caret.ime_preedit = None;
                if delete_backward(&mut input) {
                    commands.trigger(UTextInputChanged {
                        entity,
                        value: input.value.clone(),
                    });
                }
                caret.reset_blink();
            }
            KeyCode::Delete => {
                caret.ime_preedit = None;
                if delete_forward(&mut input) {
                    commands.trigger(UTextInputChanged {
                        entity,
                        value: input.value.clone(),
                    });
                }
                caret.reset_blink();
            }
            KeyCode::ArrowLeft => {
                caret.ime_preedit = None;
                move_cursor_left(&mut input, shift);
                caret.reset_blink();
            }
            KeyCode::ArrowRight => {
                caret.ime_preedit = None;
                move_cursor_right(&mut input, shift);
                caret.reset_blink();
            }
            KeyCode::ArrowUp if input.multiline => {
                caret.ime_preedit = None;
                if let Some((label, cache, computed)) = parley_ready {
                    let cur_disp = value_byte_to_display_byte(&input, input.cursor_position);
                    let next_disp = text_move_caret_vertical(label, cache, computed, cur_disp, -1);
                    let next_val = display_byte_to_value_byte(&input, next_disp);
                    move_cursor_to_byte(&mut input, next_val, shift);
                } else {
                    move_cursor_up(&mut input, shift);
                }
                caret.reset_blink();
            }
            KeyCode::ArrowDown if input.multiline => {
                caret.ime_preedit = None;
                if let Some((label, cache, computed)) = parley_ready {
                    let cur_disp = value_byte_to_display_byte(&input, input.cursor_position);
                    let next_disp = text_move_caret_vertical(label, cache, computed, cur_disp, 1);
                    let next_val = display_byte_to_value_byte(&input, next_disp);
                    move_cursor_to_byte(&mut input, next_val, shift);
                } else {
                    move_cursor_down(&mut input, shift);
                }
                caret.reset_blink();
            }
            KeyCode::Home => {
                caret.ime_preedit = None;
                if input.multiline
                    && let Some((label, cache, computed)) = parley_ready
                {
                    let cur_disp = value_byte_to_display_byte(&input, input.cursor_position);
                    let next_disp = text_move_caret_line_edge(label, cache, computed, cur_disp, false);
                    let next_val = display_byte_to_value_byte(&input, next_disp);
                    move_cursor_to_byte(&mut input, next_val, shift);
                } else {
                    move_cursor_to_start(&mut input, shift);
                }
                caret.reset_blink();
            }
            KeyCode::End => {
                caret.ime_preedit = None;
                if input.multiline
                    && let Some((label, cache, computed)) = parley_ready
                {
                    let cur_disp = value_byte_to_display_byte(&input, input.cursor_position);
                    let next_disp = text_move_caret_line_edge(label, cache, computed, cur_disp, true);
                    let next_val = display_byte_to_value_byte(&input, next_disp);
                    move_cursor_to_byte(&mut input, next_val, shift);
                } else {
                    move_cursor_to_end(&mut input, shift);
                }
                caret.reset_blink();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                caret.ime_preedit = None;
                if input.multiline {
                    if insert_text(&mut input, "\n") {
                        commands.trigger(UTextInputChanged {
                            entity,
                            value: input.value.clone(),
                        });
                    }
                } else {
                    commands.trigger(UTextInputSubmit {
                        entity,
                        value: input.value.clone(),
                    });

                    if input.clear_on_submit {
                        input.value.clear();
                        input.cursor_position = 0;
                        input.selection = None;
                        commands.trigger(UTextInputChanged {
                            entity,
                            value: String::new(),
                        });
                    }
                }
                caret.reset_blink();
            }
            KeyCode::Escape => {
                caret.ime_preedit = None;
                clear_selection(&mut input);
                commands.trigger(UTextInputCancelled { entity });
            }
            _ => {
                let typed: Option<&str> = match &ev.logical_key {
                    Key::Character(smol) => Some(smol.as_str()),
                    Key::Space => Some(" "),
                    _ => None,
                };
                if let Some(s) = typed
                    && !s.is_empty()
                    && !s.chars().any(|c| c.is_control())
                {
                    caret.ime_preedit = None;
                    if insert_text(&mut input, s) {
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
}

/// Blinks the caret cursor on active focused inputs.
pub fn text_input_caret_blink_system(
    time: Option<Res<Time>>,
    focus_state: Option<Res<UFocusState>>,
    mut carets: Query<(Entity, &mut UTextInputCaret)>,
) {
    let (Some(time), Some(focus_state)) = (time, focus_state) else {
        return;
    };
    for (entity, mut caret) in carets.iter_mut() {
        if focus_state.focused == Some(entity) {
            caret.timer.tick(time.delta());
            if caret.timer.just_finished() {
                caret.visible = !caret.visible;
            }
        } else {
            caret.visible = false;
        }
    }
}

/// Automatically links child viewport, text, caret, and selection marker entities spawned
/// via BSN or composite builders to the parent [`UTextInputCaret`].
pub fn link_text_input_children_system(
    mut commands: Commands,
    mut inputs: Query<(Entity, &mut UTextInputCaret, Option<&UNode>, Option<&UClip>)>,
    children_q: Query<&Children>,
    viewport_markers: Query<(), With<UTextInputViewportMarker>>,
    text_markers: Query<(), With<UTextInputTextMarker>>,
    caret_markers: Query<(), With<UTextInputCaretMarker>>,
    selection_markers: Query<(), With<UTextInputSelectionMarker>>,
) {
    for (entity, mut caret, parent_node, parent_clip) in inputs.iter_mut() {
        let missing = caret.viewport_entity.is_none()
            || caret.text_entity.is_none()
            || caret.caret_entity.is_none()
            || caret.selection_entity.is_none();

        if missing && let Ok(children) = children_q.get(entity) {
            for child in children.iter() {
                if caret.viewport_entity.is_none() && viewport_markers.get(child).is_ok() {
                    caret.viewport_entity = Some(child);
                }
                if caret.text_entity.is_none() && text_markers.get(child).is_ok() {
                    caret.text_entity = Some(child);
                }
                if caret.caret_entity.is_none() && caret_markers.get(child).is_ok() {
                    caret.caret_entity = Some(child);
                }
                if caret.selection_entity.is_none() && selection_markers.get(child).is_ok() {
                    caret.selection_entity = Some(child);
                }
                if viewport_markers.get(child).is_ok()
                    && let Ok(grandchildren) = children_q.get(child)
                {
                    for grandchild in grandchildren.iter() {
                        if caret.text_entity.is_none() && text_markers.get(grandchild).is_ok() {
                            caret.text_entity = Some(grandchild);
                        }
                        if caret.caret_entity.is_none() && caret_markers.get(grandchild).is_ok() {
                            caret.caret_entity = Some(grandchild);
                        }
                        if caret.selection_entity.is_none()
                            && selection_markers.get(grandchild).is_ok()
                        {
                            caret.selection_entity = Some(grandchild);
                        }
                    }
                }
            }
        }

        // Direct-children BSN inputs without a dedicated viewport still need clipping so
        // horizontally or vertically scrolled text stays bounded inside the input container.
        if caret.viewport_entity.is_none()
            && caret.text_entity.is_some()
            && parent_node.is_some()
            && parent_clip.is_none()
        {
            commands.entity(entity).try_insert(UClip::enabled(true));
        }
    }
}

struct ComposedDisplay {
    is_placeholder: bool,
    text: String,
    color: Color,
    cursor_byte: usize,
    selection: Option<(usize, usize)>,
}

fn compose_text_display(input: &UTextInput, caret: &UTextInputCaret) -> ComposedDisplay {
    let preedit = caret.ime_preedit.as_ref().filter(|p| !p.value.is_empty());
    if input.value.is_empty() && preedit.is_none() {
        return ComposedDisplay {
            is_placeholder: true,
            text: input.placeholder.clone(),
            color: input.placeholder_color,
            cursor_byte: 0,
            selection: None,
        };
    }

    let base = display_text(input);
    if let Some(pre) = preedit {
        let (rep_start, rep_end) = if let Some((s, e)) = selection_bounds(input) {
            (
                value_byte_to_display_byte(input, s),
                value_byte_to_display_byte(input, e),
            )
        } else {
            let cur = value_byte_to_display_byte(input, input.cursor_position);
            (cur, cur)
        };
        let rep_start = rep_start.min(base.len());
        let rep_end = rep_end.clamp(rep_start, base.len());

        let mut composed = String::with_capacity(base.len() + pre.value.len());
        composed.push_str(&base[..rep_start]);
        composed.push_str(&pre.value);
        composed.push_str(&base[rep_end..]);

        let pre_len = pre.value.len();
        let cursor_byte = rep_start + pre.cursor.map_or(pre_len, |(_, end)| end.min(pre_len));
        let selection = pre.cursor.and_then(|(s, e)| {
            let cs = s.min(pre_len);
            let ce = e.min(pre_len);
            (cs != ce).then_some((rep_start + cs.min(ce), rep_start + cs.max(ce)))
        });

        ComposedDisplay {
            is_placeholder: false,
            text: composed,
            color: input.text_color,
            cursor_byte,
            selection,
        }
    } else {
        let cursor_byte = value_byte_to_display_byte(input, input.cursor_position);
        let selection = selection_bounds(input).map(|(s, e)| {
            (
                value_byte_to_display_byte(input, s),
                value_byte_to_display_byte(input, e),
            )
        });
        ComposedDisplay {
            is_placeholder: false,
            text: base,
            color: input.text_color,
            cursor_byte,
            selection,
        }
    }
}

fn parley_local_rect_to_top_left(
    rect: UTextSelectionRect,
    text_node_size: Vec2,
) -> (f32, f32, f32, f32) {
    let left = text_node_size.x * 0.5 + (rect.center.x - rect.size.x * 0.5);
    let top = text_node_size.y * 0.5 - (rect.center.y + rect.size.y * 0.5);
    (left, top, rect.size.x, rect.size.y)
}

fn apply_overlay_rect(
    node: &mut UNode,
    self_style: &mut USelf,
    visibility: &mut Visibility,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: Color,
    target_vis: Visibility,
) {
    if *visibility != target_vis {
        *visibility = target_vis;
    }
    if node.background_color != color {
        node.background_color = color;
    }
    let w = UVal::Px(width);
    if node.width != w {
        node.width = w;
    }
    let h = UVal::Px(height);
    if node.height != h {
        node.height = h;
    }
    let l = UVal::Px(left);
    if self_style.left != l {
        self_style.left = l;
    }
    let t = UVal::Px(top);
    if self_style.top != t {
        self_style.top = t;
    }
}

/// Synchronizes the visual child text label, viewport scroll, caret, and selection highlight quads
/// with the [`UTextInput`] and IME composition state.
pub fn text_input_sync_visual_system(
    mut commands: Commands,
    focus_state: Option<Res<UFocusState>>,
    mut inputs: Query<(Entity, &UTextInput, &mut UTextInputCaret)>,
    parent_layout_q: Query<
        (&UNode, &ComputedSize),
        (
            With<UTextInput>,
            Without<UTextInputViewportMarker>,
            Without<UTextInputTextMarker>,
            Without<UTextInputCaretMarker>,
            Without<UTextInputSelectionMarker>,
        ),
    >,
    viewport_layout_q: Query<
        &ComputedSize,
        (
            With<UTextInputViewportMarker>,
            Without<UTextInput>,
            Without<UTextInputTextMarker>,
            Without<UTextInputCaretMarker>,
            Without<UTextInputSelectionMarker>,
        ),
    >,
    mut label_query: Query<
        (
            &mut UTextLabel,
            &mut UNode,
            &mut USelf,
            Option<&ComputedSize>,
            Option<&UTextLabelLayoutCache>,
            Option<&ComputedTextBlock>,
        ),
        (
            With<UTextInputTextMarker>,
            Without<UTextInput>,
            Without<UTextInputViewportMarker>,
            Without<UTextInputCaretMarker>,
            Without<UTextInputSelectionMarker>,
        ),
    >,
    mut caret_query: Query<
        (&mut UNode, &mut USelf, &mut Visibility),
        (
            With<UTextInputCaretMarker>,
            Without<UTextInput>,
            Without<UTextInputViewportMarker>,
            Without<UTextInputTextMarker>,
            Without<UTextInputSelectionMarker>,
        ),
    >,
    mut selection_query: Query<
        (&mut UNode, &mut USelf, &mut Visibility),
        (
            With<UTextInputSelectionMarker>,
            Without<UTextInput>,
            Without<UTextInputViewportMarker>,
            Without<UTextInputTextMarker>,
            Without<UTextInputCaretMarker>,
        ),
    >,
) {
    for (entity, input, mut caret) in inputs.iter_mut() {
        let is_focused = focus_state
            .as_ref()
            .is_some_and(|f| f.focused == Some(entity));

        let composed = compose_text_display(input, &caret);

        // 1. Determine viewport inner dimensions and origin in overlay container space
        let (container_entity, origin_left, origin_top, vw, vh) = if let Some(vp) =
            caret.viewport_entity
            && let Ok(vp_size) = viewport_layout_q.get(vp)
        {
            (vp, 0.0, 0.0, vp_size.width.max(0.0), vp_size.height.max(0.0))
        } else if let Ok((p_node, p_size)) = parent_layout_q.get(entity) {
            let pad_l = p_node.padding.left;
            let pad_t = p_node.padding.top;
            let w = (p_size.width - pad_l - p_node.padding.right).max(0.0);
            let h = (p_size.height - pad_t - p_node.padding.bottom).max(0.0);
            (entity, pad_l, pad_t, w, h)
        } else {
            (caret.viewport_entity.unwrap_or(entity), 14.0, 6.0, 0.0, 0.0)
        };

        // 2. Sync label component and read current Parley layout metrics if settled
        let mut measured_size = Vec2::ZERO;
        let mut text_node_size = Vec2::ZERO;
        let default_h = caret.height.unwrap_or(input.font_size * 1.2);
        let mut caret_in_text = (0.0_f32, 0.0_f32, default_h);
        let mut sel_rects_in_text: Vec<(f32, f32, f32, f32)> = Vec::new();
        let mut parley_settled = false;

        if let Some(text_entity) = caret.text_entity
            && let Ok((
                mut label,
                mut text_node,
                _text_self,
                text_cs_opt,
                cache_opt,
                computed_opt,
            )) = label_query.get_mut(text_entity)
        {
            if label.text != composed.text {
                label.text = composed.text.clone();
            }
            if label.color != composed.color {
                label.color = composed.color;
            }
            if label.font_size != input.font_size {
                label.font_size = input.font_size;
            }
            if !label.autosize {
                label.autosize = true;
            }
            if label.overflow != UTextOverflow::Clip {
                label.overflow = UTextOverflow::Clip;
            }

            let target_wrap = if input.multiline {
                UTextWrapMode::WrapToParent
            } else {
                UTextWrapMode::SingleLine
            };
            if label.wrap_mode != target_wrap {
                label.wrap_mode = target_wrap;
            }
            if input.multiline && label.linebreak == LineBreak::NoWrap {
                label.linebreak = LineBreak::WordBoundary;
            } else if !input.multiline && label.linebreak != LineBreak::NoWrap {
                label.linebreak = LineBreak::NoWrap;
            }

            let target_text_w = if input.multiline {
                UVal::Percent(1.0)
            } else {
                UVal::Auto
            };
            if text_node.width != target_text_w {
                text_node.width = target_text_w;
            }

            if let Some(cache) = cache_opt {
                measured_size = cache.measured_size;
                text_node_size = text_cs_opt
                    .map(|cs| Vec2::new(cs.width, cs.height))
                    .unwrap_or(cache.measured_size);

                if !composed.is_placeholder
                    && let Some(computed) = computed_opt
                    && !computed.buffer().is_empty()
                    && cache.displayed_text == composed.text
                {
                    parley_settled = true;
                    let rect = text_caret_rect_from_byte_index(
                        &label,
                        &text_node,
                        text_cs_opt.unwrap_or(&ComputedSize {
                            width: text_node_size.x,
                            height: text_node_size.y,
                            ..default()
                        }),
                        cache,
                        computed,
                        composed.cursor_byte,
                        caret.width,
                    );
                    let (cx, cy, _, ch) = parley_local_rect_to_top_left(rect, text_node_size);
                    caret_in_text = (cx, cy, caret.height.unwrap_or(ch));

                    if let Some(sel) = composed.selection {
                        let rects = compute_text_selection_rects(
                            &label,
                            &text_node,
                            text_cs_opt.unwrap_or(&ComputedSize {
                                width: text_node_size.x,
                                height: text_node_size.y,
                                ..default()
                            }),
                            cache,
                            computed,
                            sel,
                        );
                        sel_rects_in_text = rects
                            .into_iter()
                            .map(|r| parley_local_rect_to_top_left(r, text_node_size))
                            .collect();
                    }
                }
            }
        }

        if !parley_settled && !composed.is_placeholder {
            let cx = char_offset_to_x(input, input.cursor_position, measured_size.x);
            caret_in_text = (cx, 0.0, default_h);
            if let Some((s, e)) = selection_bounds(input) {
                let sx = char_offset_to_x(input, s, measured_size.x);
                let ex = char_offset_to_x(input, e, measured_size.x);
                sel_rects_in_text.push((sx, 0.0, (ex - sx).max(2.0), default_h));
            }
        }

        // 3. Keep caret comfortably visible inside clipped viewport (horizontal & vertical scrolling)
        let mut next_scroll = caret.scroll_offset;
        if vw > 0.0 {
            let (cx, cy, ch) = caret_in_text;
            if input.multiline {
                next_scroll.x = 0.0;
                if vh > 0.0 {
                    let max_sy = (text_node_size.y.max(measured_size.y) - vh).max(0.0);
                    if cy + ch - next_scroll.y > vh {
                        next_scroll.y = cy + ch - vh;
                    }
                    if cy - next_scroll.y < 0.0 {
                        next_scroll.y = cy;
                    }
                    next_scroll.y = next_scroll.y.clamp(0.0, max_sy);
                }
            } else {
                next_scroll.y = 0.0;
                let max_sx = (text_node_size.x.max(measured_size.x) - vw).max(0.0);
                let margin = caret.width.max(2.0);
                if cx - next_scroll.x > vw - margin {
                    next_scroll.x = cx - (vw - margin);
                }
                if cx - next_scroll.x < 0.0 {
                    next_scroll.x = cx;
                }
                next_scroll.x = next_scroll.x.clamp(0.0, max_sx);
            }
        }
        if next_scroll.distance_squared(caret.scroll_offset) > 1e-4 {
            caret.scroll_offset = next_scroll;
        }

        // 4. Compute text anchor in overlay container and apply scroll offset to text node
        let text_h = if text_node_size.y > 0.0 {
            text_node_size.y
        } else {
            default_h
        };
        let base_text_top = if input.multiline {
            origin_top
        } else {
            origin_top + ((vh - text_h) * 0.5).max(0.0)
        };
        let text_origin_x = origin_left - caret.scroll_offset.x;
        let text_origin_y = base_text_top - caret.scroll_offset.y;

        if let Some(text_entity) = caret.text_entity
            && let Ok((_, _, mut text_self, _, _, _)) = label_query.get_mut(text_entity)
        {
            if text_self.position_type == UPositionType::Absolute {
                let target_left = UVal::Px(text_origin_x);
                let target_top = UVal::Px(text_origin_y);
                if text_self.left != target_left {
                    text_self.left = target_left;
                }
                if text_self.top != target_top {
                    text_self.top = target_top;
                }
            } else {
                let target_left = UVal::Px(-caret.scroll_offset.x);
                let target_top = UVal::Px(-caret.scroll_offset.y);
                if text_self.left != target_left {
                    text_self.left = target_left;
                }
                if text_self.top != target_top {
                    text_self.top = target_top;
                }
                let target_shrink = if input.multiline { None } else { Some(0.0) };
                if text_self.flex_shrink != target_shrink {
                    text_self.flex_shrink = target_shrink;
                }
            }
        }

        // 5. Update caret node position and visibility
        if let Some(caret_entity) = caret.caret_entity
            && let Ok((mut caret_node, mut caret_self, mut visibility)) =
                caret_query.get_mut(caret_entity)
        {
            let target_vis = if is_focused && caret.visible && input.enabled {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            apply_overlay_rect(
                &mut caret_node,
                &mut caret_self,
                &mut visibility,
                text_origin_x + caret_in_text.0,
                text_origin_y + caret_in_text.1,
                caret.width,
                caret_in_text.2,
                caret.color,
                target_vis,
            );
        }

        // 6. Update primary and extra multiline selection highlight quads
        if let Some(first_rect) = sel_rects_in_text.first().copied() {
            if let Some(sel_entity) = caret.selection_entity
                && let Ok((mut sel_node, mut sel_self, mut sel_vis)) =
                    selection_query.get_mut(sel_entity)
            {
                apply_overlay_rect(
                    &mut sel_node,
                    &mut sel_self,
                    &mut sel_vis,
                    text_origin_x + first_rect.0,
                    text_origin_y + first_rect.1,
                    first_rect.2.max(2.0),
                    first_rect.3.max(1.0),
                    caret.selection_color,
                    Visibility::Inherited,
                );
            }

            let extra_needed = sel_rects_in_text.len().saturating_sub(1);
            for (idx, rect) in sel_rects_in_text.iter().skip(1).copied().enumerate() {
                let left = text_origin_x + rect.0;
                let top = text_origin_y + rect.1;
                let width = rect.2.max(2.0);
                let height = rect.3.max(1.0);
                if let Some(&extra_entity) = caret.extra_selection_entities.get(idx) {
                    if let Ok((mut sel_node, mut sel_self, mut sel_vis)) =
                        selection_query.get_mut(extra_entity)
                    {
                        apply_overlay_rect(
                            &mut sel_node,
                            &mut sel_self,
                            &mut sel_vis,
                            left,
                            top,
                            width,
                            height,
                            caret.selection_color,
                            Visibility::Inherited,
                        );
                    }
                } else {
                    let spawned = commands
                        .spawn((
                            ChildOf(container_entity),
                            UNode {
                                width: UVal::Px(width),
                                height: UVal::Px(height),
                                background_color: caret.selection_color,
                                border_radius: UCornerRadius::all(2.0),
                                ..default()
                            },
                            USelf {
                                position_type: UPositionType::Absolute,
                                left: UVal::Px(left),
                                top: UVal::Px(top),
                                ..default()
                            },
                            UZIndex::Local(-1),
                            Visibility::Inherited,
                            UTextInputSelectionMarker,
                        ))
                        .id();
                    caret.extra_selection_entities.push(spawned);
                }
            }

            for &unused_entity in caret.extra_selection_entities.iter().skip(extra_needed) {
                if let Ok((_, _, mut vis)) = selection_query.get_mut(unused_entity)
                    && *vis != Visibility::Hidden
                {
                    *vis = Visibility::Hidden;
                }
            }
        } else {
            if let Some(sel_entity) = caret.selection_entity
                && let Ok((_, _, mut sel_vis)) = selection_query.get_mut(sel_entity)
                && *sel_vis != Visibility::Hidden
            {
                *sel_vis = Visibility::Hidden;
            }
            for &extra_entity in caret.extra_selection_entities.iter() {
                if let Ok((_, _, mut vis)) = selection_query.get_mut(extra_entity)
                    && *vis != Visibility::Hidden
                {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
}

type TextLayoutQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static UTextLabel>,
        Option<&'static UNode>,
        Option<&'static ComputedSize>,
        Option<&'static UTextLabelLayoutCache>,
        Option<&'static ComputedTextBlock>,
        Option<&'static GlobalTransform>,
    ),
>;

fn resolve_pointer_byte_and_word(
    hit_world: Option<Vec3>,
    screen_pointer: Option<(Vec2, Option<&bevy::camera::NormalizedRenderTarget>)>,
    parent_gt: &GlobalTransform,
    parent_computed: &ComputedSize,
    parent_node: Option<&UNode>,
    world_scale: f32,
    preferred_camera: Option<Entity>,
    primary_window: Option<Entity>,
    cameras: Option<&Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>>,
    input: &UTextInput,
    caret: &UTextInputCaret,
    text_query: &TextLayoutQuery,
) -> Option<(usize, Option<(usize, usize)>)> {
    let parley_item = caret.text_entity.and_then(|te| text_query.get(te).ok());
    let measured_width = parley_item
        .and_then(|(_, _, _, cache, _, _)| cache.map(|c| c.measured_size.x))
        .unwrap_or(0.0);

    // Exact Parley point picking when the child text block has been laid out for the current string
    if let Some((Some(label), Some(text_node), Some(text_cs), Some(cache), Some(computed), text_gt_opt)) =
        parley_item
        && !computed.buffer().is_empty()
        && caret.ime_preedit.is_none()
        && cache.displayed_text == display_text(input)
    {
        if input.value.is_empty() {
            return Some((0, None));
        }

        let local_pt = if let (Some(world_pos), Some(text_gt)) = (hit_world, text_gt_opt) {
            Some(hit_world_to_local_logical(world_pos, text_gt, world_scale))
        } else if let (Some((screen_pos, target)), Some(cams), Some(text_gt)) =
            (screen_pointer, cameras, text_gt_opt)
        {
            project_screen_point_to_node(
                screen_pos,
                target,
                None,
                primary_window,
                preferred_camera,
                text_gt,
                world_scale,
                false,
                cams,
            )
            .map(|(_, _, _, local_logical, _)| local_logical)
        } else if let Some(world_pos) = hit_world {
            let parent_local = hit_world_to_local_logical(world_pos, parent_gt, world_scale);
            Some(parent_local - text_cs.local_pos)
        } else if let (Some((screen_pos, target)), Some(cams)) = (screen_pointer, cameras) {
            project_screen_point_to_node(
                screen_pos,
                target,
                None,
                primary_window,
                preferred_camera,
                parent_gt,
                world_scale,
                false,
                cams,
            )
            .map(|(_, _, _, parent_local, _)| parent_local - text_cs.local_pos)
        } else {
            None
        };

        if let Some(pt) = local_pt {
            let disp_idx =
                text_byte_index_from_local_point(label, text_node, text_cs, cache, computed, pt);
            let byte_idx = display_byte_to_value_byte(input, disp_idx);
            let (ws_disp, we_disp) =
                text_word_range_from_local_point(label, text_node, text_cs, cache, computed, pt);
            let word_range = (
                display_byte_to_value_byte(input, ws_disp),
                display_byte_to_value_byte(input, we_disp),
            );
            return Some((byte_idx, (word_range.0 < word_range.1).then_some(word_range)));
        }
    }

    // Headless / fallback projection (used when TextPlugin layout cache is unavailable)
    let parent_local_x = if let Some(world_pos) = hit_world {
        hit_world_to_local_logical(world_pos, parent_gt, world_scale).x
    } else if let (Some((screen_pos, target)), Some(cams)) = (screen_pointer, cameras) {
        project_screen_point_to_node(
            screen_pos,
            target,
            None,
            primary_window,
            preferred_camera,
            parent_gt,
            world_scale,
            false,
            cams,
        )?
        .3
        .x
    } else {
        return None;
    };

    let padding_left = parent_node.map_or(14.0, |n| n.padding.left);
    let x_from_left = parent_local_x + parent_computed.width / 2.0;
    let x_in_text = (x_from_left - padding_left + caret.scroll_offset.x).max(0.0);
    Some((
        cursor_index_from_x_offset(input, x_in_text, measured_width),
        None,
    ))
}

/// Observer handling pointer down / press to position the caret, initiate drag selection,
/// or double-click to select word.
pub fn on_text_input_pointer_press(
    trigger: On<PointerPress>,
    time: Option<Res<Time>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut commands: Commands,
    mut focus_state: Option<ResMut<UFocusState>>,
    mut query: Query<(
        Entity,
        &mut UTextInput,
        &mut UTextInputCaret,
        &UFocusable,
        &GlobalTransform,
        &ComputedSize,
        Option<&UNode>,
        Option<&UiPickingContext>,
    )>,
    text_query: TextLayoutQuery,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }
    let entity = trigger.entity;
    let Ok((
        _,
        mut input,
        mut caret,
        focusable,
        gt,
        computed,
        parent_node,
        picking_ctx,
    )) = query.get_mut(entity)
    else {
        return;
    };

    if !focusable.enabled || !input.enabled {
        return;
    }

    if let Some(ref mut focus_state) = focus_state {
        set_focus(
            &mut commands,
            focus_state,
            Some(entity),
            FocusModality::Pointer,
        );
    }

    let world_scale = resolve_node_world_scale(entity, picking_ctx, &parents_query, &root_query);
    let hit_world = trigger.event().hit.position.unwrap_or(gt.translation());
    let (new_cursor, parley_word) = resolve_pointer_byte_and_word(
        Some(hit_world),
        None,
        gt,
        computed,
        parent_node,
        world_scale,
        None,
        None,
        None,
        &input,
        &caret,
        &text_query,
    )
    .unwrap_or((input.cursor_position, None));

    caret.ime_preedit = None;

    let now = time.as_ref().map_or(0.0, |t| t.elapsed_secs_f64());
    let is_double_click =
        (now - caret.last_click_secs) < 0.35 && (now - caret.last_click_secs) > 0.0;
    caret.last_click_secs = now;

    let is_shift = keyboard
        .as_ref()
        .is_some_and(|k| k.pressed(KeyCode::ShiftLeft) || k.pressed(KeyCode::ShiftRight));
    if is_double_click {
        if let Some((ws, we)) = parley_word {
            input.selection = Some((ws, we));
            input.cursor_position = we;
        } else {
            input.cursor_position = new_cursor;
            select_word_at_cursor(&mut input);
        }
        caret.drag_anchor = None;
    } else if is_shift {
        let anchor = input
            .selection
            .map(|(s, _)| s)
            .unwrap_or(input.cursor_position);
        input.cursor_position = new_cursor;
        if anchor != new_cursor {
            input.selection = Some((anchor, new_cursor));
        } else {
            input.selection = None;
        }
        caret.drag_anchor = Some(anchor);
    } else {
        input.cursor_position = new_cursor;
        input.selection = None;
        caret.drag_anchor = Some(new_cursor);
    }

    caret.reset_blink();
}

/// Observer handling pointer dragging across the text input to update selection range.
pub fn on_text_input_pointer_drag(
    trigger: On<PointerDrag>,
    windows: Query<(Entity, Option<&PrimaryWindow>)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
    mut query: Query<(
        Entity,
        &mut UTextInput,
        &mut UTextInputCaret,
        &GlobalTransform,
        &ComputedSize,
        Option<&UNode>,
        Option<&UiPickingContext>,
    )>,
    text_query: TextLayoutQuery,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
) {
    let entity = trigger.entity;
    let Ok((
        _,
        mut input,
        mut caret,
        gt,
        computed,
        parent_node,
        picking_ctx,
    )) = query.get_mut(entity)
    else {
        return;
    };

    if !input.enabled {
        return;
    }

    let Some(anchor) = caret.drag_anchor else {
        return;
    };

    let preferred_camera = picking_ctx.and_then(|ctx| ctx.camera_entity);
    let world_scale = resolve_node_world_scale(entity, picking_ctx, &parents_query, &root_query);
    let primary_window =
        resolve_primary_window_entity(windows.iter().map(|(e, primary)| (e, primary.is_some())));

    let Some((drag_cursor, _)) = resolve_pointer_byte_and_word(
        None,
        Some((trigger.pointer.position, Some(&trigger.pointer.target))),
        gt,
        computed,
        parent_node,
        world_scale,
        preferred_camera,
        primary_window,
        Some(&cameras),
        &input,
        &caret,
        &text_query,
    ) else {
        return;
    };

    caret.ime_preedit = None;
    input.cursor_position = drag_cursor;
    if anchor != drag_cursor {
        input.selection = Some((anchor, drag_cursor));
    } else {
        input.selection = None;
    }

    caret.reset_blink();
}

/// Observer clearing the drag anchor when the pointer is released.
pub fn on_text_input_pointer_release(
    trigger: On<PointerRelease>,
    mut query: Query<(&mut UTextInput, &mut UTextInputCaret)>,
) {
    let entity = trigger.entity;
    if let Ok((mut input, mut caret)) = query.get_mut(entity) {
        caret.drag_anchor = None;
        if let Some((s, e)) = input.selection
            && s == e
        {
            input.selection = None;
        }
    }
}

/// Observer focusing a text input field on click and resetting its caret blink.
pub fn on_text_input_pointer_click(
    trigger: On<PointerClick>,
    mut commands: Commands,
    mut focus_state: Option<ResMut<UFocusState>>,
    mut query: Query<(&mut UTextInput, &mut UTextInputCaret, &UFocusable)>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }
    let entity = trigger.entity;
    if let Ok((_, mut caret, focusable)) = query.get_mut(entity)
        && focusable.enabled
    {
        if let Some(ref mut focus_state) = focus_state {
            set_focus(
                &mut commands,
                focus_state,
                Some(entity),
                FocusModality::Pointer,
            );
        }
        caret.reset_blink();
    }
}

/// Observer clearing selections and uncommitted IME composition when a text input loses focus.
pub fn on_text_input_focus_lost(
    trigger: On<FocusLost>,
    mut query: Query<(&mut UTextInput, &mut UTextInputCaret)>,
) {
    let entity = trigger.entity;
    if let Ok((mut input, mut caret)) = query.get_mut(entity) {
        clear_selection(&mut input);
        caret.ime_preedit = None;
        caret.visible = false;
        caret.drag_anchor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::LumaInteractionPlugin;
    use bevy::camera::{
        CameraProjection, ComputedCameraValues, NormalizedRenderTarget, RenderTargetInfo,
    };
    use bevy::picking::backend::{HitData, PointerHits};
    use bevy::picking::events::{Pointer, PointerDrag, PointerPress};
    use bevy::picking::pointer::{Location, PointerId, PointerMap};
    use bevy::window::WindowRef;
    use luma_engine::layout::layout_system::{URootUi, UiSpace};

    #[test]
    fn pointer_press_and_drag_handle_secondary_window_world3d_with_world_scale() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin));
        app.add_observer(on_text_input_pointer_press);
        app.add_observer(on_text_input_pointer_drag);

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
                camera.clone(),
                RenderTarget::Window(WindowRef::Entity(secondary_win)),
                Transform::from_xyz(0.0, 0.0, 5.0),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 5.0)),
            ))
            .id();

        let text_child = app
            .world_mut()
            .spawn(UTextLabelLayoutCache {
                measured_size: Vec2::new(100.0, 20.0),
                ..default()
            })
            .id();

        let world_scale = URootUi::DEFAULT_METERS_PER_UNIT;
        let mut input = UTextInput::new("").with_value("0123456789");
        input.cursor_position = 0;
        let mut caret = UTextInputCaret::default();
        caret.text_entity = Some(text_child);

        let input_entity = app
            .world_mut()
            .spawn((
                input,
                caret,
                UFocusable::default(),
                ComputedSize {
                    width: 200.0,
                    height: 40.0,
                    ..default()
                },
                Transform::from_scale(Vec3::splat(world_scale)),
                GlobalTransform::from(Transform::from_scale(Vec3::splat(world_scale))),
                UiPickingContext {
                    root_entity: None,
                    camera_entity: Some(sec_camera),
                    space: UiSpace::World3d,
                    clip_ancestor: None,
                    root_sort_key: 0.0,
                    local_depth_key: 0.0,
                    world_scale,
                },
            ))
            .id();

        let camera_gt = *app.world().get::<GlobalTransform>(sec_camera).unwrap();
        let target: NormalizedRenderTarget = RenderTarget::Window(WindowRef::Entity(secondary_win))
            .normalize(Some(primary_win))
            .unwrap();

        // Hit near left side of text (-40 logical px -> * world_scale in 3D world) and drag right (+40 logical px).
        let start_world = Vec3::new(-40.0 * world_scale, 0.0, 0.0);
        let end_world = Vec3::new(40.0 * world_scale, 0.0, 0.0);
        let start_screen = camera.world_to_viewport(&camera_gt, start_world).unwrap();
        let end_screen = camera.world_to_viewport(&camera_gt, end_world).unwrap();

        app.world_mut().trigger(PointerPress {
            entity: input_entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: target.clone(),
                    position: start_screen,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(sec_camera, 5.0, Some(start_world), Some(Vec3::Z)),
            count: 1,
        });
        let anchor_after_press = app
            .world()
            .get::<UTextInput>(input_entity)
            .unwrap()
            .cursor_position;
        assert!(anchor_after_press <= 3, "expected left cursor on press, got {anchor_after_press}");

        app.world_mut().trigger(PointerDrag {
            entity: input_entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target,
                    position: end_screen,
                },
            ),
            button: PointerButton::Primary,
            distance: end_screen - start_screen,
            delta: end_screen - start_screen,
        });

        let updated = app.world().get::<UTextInput>(input_entity).unwrap();
        assert!(
            updated.cursor_position >= 7,
            "expected right cursor after drag in 3D secondary window, got {}",
            updated.cursor_position
        );
        let (sel_start, sel_end) = updated.selection.expect("drag should create selection");
        assert!(sel_end > sel_start);
        let _ = LumaInteractionPlugin;
    }

    #[derive(Resource, Default)]
    struct RecordedTextEvents {
        changed: Vec<String>,
        submitted: Vec<String>,
    }

    fn build_full_text_input_test_app() -> (App, Entity, Entity, Entity, Handle<Font>, Handle<Font>)
    {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            luma_engine::LumaEnginePlugin,
            LumaInteractionPlugin,
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<Image>();
        app.init_asset::<TextureAtlasLayout>();
        app.init_asset::<bevy::sprite_render::ColorMaterial>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_message::<KeyboardInput>();
        app.add_message::<bevy::window::Ime>();
        app.init_resource::<PointerMap>();
        app.add_message::<PointerHits>();
        app.init_resource::<RecordedTextEvents>();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            core::time::Duration::from_millis(16),
        ));

        app.add_observer(
            |trigger: On<UTextInputChanged>, mut recorded: ResMut<RecordedTextEvents>| {
                recorded.changed.push(trigger.event().value.clone());
            },
        );
        app.add_observer(
            |trigger: On<UTextInputSubmit>, mut recorded: ResMut<RecordedTextEvents>| {
                recorded.submitted.push(trigger.event().value.clone());
            },
        );

        let win_a = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let win_b = app.world_mut().spawn(Window::default()).id();

        let mut camera = Camera::default();
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(800, 600),
                scale_factor: 1.0,
            }),
            clip_from_view: OrthographicProjection {
                area: Rect::new(-400.0, -300.0, 400.0, 300.0),
                ..OrthographicProjection::default_2d()
            }
            .get_clip_from_view(),
            ..default()
        };
        let cam_a = app
            .world_mut()
            .spawn((
                camera,
                RenderTarget::Window(WindowRef::Primary),
                Transform::from_xyz(0.0, 0.0, 1000.0),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 1000.0)),
            ))
            .id();

        let latin_font = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(
                include_bytes!(
                    "../../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf"
                )
                .to_vec(),
            ));
        let arabic_font = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(
                include_bytes!(
                    "../../../../luma_style/src/style/assets/fonts/NotoSansArabic-Regular.ttf"
                )
                .to_vec(),
            ));

        (app, win_a, win_b, cam_a, latin_font, arabic_font)
    }

    fn spawn_root_input(
        app: &mut App,
        cam: Entity,
        width: f32,
        height: f32,
        input: UTextInput,
        font: Handle<Font>,
    ) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                URootUi {
                    camera: luma_engine::layout::layout_system::UiCameraRef::Entity(cam),
                    ..URootUi::screen()
                },
                UNode {
                    width: UVal::Px(800.0),
                    height: UVal::Px(600.0),
                    ..default()
                },
            ))
            .id();

        let input_entity = {
            let mut cmds = app.world_mut().commands();
            let id = super::super::builder::spawn_text_input(&mut cmds, input);
            cmds.entity(id).insert((
                ChildOf(root),
                UNode {
                    width: UVal::Px(width),
                    height: UVal::Px(height),
                    padding: luma_engine::layout::geometry::USides::axes(14.0, 6.0),
                    background_color: Color::BLACK,
                    ..default()
                },
            ));
            id
        };
        app.update();

        let text_entity = app
            .world()
            .get::<UTextInputCaret>(input_entity)
            .unwrap()
            .text_entity
            .unwrap();
        app.world_mut()
            .get_mut::<UTextLabel>(text_entity)
            .unwrap()
            .font = font;
        input_entity
    }

    #[test]
    fn ime_preedit_commit_and_multiwindow_isolation() {
        let (mut app, win_a, win_b, cam_a, latin_font, _) = build_full_text_input_test_app();
        let input_entity = spawn_root_input(
            &mut app,
            cam_a,
            260.0,
            42.0,
            UTextInput::new("Type here").with_value("ab"),
            latin_font,
        );

        app.world_mut().resource_scope(|world, mut focus_state: Mut<UFocusState>| {
            let mut cmds = world.commands();
            set_focus(
                &mut cmds,
                &mut focus_state,
                Some(input_entity),
                FocusModality::Pointer,
            );
        });
        app.update();
        app.update();

        assert!(app.world().get::<Window>(win_a).unwrap().ime_enabled);
        assert!(!app.world().get::<Window>(win_b).unwrap().ime_enabled);

        // Preedit on win_a renders in label without mutating `input.value` or firing `UTextInputChanged`.
        app.world_mut().write_message(bevy::window::Ime::Preedit {
            window: win_a,
            value: "にほん".into(),
            cursor: Some((0, 9)),
        });
        app.update();

        assert_eq!(app.world().get::<UTextInput>(input_entity).unwrap().value, "ab");
        assert!(app.world().resource::<RecordedTextEvents>().changed.is_empty());
        let text_entity = app
            .world()
            .get::<UTextInputCaret>(input_entity)
            .unwrap()
            .text_entity
            .unwrap();
        assert_eq!(
            app.world().get::<UTextLabel>(text_entity).unwrap().text,
            "abにほん"
        );

        // Events coming from win_b must be ignored while focused on win_a.
        app.world_mut().write_message(bevy::window::Ime::Commit {
            window: win_b,
            value: "WRONG".into(),
        });
        app.update();
        assert_eq!(app.world().get::<UTextInput>(input_entity).unwrap().value, "ab");

        // Commit on win_a clears preedit and emits one change event.
        app.world_mut().write_message(bevy::window::Ime::Commit {
            window: win_a,
            value: "日本".into(),
        });
        app.update();

        let input = app.world().get::<UTextInput>(input_entity).unwrap();
        let caret = app.world().get::<UTextInputCaret>(input_entity).unwrap();
        assert_eq!(input.value, "ab日本");
        assert!(caret.ime_preedit.is_none());
        assert_eq!(
            app.world().resource::<RecordedTextEvents>().changed,
            vec!["ab日本".to_string()]
        );
        assert_eq!(
            app.world().get::<UTextLabel>(text_entity).unwrap().text,
            "ab日本"
        );

        // Unfocus disables IME on win_a cleanly.
        app.world_mut().resource_scope(|world, mut focus_state: Mut<UFocusState>| {
            let mut cmds = world.commands();
            set_focus(&mut cmds, &mut focus_state, None, FocusModality::Pointer);
        });
        app.update();
        assert!(!app.world().get::<Window>(win_a).unwrap().ime_enabled);
    }

    #[test]
    fn parley_variable_width_and_arabic_rtl_click_and_double_click() {
        let (mut app, _win_a, _, cam_a, latin_font, arabic_font) =
            build_full_text_input_test_app();

        // 1. Arabic RTL click + double-click word selection using NotoSansArabic-Regular.ttf
        let rtl_text = "مرحبا بالعالم";
        let rtl_input = spawn_root_input(
            &mut app,
            cam_a,
            300.0,
            42.0,
            UTextInput::new("").with_value(rtl_text).with_font_size(22.0),
            arabic_font,
        );
        app.update();
        app.update();

        let text_entity = app
            .world()
            .get::<UTextInputCaret>(rtl_input)
            .unwrap()
            .text_entity
            .unwrap();
        let text_gt = *app.world().get::<GlobalTransform>(text_entity).unwrap();
        let text_cs = *app.world().get::<ComputedSize>(text_entity).unwrap();
        assert!(text_cs.width > 20.0);

        // In Arabic RTL, clicking the right side hits near the logical start ("مرحبا"),
        // whereas clicking the left side hits near the logical end ("بالعالم").
        let right_world = text_gt.translation() + Vec3::new(text_cs.width * 0.4, 0.0, 0.0);
        let left_world = text_gt.translation() - Vec3::new(text_cs.width * 0.4, 0.0, 0.0);

        let camera = app.world().get::<Camera>(cam_a).unwrap().clone();
        let cam_gt = *app.world().get::<GlobalTransform>(cam_a).unwrap();
        let right_screen = camera.world_to_viewport(&cam_gt, right_world).unwrap();

        app.world_mut().trigger(PointerPress {
            entity: rtl_input,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: right_screen,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(cam_a, 1.0, Some(right_world), Some(Vec3::Z)),
            count: 1,
        });
        let rtl_start_cursor = app.world().get::<UTextInput>(rtl_input).unwrap().cursor_position;
        assert!(rtl_text.is_char_boundary(rtl_start_cursor));

        let left_screen = camera.world_to_viewport(&cam_gt, left_world).unwrap();
        app.world_mut().trigger(PointerPress {
            entity: rtl_input,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: left_screen,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(cam_a, 1.0, Some(left_world), Some(Vec3::Z)),
            count: 1,
        });
        let rtl_end_cursor = app.world().get::<UTextInput>(rtl_input).unwrap().cursor_position;
        assert!(rtl_text.is_char_boundary(rtl_end_cursor));
        assert!(
            rtl_start_cursor < rtl_end_cursor,
            "expected right side of RTL string ({rtl_start_cursor}) < left side ({rtl_end_cursor})"
        );

        // 2. Variable-width LTR double-click selects the word accurately under pointer.
        let ltr_input = spawn_root_input(
            &mut app,
            cam_a,
            300.0,
            42.0,
            UTextInput::new("").with_value("iiiii WWWW").with_font_size(20.0),
            latin_font,
        );
        app.update();
        app.update();

        let ltr_text_entity = app
            .world()
            .get::<UTextInputCaret>(ltr_input)
            .unwrap()
            .text_entity
            .unwrap();
        let ltr_gt = *app.world().get::<GlobalTransform>(ltr_text_entity).unwrap();
        let ltr_cs = *app.world().get::<ComputedSize>(ltr_text_entity).unwrap();
        // Click near center of right half ("WWWW") twice rapidly to double-click.
        let wwww_world = ltr_gt.translation() + Vec3::new(ltr_cs.width * 0.25, 0.0, 0.0);
        let wwww_screen = camera.world_to_viewport(&cam_gt, wwww_world).unwrap();
        for _ in 0..2 {
            app.world_mut().trigger(PointerPress {
                entity: ltr_input,
                pointer: Pointer::new(
                    PointerId::Mouse,
                    Location {
                        target: NormalizedRenderTarget::None {
                            width: 800,
                            height: 600,
                        },
                        position: wwww_screen,
                    },
                ),
                button: PointerButton::Primary,
                hit: HitData::new(cam_a, 1.0, Some(wwww_world), Some(Vec3::Z)),
                count: 1,
            });
            app.update();
        }
        let ltr_sel = app
            .world()
            .get::<UTextInput>(ltr_input)
            .unwrap()
            .selection
            .expect("double click should select word");
        let ltr_val = &app.world().get::<UTextInput>(ltr_input).unwrap().value;
        assert_eq!(&ltr_val[ltr_sel.0..ltr_sel.1], "WWWW");
    }

    #[test]
    fn horizontal_viewport_scroll_keeps_caret_inside_clipped_box() {
        let (mut app, win_a, _, cam_a, latin_font, _) = build_full_text_input_test_app();
        let long_text = "0123456789abcdefghijklmnopqrstuvwxyz_long_overflow_string";
        let input_entity = spawn_root_input(
            &mut app,
            cam_a,
            140.0,
            42.0,
            UTextInput::new("").with_value(long_text).with_font_size(18.0),
            latin_font,
        );
        app.update();
        app.update();

        let caret = app.world().get::<UTextInputCaret>(input_entity).unwrap();
        let vp_w = app
            .world()
            .get::<ComputedSize>(caret.viewport_entity.unwrap())
            .unwrap()
            .width;
        assert!(
            caret.scroll_offset.x > 0.0,
            "expected positive horizontal scroll offset for overflowing text"
        );
        let caret_left = match app
            .world()
            .get::<USelf>(caret.caret_entity.unwrap())
            .unwrap()
            .left
        {
            UVal::Px(v) => v,
            other => panic!("expected Px caret left, got {other:?}"),
        };
        assert!(
            caret_left >= 0.0 && caret_left <= vp_w,
            "caret_left={caret_left} should stay clamped within viewport width 0..{vp_w}"
        );

        // Press Home -> scroll resets back to 0.0.
        app.world_mut().resource_scope(|world, mut focus_state: Mut<UFocusState>| {
            let mut cmds = world.commands();
            set_focus(
                &mut cmds,
                &mut focus_state,
                Some(input_entity),
                FocusModality::Keyboard,
            );
        });
        app.update();
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Home,
            logical_key: Key::Home,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: win_a,
        });
        app.update();
        app.update();

        let caret_after = app.world().get::<UTextInputCaret>(input_entity).unwrap();
        assert_eq!(caret_after.scroll_offset.x, 0.0);
    }

    #[test]
    fn multiline_enter_navigation_scroll_and_selection_highlight_pool() {
        let (mut app, win_a, _, cam_a, latin_font, _) = build_full_text_input_test_app();
        let input_entity = spawn_root_input(
            &mut app,
            cam_a,
            220.0,
            56.0,
            UTextInput::new("Notes")
                .with_multiline(true)
                .with_value("line 1\nline 2\nline 3\nline 4")
                .with_font_size(16.0),
            latin_font,
        );
        app.world_mut().resource_scope(|world, mut focus_state: Mut<UFocusState>| {
            let mut cmds = world.commands();
            set_focus(
                &mut cmds,
                &mut focus_state,
                Some(input_entity),
                FocusModality::Keyboard,
            );
        });
        app.update();
        app.update();

        // Caret at end of line 4 overflows 56px height -> vertical scroll > 0.
        let caret_initial = app.world().get::<UTextInputCaret>(input_entity).unwrap();
        assert!(
            caret_initial.scroll_offset.y > 0.0,
            "expected positive vertical scroll_offset.y for 4 lines in 56px box, got {}",
            caret_initial.scroll_offset.y
        );

        // Enter inserts `\n` without firing UTextInputSubmit.
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: Key::Enter,
            state: ButtonState::Pressed,
            text: Some("\n".into()),
            repeat: false,
            window: win_a,
        });
        app.update();
        assert!(app.world().resource::<RecordedTextEvents>().submitted.is_empty());
        assert!(
            app.world()
                .get::<UTextInput>(input_entity)
                .unwrap()
                .value
                .ends_with('\n')
        );

        // ArrowUp moves caret up from blank 5th line back onto line 4.
        app.update();
        let len_with_nl = app.world().get::<UTextInput>(input_entity).unwrap().value.len();
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::ArrowUp,
            logical_key: Key::ArrowUp,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: win_a,
        });
        app.update();
        let moved_cursor = app.world().get::<UTextInput>(input_entity).unwrap().cursor_position;
        assert!(moved_cursor < len_with_nl);

        // Ctrl+A selects all lines and spawns pooled extra selection quads.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character("a".into()),
            state: ButtonState::Pressed,
            text: Some("a".into()),
            repeat: false,
            window: win_a,
        });
        app.update();
        app.update();
        assert!(
            !app.world()
                .get::<UTextInputCaret>(input_entity)
                .unwrap()
                .extra_selection_entities
                .is_empty(),
            "multiline select-all must populate extra_selection_entities"
        );

        // Ctrl+Enter submits multiline input.
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: Key::Enter,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: win_a,
        });
        app.update();
        assert_eq!(app.world().resource::<RecordedTextEvents>().submitted.len(), 1);
    }
}
