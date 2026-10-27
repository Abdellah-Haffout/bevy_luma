//! Minimal interactive button example using Bevy Scene Notation (BSN).

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
    commands.spawn_scene(button_scene(&theme));
}

fn button_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();
    let icon_font = theme.icon.font.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.08, 0.09, 0.12),
        }
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            UNode {
                width: UVal::Px(180.0),
                height: UVal::Px(48.0),
                background_color: Color::srgb(0.2, 0.35, 0.75),
                border_radius: UCornerRadius::all(10.0),
                padding: USides::axes(16.0, 8.0),
            }
            UBorder {
                color: Color::srgb(0.35, 0.5, 0.9),
                width: 1.0,
                radius: UCornerRadius::all(10.0),
            }
            UInteraction::default()
            UInteractionColors {
                normal: Color::srgb(0.2, 0.35, 0.75),
                hovered: Color::srgb(0.28, 0.45, 0.9),
                pressed: Color::srgb(0.14, 0.25, 0.55),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                gap: 8.0,
            }
            Children [
                UText {
                    text: { Icon::PLAY.to_string() },
                    font_size: 16.0,
                    font: { icon_font },
                    color: Color::WHITE,
                }
                --
                UText {
                    text: { "Click Me".to_string() },
                    font_size: 15.0,
                    font: { font },
                    color: Color::WHITE,
                }
            ]
            on(|_event: On<PointerClick>| {
                info!("Button clicked!");
            })
        ]
    }
}
