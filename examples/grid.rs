//! Responsive 3-column CSS Grid layout example using Bevy Scene Notation (BSN).

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
    commands.spawn_scene(grid_scene(&theme));
}

fn grid_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.06, 0.07, 0.10),
            padding: USides::all(32.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            gap: 20.0,
        }
        Children [
            // Page Header
            UText {
                text: { "Responsive CSS Grid (3 Columns)".to_string() },
                font_size: 22.0,
                font: { font.clone() },
                color: Color::WHITE,
            }
            --
            // 3-Column Grid Container
            UNode {
                width: UVal::Percent(1.0),
            }
            ULayout {
                display: UDisplay::Grid,
                grid_template_columns: vec![
                    UTrackSize::Fr(1.0),
                    UTrackSize::Fr(1.0),
                    UTrackSize::Fr(1.0),
                ],
                gap: 16.0,
            }
            Children [
                @grid_card("Card 1", "SDF Engine", font.clone())
                --
                @grid_card("Card 2", "Flexbox & Grid", font.clone())
                --
                @grid_card("Card 3", "2D/3D Hybrid", font.clone())
                --
                @grid_card("Card 4", "Lucide Icons", font.clone())
                --
                @grid_card("Card 5", "Arabic RTL", font.clone())
                --
                @grid_card("Card 6", "Spatial Focus", font.clone())
            ]
        ]
    }
}

fn grid_card(title: &str, subtitle: &str, font: Handle<Font>) -> impl Scene {
    let title = title.to_string();
    let subtitle = subtitle.to_string();
    bsn! {
        UNode {
            height: UVal::Px(120.0),
            background_color: Color::srgb(0.12, 0.15, 0.22),
            border_radius: UCornerRadius::all(12.0),
            padding: USides::all(16.0),
        }
        UBorder {
            color: Color::srgb(0.22, 0.28, 0.40),
            width: 1.0,
            radius: UCornerRadius::all(12.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            UText {
                text: title,
                font_size: 16.0,
                font: { font.clone() },
                color: Color::WHITE,
            }
            --
            UText {
                text: subtitle,
                font_size: 13.0,
                font,
                color: Color::srgb(0.55, 0.65, 0.80),
            }
        ]
    }
}
