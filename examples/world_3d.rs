//! 3D In-World spatial UI panel example using Bevy Scene Notation (BSN).

use bevy::prelude::*;
use bevy_luma::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_3d_scene)
        .run();
}

fn setup_3d_scene(mut commands: Commands, theme: Res<Theme>) {
    // 3D Camera looking at the origin
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.5, 3.5).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // In-World 3D Spatial UI Terminal
    commands.spawn_scene(world_panel_scene(&theme));
}

fn world_panel_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    bsn! {
        URootUi::world_3d(Vec2::new(500.0, 320.0))
        Transform::from_xyz(0.0, 1.2, 0.0)
        UNode {
            width: UVal::Px(500.0),
            height: UVal::Px(320.0),
            background_color: Color::srgba(0.08, 0.12, 0.20, 0.90),
            border_radius: UCornerRadius::all(20.0),
            padding: USides::all(28.0),
        }
        UBorder {
            color: Color::srgb(0.20, 0.70, 1.0),
            width: 2.0,
            radius: UCornerRadius::all(20.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            (UText {
                text: { "3D In-World Terminal".to_string() },
                font_size: 24.0,
                font: { font.clone() },
                color: Color::WHITE,
            }),
            (UText {
                text: { "This panel exists physically in 3D world space.\nIdentical layout and SDF shaders work seamlessly in 3D.".to_string() },
                font_size: 14.0,
                font: { font.clone() },
                color: Color::srgb(0.65, 0.80, 0.95),
            }),
            (
                UNode {
                    width: UVal::Px(160.0),
                    height: UVal::Px(44.0),
                    background_color: Color::srgb(0.15, 0.55, 0.85),
                    border_radius: UCornerRadius::all(8.0),
                }
                UInteraction::default()
                UInteractionColors {
                    normal: Color::srgb(0.15, 0.55, 0.85),
                    hovered: Color::srgb(0.25, 0.65, 0.98),
                    pressed: Color::srgb(0.10, 0.40, 0.70),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { "Interact".to_string() },
                        font_size: 15.0,
                        font,
                        color: Color::WHITE,
                    })
                ]
                on(|_event: On<Pointer<Click>>| {
                    info!("3D Terminal button clicked in world space!");
                })
            )
        ]
    }
}
