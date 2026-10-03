//! Centered modal dialog example using Bevy Scene Notation (BSN).

use bevy::prelude::*;
use bevy_luma::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_ui)
        .run();
}

fn setup_ui(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(modal_scene(&theme));
}

fn modal_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.65), // Dimmed backdrop
        }
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            (
                UNode {
                    width: UVal::Px(420.0),
                    background_color: Color::srgb(0.12, 0.14, 0.20),
                    border_radius: UCornerRadius::all(16.0),
                    padding: USides::all(24.0),
                }
                UBorder {
                    color: Color::srgb(0.25, 0.30, 0.42),
                    width: 1.0,
                    radius: UCornerRadius::all(16.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 16.0,
                }
                Children [
                    // Dialog Title
                    (UText {
                        text: { "Confirm Action".to_string() },
                        font_size: 20.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    // Dialog Body
                    (UText {
                        text: { "Are you sure you want to proceed with this operation?".to_string() },
                        font_size: 14.0,
                        font: { font.clone() },
                        color: Color::srgb(0.70, 0.75, 0.85),
                    }),
                    // Buttons Row
                    (
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Row,
                            justify_content: UJustifyContent::End,
                            gap: 12.0,
                        }
                        UNode {
                            width: UVal::Percent(1.0),
                        }
                        Children [
                            (
                                dialog_button("Cancel", Color::srgb(0.22, 0.25, 0.32), Color::srgb(0.28, 0.32, 0.42), font.clone())
                                on(|_event: On<Pointer<Click>>| {
                                    info!("Cancel clicked");
                                })
                            ),
                            (
                                dialog_button("Confirm", Color::srgb(0.20, 0.45, 0.85), Color::srgb(0.28, 0.55, 0.95), font.clone())
                                on(|_event: On<Pointer<Click>>| {
                                    info!("Confirm clicked");
                                })
                            ),
                        ]
                    )
                ]
            )
        ]
    }
}

fn dialog_button(label: &str, bg: Color, hover: Color, font: Handle<Font>) -> impl Scene {
    let label = label.to_string();
    bsn! {
        UNode {
            background_color: bg,
            border_radius: UCornerRadius::all(8.0),
            padding: USides::axes(18.0, 10.0),
        }
        UInteraction::default()
        UInteractionColors {
            normal: bg,
            hovered: hover,
            pressed: bg,
        }
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            (UText {
                text: label,
                font_size: 14.0,
                font,
                color: Color::WHITE,
            })
        ]
    }
}
