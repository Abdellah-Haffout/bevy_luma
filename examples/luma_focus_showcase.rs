//! Comprehensive Focus Showcase for Bevy Luma.
//!
//! Demonstrates all focus visual & motion features:
//! - Contracting Focus Ring (Inward Snap Motion from expanded offset)
//! - Focus as Hover (Arrows/Gamepad activate `UInteraction::Hovered` styling)
//! - Dynamic Thickness & Pulse (Breathing focus ring)
//! - Spring Scaling Zoom (Bouncy physics scale)
//! - All Effects Combined (Contracting Ring + Hover Color + Pulse + Spring Zoom)
//! - Background Color Morph
//!
//! Controls:
//! - Arrow Keys / D-pad / Left Stick: 2D Spatial Navigation
//! - Tab / Shift+Tab: Sequential Tab Navigation
//! - Enter / Spacebar / Gamepad South (A/Cross): Activation

use bevy::prelude::*;
use bevy_luma::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - Focus Motion & Visual Showcase".into(),
                resolution: (1000, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_ui)
        .add_systems(Update, update_hud_status)
        .add_observer(on_card_activate)
        .add_observer(on_card_click)
        .run();
}

#[derive(Component, Clone, Default)]
struct StatusText;

#[derive(Component, Clone, Default)]
struct CardInfo {
    title: &'static str,
}

fn setup_ui(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(focus_showcase_scene(&theme));
}

fn focus_showcase_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();
    let icon_font = theme.icon.font.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.04, 0.045, 0.065),
            padding: USides::all(32.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            // --- Header Title ---
            UNode {
                width: UVal::Percent(1.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                align_items: UAlignItems::Center,
                gap: 6.0,
            }
            Children [
                UText {
                    text: { "FOCUS VISUAL & MOTION SHOWCASE".to_string() },
                    font_size: 24.0,
                    font: { font.clone() },
                    color: Color::srgb(0.95, 0.96, 0.98),
                }
                --
                UText {
                    text: { "Navigate with Arrow Keys, Gamepad D-pad/Stick, or Tab • Press Enter or Space to Activate".to_string() },
                    font_size: 13.0,
                    font: { font.clone() },
                    color: Color::srgb(0.55, 0.60, 0.70),
                }
            ]

            --

            // --- 2x3 Grid of Focus Mode Cards ---
            UNode {
                width: UVal::Percent(1.0),
                max_width: 860.0,
            }
            ULayout {
                display: UDisplay::Grid,
                grid_template_columns: vec![
                    UTrackSize::Fr(1.0),
                    UTrackSize::Fr(1.0),
                    UTrackSize::Fr(1.0),
                ],
                grid_template_rows: vec![
                    UTrackSize::Px(120.0),
                    UTrackSize::Px(120.0),
                ],
                gap: 20.0,
            }
            Children [
                // 1. Inward Contracting Focus Ring
                @focus_card(
                    "Snap & Contract",
                    "Starts +8px outside and smoothly snaps inward to 0px",
                    Icon::TARGET.to_string(),
                    Color::srgb(0.16, 0.71, 0.96),
                    UFocusVisual::ring(Color::srgb(0.16, 0.71, 0.96))
                        .with_contract(8.0, 0.0)
                        .with_width(2.5)
                        .with_contract_speed(28.0),
                    font.clone(),
                    icon_font.clone(),
                )

                --

                // 2. Focus As Hover
                @focus_card(
                    "Focus As Hover",
                    "Applies hover background color when navigated via arrows",
                    Icon::COMPASS.to_string(),
                    Color::srgb(1.0, 0.79, 0.16),
                    UFocusVisual::ring(Color::srgb(1.0, 0.79, 0.16))
                        .with_contract(6.0, 0.0)
                        .with_width(2.5)
                        .with_as_hover(true),
                    font.clone(),
                    icon_font.clone(),
                )

                --

                // 3. Gentle Breathing Pulse
                @focus_card(
                    "Breathing Pulse",
                    "Gentle animated sine wave pulse on the border thickness",
                    Icon::ACTIVITY.to_string(),
                    Color::srgb(1.0, 0.25, 0.51),
                    UFocusVisual::ring(Color::srgb(1.0, 0.25, 0.51))
                        .with_contract(6.0, 0.0)
                        .with_width(2.5)
                        .with_pulse(true)
                        .with_pulse_speed(3.5)
                        .with_glow(Color::srgba(1.0, 0.25, 0.51, 0.40), 14.0),
                    font.clone(),
                    icon_font.clone(),
                )

                --

                // 4. Spring Scaling Zoom
                @focus_card(
                    "Spring Scaling Zoom",
                    "Snappy spring scale zoom (1.10x) with no border",
                    Icon::ZOOM_IN.to_string(),
                    Color::srgb(0.67, 0.28, 0.74),
                    UFocusVisual::scale(1.10)
                        .with_border(Color::srgba(0.67, 0.28, 0.74, 0.55), 1.5)
                        .with_shadow(UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.65), Vec2::new(0.0, -10.0), 20.0)),
                    font.clone(),
                    icon_font.clone(),
                )

                --

                // 5. Ultimate Focus Combo
                @focus_card(
                    "Ultimate Combo",
                    "Contracting Ring + Hover Color + Bouncy Scale + Pulse",
                    Icon::CHECK.to_string(),
                    Color::srgb(0.0, 0.90, 0.46),
                    UFocusVisual::ring(Color::srgb(0.0, 0.90, 0.46))
                        .with_contract(9.0, 0.0)
                        .with_width(3.0)
                        .with_as_hover(true)
                        .with_pulse(true)
                        .with_scale(1.08)
                        .with_glow(Color::srgba(0.0, 0.90, 0.46, 0.45), 16.0),
                    font.clone(),
                    icon_font.clone(),
                )

                --

                // 6. Background Color Morph
                @focus_card(
                    "Background Shift",
                    "Directly morphs background color without extra border",
                    Icon::HEXAGON.to_string(),
                    Color::srgb(1.0, 0.44, 0.26),
                    UFocusVisual::background(Color::srgb(0.23, 0.13, 0.30))
                        .with_border(Color::srgb(0.48, 0.26, 0.59), 1.2)
                        .with_glow(Color::srgba(0.48, 0.26, 0.59, 0.35), 16.0),
                    font.clone(),
                    icon_font.clone(),
                )
            ]

            --

            // --- Footer HUD ---
            UNode {
                width: UVal::Px(640.0),
                background_color: Color::srgb(0.065, 0.075, 0.10),
                border_radius: UCornerRadius::all(12.0),
                padding: USides::axes(24.0, 14.0),
            }
            UBorder {
                color: Color::srgb(0.12, 0.14, 0.19),
                width: 1.0,
                radius: UCornerRadius::all(12.0),
            }
            UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.4), Vec2::new(0.0, -4.0), 12.0)
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
            }
            Children [
                UText {
                    text: { "ACTIVE FOCUS:".to_string() },
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.45, 0.50, 0.60),
                }
                --
                StatusText
                UText {
                    text: { "Press an Arrow Key or Tab to start".to_string() },
                    font_size: 13.0,
                    font: { font.clone() },
                    color: Color::srgb(0.90, 0.92, 0.96),
                }
            ]
        ]
    }
}

/// Helper building an interactive focusable card.
fn focus_card(
    title: &'static str,
    subtitle: &'static str,
    icon: String,
    accent_color: Color,
    focus_visual: UFocusVisual,
    font: Handle<Font>,
    icon_font: Handle<Font>,
) -> impl Scene {
    let base_bg = Color::srgb(0.07, 0.08, 0.10);
    let hover_bg = Color::srgb(0.10, 0.115, 0.15);
    let press_bg = Color::srgb(0.05, 0.06, 0.08);

    bsn! {
        CardInfo { title }
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(120.0),
            background_color: base_bg,
            border_radius: UCornerRadius::all(14.0),
            padding: USides::all(16.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.07),
            width: 1.0,
            radius: UCornerRadius::all(14.0),
        }
        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.45), Vec2::new(0.0, -6.0), 12.0)
        UTransition::bouncy(240.0, 0.85)
        UFocusable::new()
        focus_visual
        UInteraction::default()
        UInteractionColors {
            normal: base_bg,
            hovered: hover_bg,
            pressed: press_bg,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            // Top row with Icon and Title
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 12.0,
            }
            Children [
                UNode {
                    width: UVal::Px(36.0),
                    height: UVal::Px(36.0),
                    background_color: Color::srgba(accent_color.to_srgba().red, accent_color.to_srgba().green, accent_color.to_srgba().blue, 0.15),
                    border_radius: UCornerRadius::all(9.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: icon,
                        font_size: 18.0,
                        font: { icon_font.clone() },
                        color: accent_color,
                    }
                ]
                --
                UText {
                    text: { title.to_string() },
                    font_size: 15.0,
                    font: { font.clone() },
                    color: Color::WHITE,
                }
            ]
            --
            // Subtitle description
            UText {
                text: { subtitle.to_string() },
                font_size: 11.5,
                font: { font },
                color: Color::srgb(0.50, 0.55, 0.65),
            }
        ]
    }
}

fn on_card_activate(trigger: On<UFocusActivate>, query: Query<&CardInfo>) {
    if let Ok(info) = query.get(trigger.entity) {
        info!("Card activated via keyboard/gamepad: {}", info.title);
    }
}

fn on_card_click(trigger: On<PointerClick>, query: Query<&CardInfo>) {
    if let Ok(info) = query.get(trigger.entity) {
        info!("Card clicked via mouse: {}", info.title);
    }
}

/// Updates the HUD status text reflecting the currently focused entity and modality.
fn update_hud_status(
    focus_state: Res<UFocusState>,
    card_query: Query<&CardInfo>,
    mut text_query: Query<&mut UText, With<StatusText>>,
) {
    if !focus_state.is_changed() {
        return;
    }

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    if let Some(entity) = focus_state.focused {
        let name = card_query.get(entity).map(|c| c.title).unwrap_or("Unknown");
        let modality_str = match focus_state.modality {
            FocusModality::Keyboard => "Keyboard",
            FocusModality::Gamepad => "Gamepad",
            FocusModality::Pointer => "Mouse Pointer",
        };
        text.text = format!("'{}' focused via {}", name, modality_str);
    } else {
        text.text = "No focus • Use Arrow Keys or Tab".to_string();
    }
}
