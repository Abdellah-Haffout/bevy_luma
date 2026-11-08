//! Visual focus states and default observers for Univis focus navigation.

use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use luma_engine::layout::{
    transition::UTransitionState,
    univis_node::{UBorder, UNode},
    visual::UShadow,
};

use super::types::{
    FocusGained, FocusLost, FocusModality, UFocusActivate, UFocusDeactivate, UFocusState,
    UFocusable, set_focus,
};
use crate::interaction::feedback::{UInteraction, UInteractionColors, UInteractionScale};

/// Declarative focus styling applied when an entity gains focus and restored when focus is lost.
#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct UFocusVisual {
    /// Border color applied when focused.
    pub focused_border_color: Option<Color>,
    /// Border width applied when focused.
    pub focused_border_width: Option<f32>,
    /// Resting border offset applied when focused (default: 0.0).
    pub focused_border_offset: f32,
    /// Initial expanded offset when gaining focus, which snaps/contracts inward to `focused_border_offset`.
    pub contract_offset: Option<f32>,
    /// Inward contraction speed (higher = snappier, default: 28.0).
    pub contract_speed: f32,
    /// Whether gaining focus also triggers hover interaction styling (`UInteraction::Hovered` & `colors.hovered`).
    pub as_hover: bool,
    /// Whether the focus ring gently pulses while focused.
    pub pulse: bool,
    /// Pulse animation speed.
    pub pulse_speed: f32,
    /// Background color applied when focused.
    pub focused_background_color: Option<Color>,
    /// Scale factor applied when focused (modifies [`UTransitionState`]).
    pub focused_scale: Option<f32>,
    /// Optional focus glow or shadow applied when focused.
    pub focused_shadow: Option<UShadow>,
    /// Saved original border color before focus was gained.
    #[reflect(ignore)]
    pub(crate) original_border_color: Option<Color>,
    /// Saved original border width before focus was gained.
    #[reflect(ignore)]
    pub(crate) original_border_width: Option<f32>,
    /// Saved original border offset before focus was gained.
    #[reflect(ignore)]
    pub(crate) original_border_offset: Option<f32>,
    /// Saved original background color before focus was gained.
    #[reflect(ignore)]
    pub(crate) original_background_color: Option<Color>,
    /// Saved original shadow before focus was gained.
    #[reflect(ignore)]
    pub(crate) original_shadow: Option<Option<UShadow>>,
    /// Saved original scale before focus was gained.
    #[reflect(ignore)]
    pub(crate) original_scale: Option<f32>,
    /// Whether this entity already had a UBorder component before focus.
    #[reflect(ignore)]
    pub(crate) had_border_component: bool,
    /// Whether original values have been captured.
    #[reflect(ignore)]
    pub(crate) captured: bool,
}

/// Runtime animation state driving the inward contraction and dynamic thickness of the focus indicator.
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct UFocusMotionState {
    /// Current animated border offset.
    pub current_offset: f32,
    /// Target resting border offset.
    pub target_offset: f32,
    /// Current animated border width.
    pub current_width: f32,
    /// Target resting border width.
    pub target_width: f32,
    /// Rate of contraction/expansion towards target values.
    pub speed: f32,
    /// Whether pulse animation is active.
    pub pulse: bool,
    /// Accumulated time for pulse animation.
    pub pulse_timer: f32,
    /// Pulse frequency speed.
    pub pulse_speed: f32,
}

impl Default for UFocusMotionState {
    fn default() -> Self {
        Self {
            current_offset: 0.0,
            target_offset: 0.0,
            current_width: 0.0,
            target_width: 2.0,
            speed: 28.0,
            pulse: false,
            pulse_timer: 0.0,
            pulse_speed: 3.0,
        }
    }
}

impl Default for UFocusVisual {
    fn default() -> Self {
        Self {
            focused_border_color: Some(Color::srgb(0.3, 0.7, 1.0)),
            focused_border_width: Some(2.5),
            focused_border_offset: 0.0,
            contract_offset: None,
            contract_speed: 28.0,
            as_hover: false,
            pulse: false,
            pulse_speed: 3.0,
            focused_background_color: None,
            focused_scale: Some(1.05),
            focused_shadow: None,
            original_border_color: None,
            original_border_width: None,
            original_border_offset: None,
            original_background_color: None,
            original_shadow: None,
            original_scale: None,
            had_border_component: false,
            captured: false,
        }
    }
}

impl UFocusVisual {
    /// Creates a focus visual with a specific border color and width.
    pub fn border(color: Color, width: f32) -> Self {
        Self {
            focused_border_color: Some(color),
            focused_border_width: Some(width),
            focused_border_offset: 0.0,
            contract_offset: None,
            focused_background_color: None,
            focused_scale: None,
            ..Default::default()
        }
    }

    /// Creates an animated focus ring that contracts inward onto the focused element.
    pub fn ring(color: Color) -> Self {
        Self {
            focused_border_color: Some(color),
            focused_border_width: Some(2.5),
            focused_border_offset: 0.0,
            contract_offset: Some(6.0),
            contract_speed: 28.0,
            as_hover: false,
            pulse: false,
            focused_background_color: None,
            focused_scale: None,
            ..Default::default()
        }
    }

    /// Creates a focus visual with a standard outline (2.0px).
    pub fn outline(color: Color) -> Self {
        Self::border(color, 2.0)
    }

    /// Creates a focus visual with spring scaling zoom.
    pub fn scale(scale: f32) -> Self {
        Self {
            focused_border_color: None,
            focused_border_width: None,
            focused_background_color: None,
            focused_scale: Some(scale),
            ..Default::default()
        }
    }

    /// Creates a focus visual combining an accent border and subtle zoom.
    pub fn glow(color: Color) -> Self {
        Self {
            focused_border_color: Some(color),
            focused_border_width: Some(2.5),
            focused_border_offset: 0.0,
            contract_offset: Some(5.0),
            focused_background_color: None,
            focused_scale: Some(1.06),
            ..Default::default()
        }
    }

    /// Creates a focus visual that alters background color.
    pub fn background(color: Color) -> Self {
        Self {
            focused_border_color: None,
            focused_border_width: None,
            focused_background_color: Some(color),
            focused_scale: None,
            ..Default::default()
        }
    }

    /// Sets both focused border color and width.
    pub fn with_border(mut self, color: Color, width: f32) -> Self {
        self.focused_border_color = Some(color);
        self.focused_border_width = Some(width);
        self
    }

    /// Sets the focused border color.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.focused_border_color = Some(color);
        self
    }

    /// Sets the focused border width (thickness).
    pub fn with_width(mut self, width: f32) -> Self {
        self.focused_border_width = Some(width);
        self
    }

    /// Sets the contracting motion parameters (inward snap animation).
    /// `start_offset`: the distance outside the element where the ring appears (e.g. 6.0).
    /// `target_offset`: the resting offset against the element (e.g. 0.0).
    pub fn with_contract(mut self, start_offset: f32, target_offset: f32) -> Self {
        self.contract_offset = Some(start_offset);
        self.focused_border_offset = target_offset;
        self
    }

    /// Sets the speed of the contraction animation.
    pub fn with_contract_speed(mut self, speed: f32) -> Self {
        self.contract_speed = speed;
        self
    }

    /// Enables or disables "focus as hover" behavior: when focused, the element also applies
    /// `UInteraction::Hovered` and `UInteractionColors::hovered`.
    pub fn with_as_hover(mut self, as_hover: bool) -> Self {
        self.as_hover = as_hover;
        self
    }

    /// Enables or disables gentle breathing/pulsing of the focus ring while focused.
    pub fn with_pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    /// Sets the pulse animation frequency speed.
    pub fn with_pulse_speed(mut self, speed: f32) -> Self {
        self.pulse_speed = speed;
        self
    }

    /// Sets the focused scale factor.
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.focused_scale = Some(scale);
        self
    }

    /// Sets the focused background color.
    pub fn with_background(mut self, color: Color) -> Self {
        self.focused_background_color = Some(color);
        self
    }

    /// Sets a custom focus shadow (or drop shadow) when focused.
    pub fn with_shadow(mut self, shadow: UShadow) -> Self {
        self.focused_shadow = Some(shadow);
        self
    }

    /// Sets a symmetrical neon glow halo when focused.
    pub fn with_glow(mut self, color: Color, blur: f32) -> Self {
        self.focused_shadow = Some(UShadow::glow(color, blur));
        self
    }

    /// Returns whether this focus visual has captured original styling.
    pub fn is_captured(&self) -> bool {
        self.captured
    }

    /// Sets the original border color to restore when focus is lost.
    pub fn set_original_border_color(&mut self, color: Color) {
        self.original_border_color = Some(color);
    }

    /// Sets the original background color to restore when focus is lost.
    pub fn set_original_background_color(&mut self, color: Color) {
        self.original_background_color = Some(color);
    }
}

/// Observer updating visual state when an entity gains focus.
pub fn on_focus_gained_visual(
    trigger: On<FocusGained>,
    mut commands: Commands,
    mut query: Query<(
        Option<&mut UNode>,
        Option<&mut UBorder>,
        &mut UFocusVisual,
        Option<&mut UTransitionState>,
        Option<&mut UInteraction>,
        Option<&UInteractionColors>,
        Option<&Transform>,
        Option<&mut UShadow>,
    )>,
) {
    let entity = trigger.entity;
    if let Ok((
        mut node_opt,
        mut border_opt,
        mut visual,
        mut transition_opt,
        mut interaction_opt,
        colors_opt,
        transform_opt,
        mut shadow_opt,
    )) = query.get_mut(entity)
    {
        if !visual.captured {
            if visual.focused_background_color.is_some() {
                if let Some(ref node) = node_opt {
                    visual.original_background_color = Some(node.background_color);
                }
            } else {
                visual.original_background_color = None;
            }

            if let Some(ref border) = border_opt {
                if visual.focused_border_color.is_some() {
                    visual.original_border_color = Some(border.color);
                }
                if visual.focused_border_width.is_some() {
                    visual.original_border_width = Some(border.width);
                }
                visual.original_border_offset = Some(border.offset);
                visual.had_border_component = true;
            } else {
                visual.had_border_component = false;
            }

            if visual.focused_shadow.is_some() {
                visual.original_shadow = Some(shadow_opt.as_deref().copied());
            }

            visual.captured = true;
        }

        // 1. Focus as hover
        if visual.as_hover {
            if let Some(ref mut interaction) = interaction_opt {
                **interaction = UInteraction::Hovered;
            }
            if let (Some(ref mut node), Some(colors)) = (node_opt.as_deref_mut(), colors_opt) {
                if node.background_color != colors.hovered {
                    node.background_color = colors.hovered;
                }
            }
        }

        // 2. Apply background color only if explicitly specified
        if let (Some(ref mut node), Some(bg)) =
            (node_opt.as_deref_mut(), visual.focused_background_color)
        {
            if node.background_color != bg {
                node.background_color = bg;
            }
        }

        // 3. Apply border & contracting motion
        let target_width = visual.focused_border_width.unwrap_or(2.5);
        let target_offset = visual.focused_border_offset;
        let contract_start = visual.contract_offset;

        if visual.focused_border_color.is_some() || visual.focused_border_width.is_some() {
            let color = visual.focused_border_color.unwrap_or(Color::WHITE);
            let initial_offset = contract_start.unwrap_or(target_offset);
            // Inward snap: start with slightly thicker width and contract down
            let initial_width = if let Some(offset) = contract_start {
                target_width + (offset * 0.4).clamp(1.5, 4.0)
            } else {
                target_width
            };

            if let Some(ref mut border) = border_opt {
                border.color = color;
                border.offset = initial_offset;
                border.width = initial_width;
            } else if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_insert(UBorder {
                    color,
                    width: initial_width,
                    offset: initial_offset,
                    ..Default::default()
                });
            }

            if contract_start.is_some() || visual.pulse {
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.try_insert(UFocusMotionState {
                        current_offset: initial_offset,
                        target_offset,
                        current_width: initial_width,
                        target_width,
                        speed: visual.contract_speed,
                        pulse: visual.pulse,
                        pulse_timer: 0.0,
                        pulse_speed: visual.pulse_speed,
                    });
                }
            }
        }

        // 4. Apply focus shadow or neon glow
        if let Some(glow) = visual.focused_shadow {
            if let Some(ref mut shadow) = shadow_opt {
                **shadow = glow;
            } else if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_insert(glow);
            }
        }

        // 5. Apply spring scale zoom
        if let Some(scale) = visual.focused_scale {
            if visual.original_scale.is_none() {
                let cur_scale = transform_opt.as_ref().map(|t| t.scale.x).unwrap_or(1.0);
                visual.original_scale = Some(if (cur_scale - 1.0).abs() < 0.05 { cur_scale } else { 1.0 });
            }

            if let Some(ref mut state) = transition_opt {
                state.target_scale = Vec2::splat(scale);
            } else if let Ok(mut cmd) = commands.get_entity(entity) {
                let cur_pos = transform_opt.as_ref().map(|t| t.translation.xy()).unwrap_or(Vec2::ZERO);
                cmd.try_insert((
                    luma_engine::layout::transition::UTransition::bouncy(240.0, 0.85),
                    UTransitionState {
                        target_translation: cur_pos,
                        target_scale: Vec2::splat(scale),
                        initialized: true,
                        ..Default::default()
                    },
                ));
            }
        }
    }
}

/// Observer restoring original styling when an entity loses focus.
pub fn on_focus_lost_visual(
    trigger: On<FocusLost>,
    mut commands: Commands,
    mut query: Query<(
        Option<&mut UNode>,
        Option<&mut UBorder>,
        &mut UFocusVisual,
        Option<&mut UTransitionState>,
        Option<&mut UInteraction>,
        Option<&UInteractionColors>,
        Option<&UInteractionScale>,
        Option<&mut Transform>,
        Option<&mut UShadow>,
    )>,
) {
    let entity = trigger.entity;
    if let Ok((
        mut node_opt,
        mut border_opt,
        mut visual,
        mut transition_opt,
        mut interaction_opt,
        colors_opt,
        scale_opt,
        mut transform_opt,
        mut shadow_opt,
    )) = query.get_mut(entity)
        && visual.captured
    {
        // 0. Remove runtime motion state
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.try_remove::<UFocusMotionState>();
        }

        // 1. Restore focus as hover
        if visual.as_hover {
            if let Some(ref mut interaction) = interaction_opt {
                if **interaction == UInteraction::Hovered {
                    **interaction = UInteraction::Normal;
                }
            }
            if let (Some(ref mut node), Some(colors)) = (node_opt.as_deref_mut(), colors_opt) {
                if node.background_color != colors.normal {
                    node.background_color = colors.normal;
                }
            }
        }

        // 2. Restore background color ONLY if this focus visual modified it!
        if visual.focused_background_color.is_some() {
            if let Some(ref mut node) = node_opt {
                if let (Some(colors), Some(interaction)) = (colors_opt, interaction_opt.as_deref())
                {
                    node.background_color = match *interaction {
                        UInteraction::Hovered => colors.hovered,
                        UInteraction::Pressed | UInteraction::Clicked => colors.pressed,
                        _ => colors.normal,
                    };
                } else if let Some(bg) = visual.original_background_color {
                    node.background_color = bg;
                }
            }
        }
        visual.original_background_color = None;

        // 3. Reset Clicked / Pressed interaction state on focus transition
        if let Some(ref mut interaction) = interaction_opt {
            if **interaction == UInteraction::Clicked || **interaction == UInteraction::Pressed {
                **interaction = UInteraction::Normal;
                if let (Some(ref mut node), Some(colors)) = (node_opt.as_deref_mut(), colors_opt) {
                    node.background_color = colors.normal;
                }
                if let (Some(scale), Some(ref mut state)) =
                    (scale_opt, transition_opt.as_deref_mut())
                {
                    state.target_scale = Vec2::splat(scale.normal);
                }
            }
        }

        // 4. Restore or clean up border immediately
        if visual.had_border_component {
            if let Some(ref mut border) = border_opt {
                if let Some(color) = visual.original_border_color.take() {
                    border.color = color;
                }
                if let Some(width) = visual.original_border_width.take() {
                    border.width = width;
                }
                if let Some(offset) = visual.original_border_offset.take() {
                    border.offset = offset;
                }
            }
        } else if let Some(ref mut border) = border_opt {
            // Immediate zeroing so that visual disappears on this exact frame!
            border.width = 0.0;
            border.color = Color::NONE;
            border.offset = 0.0;
            // Remove border if it was added only for focus
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_remove::<UBorder>();
            }
        }

        // 5. Restore focus shadow / glow
        if let Some(orig_shadow_opt) = visual.original_shadow.take() {
            if let Some(orig_shadow) = orig_shadow_opt {
                if let Some(ref mut shadow) = shadow_opt {
                    **shadow = orig_shadow;
                }
            } else if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_remove::<UShadow>();
            }
        }

        // 6. Restore scale back to original
        let orig_scale = visual.original_scale.take().unwrap_or(1.0);
        if visual.focused_scale.is_some() {
            if let Some(ref mut state) = transition_opt {
                state.target_scale = Vec2::splat(orig_scale);
            } else if let Some(ref mut transform) = transform_opt {
                transform.scale = Vec3::splat(orig_scale);
            }
        }

        visual.captured = false;
    }
}

/// Advances the contracting focus ring and dynamic border animations.
pub fn focus_motion_system(
    time: Option<Res<Time>>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut UBorder, &mut UFocusMotionState)>,
) {
    let Some(time) = time else {
        return;
    };
    let dt = time.delta_secs().min(0.05);
    if dt <= 0.0 {
        return;
    }

    for (entity, mut border, mut state) in query.iter_mut() {
        let diff_offset = state.target_offset - state.current_offset;
        let diff_width = state.target_width - state.current_width;

        let snap_threshold = 0.03;
        let offset_settled = diff_offset.abs() < snap_threshold;
        let width_settled = diff_width.abs() < snap_threshold;

        if !offset_settled {
            let step = diff_offset * (state.speed * dt).min(1.0);
            state.current_offset += step;
            border.offset = state.current_offset;
        } else if border.offset != state.target_offset {
            state.current_offset = state.target_offset;
            border.offset = state.target_offset;
        }

        if !width_settled {
            let step = diff_width * (state.speed * dt).min(1.0);
            state.current_width += step;
            border.width = state.current_width;
        } else if !state.pulse && border.width != state.target_width {
            state.current_width = state.target_width;
            border.width = state.target_width;
        }

        if state.pulse {
            state.pulse_timer += dt * state.pulse_speed;
            let pulse_wave = (state.pulse_timer * core::f32::consts::PI).sin() * 0.5 + 0.5;
            let pulsed_w = state.target_width + pulse_wave * 0.75;
            if (border.width - pulsed_w).abs() > 0.01 {
                border.width = pulsed_w;
            }
        } else if offset_settled && width_settled {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_remove::<UFocusMotionState>();
            }
        }
    }
}

/// Observer setting focus when clicking directly on a focusable node.
pub fn on_pointer_focus_click(
    trigger: On<PointerClick>,
    mut commands: Commands,
    mut focus_state: Option<ResMut<UFocusState>>,
    query: Query<&UFocusable>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }
    let entity = trigger.entity;
    if let Ok(focusable) = query.get(entity)
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
    }
}

/// Default observer for [`UFocusActivate`], updating interaction state and visual feedback.
pub fn on_focus_activate_default(
    trigger: On<UFocusActivate>,
    mut query: Query<(
        &mut UInteraction,
        Option<&mut UNode>,
        Option<&UInteractionColors>,
        Option<&UInteractionScale>,
        Option<&mut UTransitionState>,
    )>,
) {
    let entity = trigger.entity;
    if let Ok((mut interaction, mut node, colors, scale, mut transition_state)) =
        query.get_mut(entity)
    {
        *interaction = UInteraction::Clicked;
        if let (Some(ref mut node), Some(color)) = (node.as_deref_mut(), colors) {
            node.background_color = color.pressed;
        }
        if let Some(scale) = scale
            && let Some(ref mut state) = transition_state
        {
            state.target_scale = Vec2::splat(scale.pressed);
        }
    }
}

/// Default observer for [`UFocusDeactivate`], restoring interaction state when activation input is released.
pub fn on_focus_deactivate_default(
    trigger: On<UFocusDeactivate>,
    mut query: Query<(
        &mut UInteraction,
        Option<&mut UNode>,
        Option<&UInteractionColors>,
        Option<&UInteractionScale>,
        Option<&mut UTransitionState>,
    )>,
) {
    let entity = trigger.entity;
    if let Ok((mut interaction, mut node, colors, scale, mut transition_state)) =
        query.get_mut(entity)
    {
        *interaction = UInteraction::Normal;
        if let (Some(ref mut node), Some(color)) = (node.as_deref_mut(), colors) {
            node.background_color = color.normal;
        }
        if let Some(scale) = scale
            && let Some(ref mut state) = transition_state
        {
            state.target_scale = Vec2::splat(scale.normal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_visual_constructors() {
        let outline = UFocusVisual::outline(Color::srgb(1.0, 0.0, 0.0));
        assert_eq!(
            outline.focused_border_color,
            Some(Color::srgb(1.0, 0.0, 0.0))
        );
        assert_eq!(outline.focused_border_width, Some(2.0));

        let glow = UFocusVisual::glow(Color::srgb(0.0, 1.0, 0.0));
        assert_eq!(glow.focused_scale, Some(1.06));

        let scale = UFocusVisual::scale(1.15);
        assert_eq!(scale.focused_scale, Some(1.15));
    }

    #[test]
    fn test_focus_lost_does_not_corrupt_background_when_not_focused_bg() {
        let mut app = App::new();
        app.add_observer(on_focus_gained_visual);
        app.add_observer(on_focus_lost_visual);

        let initial_bg = Color::srgb(0.1, 0.2, 0.3);
        let entity = app
            .world_mut()
            .spawn((
                UNode {
                    background_color: initial_bg,
                    ..Default::default()
                },
                UFocusVisual::border(Color::srgb(0.35, 0.65, 1.0), 2.0),
            ))
            .id();

        // 1. Gain focus
        app.world_mut()
            .commands()
            .entity(entity)
            .trigger(|entity| FocusGained { entity });
        app.update();

        let node = app.world().get::<UNode>(entity).unwrap();
        assert_eq!(node.background_color, initial_bg);

        // 2. Widget modifies background color while focused (e.g. toggle, checkbox, selection)
        let toggled_bg = Color::srgb(0.8, 0.2, 0.1);
        app.world_mut()
            .get_mut::<UNode>(entity)
            .unwrap()
            .background_color = toggled_bg;

        // 3. Lose focus (focus moves to another widget)
        app.world_mut()
            .commands()
            .entity(entity)
            .trigger(|entity| FocusLost { entity });
        app.update();

        // Verify background is preserved
        let node = app.world().get::<UNode>(entity).unwrap();
        assert_eq!(node.background_color, toggled_bg);
    }

    #[test]
    fn test_focus_lost_resets_activation_background_to_normal() {
        let mut app = App::new();
        app.add_observer(on_focus_gained_visual);
        app.add_observer(on_focus_lost_visual);
        app.add_observer(on_focus_activate_default);

        let normal = Color::srgb(0.2, 0.4, 0.8);
        let pressed = Color::srgb(0.1, 0.2, 0.5);
        let colors = UInteractionColors {
            normal,
            hovered: Color::srgb(0.3, 0.5, 0.9),
            pressed,
        };

        let entity = app
            .world_mut()
            .spawn((
                UNode {
                    background_color: normal,
                    ..Default::default()
                },
                UInteraction::default(),
                colors,
                UFocusVisual::border(Color::srgb(0.35, 0.65, 1.0), 2.0),
            ))
            .id();

        // 1. Gain focus
        app.world_mut()
            .commands()
            .entity(entity)
            .trigger(|entity| FocusGained { entity });
        app.update();

        // 2. Activate with Enter
        app.world_mut()
            .commands()
            .entity(entity)
            .trigger(|entity| UFocusActivate { entity });
        app.update();

        // Node is pressed
        let node = app.world().get::<UNode>(entity).unwrap();
        assert_eq!(node.background_color, pressed);

        // 3. Move focus away (FocusLost)
        app.world_mut()
            .commands()
            .entity(entity)
            .trigger(|entity| FocusLost { entity });
        app.update();

        // Node background is cleanly restored to normal
        let node = app.world().get::<UNode>(entity).unwrap();
        assert_eq!(node.background_color, normal);
        let interaction = app.world().get::<UInteraction>(entity).unwrap();
        assert_eq!(*interaction, UInteraction::Normal);
    }
}
