//! Builder and spawning helpers for assembling composite text input widgets.

use bevy::prelude::*;
use bevy::text::LineBreak;
use luma_engine::layout::{
    geometry::{UCornerRadius, USides, UVal},
    text::{UTextLabel, UTextOverflow, UTextWrapMode},
    univis_node::{
        UAlignItems, UBorder, UClip, UDisplay, UFlexDirection, ULayout, UNode, UPositionType,
        USelf, UZIndex,
    },
};

use super::types::{
    UTextInput, UTextInputCaret, UTextInputCaretMarker, UTextInputSelectionMarker,
    UTextInputTextMarker, UTextInputViewportMarker,
};
use crate::interaction::{
    feedback::UInteraction,
    focus::{UFocusVisual, UFocusable},
};

/// Spawns a complete text input widget with interactive styling, focus visual rings,
/// clipped viewport container, text label, selection highlight, and animated caret cursor.
pub fn spawn_text_input(commands: &mut Commands, input: UTextInput) -> Entity {
    let font_size = input.font_size;
    let is_multiline = input.multiline;
    let initial_text = if input.value.is_empty() {
        input.placeholder.clone()
    } else {
        super::actions::display_text(&input)
    };
    let initial_color = if input.value.is_empty() {
        input.placeholder_color
    } else {
        input.text_color
    };

    // 1. Spawn parent container
    let parent = commands
        .spawn((
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(if is_multiline { 120.0 } else { 42.0 }),
                padding: USides::axes(14.0, 6.0),
                background_color: Color::srgba(0.08, 0.1, 0.14, 0.95),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: if is_multiline {
                    UAlignItems::Start
                } else {
                    UAlignItems::Center
                },
                ..default()
            },
            UBorder {
                color: Color::srgba(0.2, 0.25, 0.35, 0.8),
                width: 1.5,
                radius: UCornerRadius::all(6.0),
                offset: 0.0,
            },
            UFocusable::new(),
            UFocusVisual::border(Color::srgb(0.0, 0.9, 1.0), 2.0),
            UInteraction::default(),
            input,
        ))
        .id();

    // 2. Spawn clipped inner viewport container respecting parent padding
    let viewport = commands
        .spawn((
            ChildOf(parent),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
            UClip::enabled(true),
            UTextInputViewportMarker,
        ))
        .id();

    // 3. Spawn selection highlight child (rendered behind text)
    let selection_child = commands
        .spawn((
            ChildOf(viewport),
            UNode {
                width: UVal::Px(0.0),
                height: UVal::Px(font_size * 1.2),
                background_color: Color::srgba(0.0, 0.45, 0.9, 0.35),
                border_radius: UCornerRadius::all(2.0),
                ..default()
            },
            USelf {
                position_type: UPositionType::Absolute,
                left: UVal::Px(0.0),
                top: UVal::Px(0.0),
                ..default()
            },
            UZIndex::Local(-1),
            Visibility::Hidden,
            UTextInputSelectionMarker,
        ))
        .id();

    // 4. Spawn text child inside clipped viewport
    let text_child = commands
        .spawn((
            ChildOf(viewport),
            UNode {
                width: if is_multiline {
                    UVal::Percent(1.0)
                } else {
                    UVal::Auto
                },
                ..default()
            },
            USelf {
                position_type: UPositionType::Absolute,
                left: UVal::Px(0.0),
                top: UVal::Px(0.0),
                ..default()
            },
            UTextLabel {
                text: initial_text,
                font_size,
                color: initial_color,
                linebreak: if is_multiline {
                    LineBreak::WordBoundary
                } else {
                    LineBreak::NoWrap
                },
                wrap_mode: if is_multiline {
                    UTextWrapMode::WrapToParent
                } else {
                    UTextWrapMode::SingleLine
                },
                autosize: true,
                overflow: UTextOverflow::Clip,
                ..default()
            },
            UTextInputTextMarker,
        ))
        .id();

    // 5. Spawn caret child
    let caret_child = commands
        .spawn((
            ChildOf(viewport),
            UNode {
                width: UVal::Px(2.0),
                height: UVal::Px(font_size * 1.2),
                background_color: Color::srgb(0.0, 0.9, 1.0),
                ..default()
            },
            USelf {
                position_type: UPositionType::Absolute,
                left: UVal::Px(0.0),
                top: UVal::Px(0.0),
                ..default()
            },
            UZIndex::Local(1),
            Visibility::Hidden,
            UTextInputCaretMarker,
        ))
        .id();

    // 6. Attach caret state linking children
    let caret = UTextInputCaret {
        viewport_entity: Some(viewport),
        text_entity: Some(text_child),
        caret_entity: Some(caret_child),
        selection_entity: Some(selection_child),
        ..default()
    };

    if let Ok(mut cmd) = commands.get_entity(parent) {
        cmd.try_insert(caret);
    }

    parent
}
