//! Interactive counter example showing state mutation with Bevy Scene Notation (BSN).

use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Resource, Default)]
struct Counter(i32);

#[derive(Component, Clone, Default)]
struct CounterDisplay;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LumaPlugin)
        .init_resource::<Counter>()
        .add_systems(Startup, setup_ui)
        .add_systems(Update, update_counter_display)
        .run();
}

fn setup_ui(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(counter_scene(&theme));
}

fn counter_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

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
                width: UVal::Px(320.0),
                background_color: Color::srgb(0.13, 0.15, 0.22),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(24.0),
            }
            UBorder {
                color: Color::srgb(0.25, 0.30, 0.45),
                width: 1.0,
                radius: UCornerRadius::all(16.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                align_items: UAlignItems::Center,
                gap: 20.0,
            }
            Children [
                UText {
                    text: { "Luma Counter".to_string() },
                    font_size: 18.0,
                    font: { font.clone() },
                    color: Color::srgb(0.65, 0.70, 0.85),
                }
                --
                CounterDisplay
                UText {
                    text: { "0".to_string() },
                    font_size: 48.0,
                    font: { font.clone() },
                    color: Color::WHITE,
                }
                --
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    gap: 16.0,
                }
                Children [
                    @counter_btn("-", font.clone())
                    on(|_event: On<PointerClick>, mut counter: ResMut<Counter>| {
                        counter.0 -= 1;
                    })
                    --
                    @counter_btn("+", font.clone())
                    on(|_event: On<PointerClick>, mut counter: ResMut<Counter>| {
                        counter.0 += 1;
                    })
                ]
            ]
        ]
    }
}

fn counter_btn(symbol: &str, font: Handle<Font>) -> impl Scene {
    let symbol = symbol.to_string();
    bsn! {
        UNode {
            width: UVal::Px(60.0),
            height: UVal::Px(44.0),
            background_color: Color::srgb(0.20, 0.35, 0.70),
            border_radius: UCornerRadius::all(8.0),
        }
        UInteraction::default()
        UInteractionColors {
            normal: Color::srgb(0.20, 0.35, 0.70),
            hovered: Color::srgb(0.28, 0.45, 0.85),
            pressed: Color::srgb(0.15, 0.25, 0.50),
        }
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            UText {
                text: symbol,
                font_size: 24.0,
                font,
                color: Color::WHITE,
            }
        ]
    }
}

fn update_counter_display(counter: Res<Counter>, mut query: Query<&mut UText, With<CounterDisplay>>) {
    if counter.is_changed() {
        for mut text in query.iter_mut() {
            text.text = counter.0.to_string();
        }
    }
}
