//! SDF Corners & Borders gallery example using Bevy Scene Notation (BSN).
//!
//! Demonstrates:
//! - Uniform rounded corners (`UCornerRadius::all`)
//! - Directional corner radii (`UCornerRadius::top`, `UCornerRadius::bottom`)
//! - Independent corner radii (`top_left`, `top_right`, `bottom_right`, `bottom_left`)
//! - Chamfered / Cut corners (`UShapeMode::Cut`) for Sci-Fi / Game HUD aesthetics
//! - Solid borders with customizable color, width, and corner radius (`UBorder`)
//! - Floating offset borders (`UBorder::offset`)

use bevy::prelude::*;
use bevy_luma::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - Corners & Borders Gallery".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_ui)
        .run();
}

fn setup_ui(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(gallery_scene(&theme));
}

fn gallery_scene(theme: &Theme) -> impl Scene {
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
            gap: 24.0,
        }
        Children [
            // Page Header
            (
                UNode {
                    width: UVal::Percent(1.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { "Corners & Borders Gallery".to_string() },
                        font_size: 26.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "Hardware-accelerated Signed Distance Field (SDF) anti-aliased corners, chamfered cuts, and borders.".to_string() },
                        font_size: 14.0,
                        font: { font.clone() },
                        color: Color::srgb(0.60, 0.65, 0.78),
                    })
                ]
            ),

            // 3-Column Responsive Grid of Showcase Cards
            (
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
                    gap: 20.0,
                }
                Children [
                    // Card 1: Uniform Rounded
                    @rounded_card(font.clone())
                    --
                    // Card 2: Chamfered / Cut (Sci-Fi / Angled)
                    @chamfered_card(font.clone())
                    --
                    // Card 3: Asymmetric Diagonal
                    @asymmetric_card(font.clone())
                    --
                    // Card 4: Top Corners Only (Tab Style)
                    @top_only_card(font.clone())
                    --
                    // Card 5: Bottom Corners Only (Sheet Style)
                    @bottom_only_card(font.clone())
                    --
                    // Card 6: Floating Outset Border (Offset)
                    @offset_border_card(font.clone())
                ]
            )
        ]
    }
}

/// Card 1: Uniform Rounded Corners (`UShapeMode::Round` + `UCornerRadius::all(16.0)`).
fn rounded_card(font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            height: UVal::Px(190.0),
            background_color: Color::srgb(0.10, 0.13, 0.20),
            border_radius: UCornerRadius::all(16.0),
            shape_mode: UShapeMode::Round,
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgb(0.28, 0.52, 0.95),
            width: 1.5,
            radius: UCornerRadius::all(16.0),
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { "1. Uniform Rounded".to_string() },
                        font_size: 17.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "border_radius: UCornerRadius::all(16.0)\nshape_mode: UShapeMode::Round".to_string() },
                        font_size: 12.0,
                        font: { font.clone() },
                        color: Color::srgb(0.55, 0.65, 0.82),
                    })
                ]
            ),
            // Mini badge preview inside
            (
                UNode {
                    width: UVal::Px(110.0),
                    height: UVal::Px(32.0),
                    background_color: Color::srgba(0.28, 0.52, 0.95, 0.20),
                    border_radius: UCornerRadius::all(8.0),
                    padding: USides::axes(8.0, 4.0),
                }
                UBorder {
                    color: Color::srgb(0.35, 0.60, 0.98),
                    width: 1.0,
                    radius: UCornerRadius::all(8.0),
                    offset: 0.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { "Smooth 8px".to_string() },
                        font_size: 12.0,
                        font,
                        color: Color::srgb(0.80, 0.90, 1.0),
                    })
                ]
            )
        ]
    }
}

/// Card 2: Chamfered Cut Corners (`UShapeMode::Cut` + `UCornerRadius::all(20.0)`).
fn chamfered_card(font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            height: UVal::Px(190.0),
            background_color: Color::srgb(0.14, 0.12, 0.08),
            border_radius: UCornerRadius::all(20.0),
            shape_mode: UShapeMode::Cut,
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgb(0.92, 0.65, 0.22),
            width: 1.5,
            radius: UCornerRadius::all(20.0),
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { "2. Chamfered / Cut".to_string() },
                        font_size: 17.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "border_radius: UCornerRadius::all(20.0)\nshape_mode: UShapeMode::Cut (45° cuts)".to_string() },
                        font_size: 12.0,
                        font: { font.clone() },
                        color: Color::srgb(0.85, 0.75, 0.55),
                    })
                ]
            ),
            // Mini badge preview inside
            (
                UNode {
                    width: UVal::Px(120.0),
                    height: UVal::Px(32.0),
                    background_color: Color::srgba(0.92, 0.65, 0.22, 0.20),
                    border_radius: UCornerRadius::all(10.0),
                    shape_mode: UShapeMode::Cut,
                    padding: USides::axes(8.0, 4.0),
                }
                UBorder {
                    color: Color::srgb(0.95, 0.75, 0.35),
                    width: 1.0,
                    radius: UCornerRadius::all(10.0),
                    offset: 0.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { "Mecha Bevel".to_string() },
                        font_size: 12.0,
                        font,
                        color: Color::srgb(1.0, 0.90, 0.70),
                    })
                ]
            )
        ]
    }
}

/// Card 3: Asymmetric Diagonal Corners (independent values per corner).
fn asymmetric_card(font: Handle<Font>) -> impl Scene {
    let radius = UCornerRadius {
        top_left: 30.0,
        top_right: 6.0,
        bottom_right: 30.0,
        bottom_left: 6.0,
    };

    bsn! {
        UNode {
            height: UVal::Px(190.0),
            background_color: Color::srgb(0.14, 0.10, 0.18),
            border_radius: radius,
            shape_mode: UShapeMode::Round,
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgb(0.72, 0.40, 0.92),
            width: 2.0,
            radius,
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { "3. Asymmetric Diagonals".to_string() },
                        font_size: 17.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "top_left/bottom_right: 30.0\ntop_right/bottom_left: 6.0".to_string() },
                        font_size: 12.0,
                        font: { font.clone() },
                        color: Color::srgb(0.78, 0.65, 0.88),
                    })
                ]
            ),
            // Mini badge preview inside
            (
                UNode {
                    width: UVal::Px(130.0),
                    height: UVal::Px(32.0),
                    background_color: Color::srgba(0.72, 0.40, 0.92, 0.20),
                    border_radius: {
                        UCornerRadius {
                            top_left: 14.0,
                            top_right: 2.0,
                            bottom_right: 14.0,
                            bottom_left: 2.0,
                        }
                    },
                    padding: USides::axes(8.0, 4.0),
                }
                UBorder {
                    color: Color::srgb(0.82, 0.50, 0.98),
                    width: 1.0,
                    radius: {
                        UCornerRadius {
                            top_left: 14.0,
                            top_right: 2.0,
                            bottom_right: 14.0,
                            bottom_left: 2.0,
                        }
                    },
                    offset: 0.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { "Diagonal Leaf".to_string() },
                        font_size: 12.0,
                        font,
                        color: Color::srgb(0.92, 0.82, 1.0),
                    })
                ]
            )
        ]
    }
}

/// Card 4: Top Corners Only (`UCornerRadius::top(24.0)`).
fn top_only_card(font: Handle<Font>) -> impl Scene {
    let radius = UCornerRadius::top(24.0);

    bsn! {
        UNode {
            height: UVal::Px(190.0),
            background_color: Color::srgb(0.08, 0.14, 0.16),
            border_radius: radius,
            shape_mode: UShapeMode::Round,
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgb(0.20, 0.80, 0.70),
            width: 1.5,
            radius,
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { "4. Top-Only Rounded".to_string() },
                        font_size: 17.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "border_radius: UCornerRadius::top(24.0)\nBottom corners remain sharp (0.0)".to_string() },
                        font_size: 12.0,
                        font: { font.clone() },
                        color: Color::srgb(0.55, 0.80, 0.75),
                    })
                ]
            ),
            // Mini tab preview inside
            (
                UNode {
                    width: UVal::Px(110.0),
                    height: UVal::Px(32.0),
                    background_color: Color::srgba(0.20, 0.80, 0.70, 0.20),
                    border_radius: UCornerRadius::top(12.0),
                    padding: USides::axes(8.0, 4.0),
                }
                UBorder {
                    color: Color::srgb(0.30, 0.90, 0.80),
                    width: 1.0,
                    radius: UCornerRadius::top(12.0),
                    offset: 0.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { "Tab Header".to_string() },
                        font_size: 12.0,
                        font,
                        color: Color::srgb(0.75, 1.0, 0.95),
                    })
                ]
            )
        ]
    }
}

/// Card 5: Bottom Corners Only (`UCornerRadius::bottom(24.0)`).
fn bottom_only_card(font: Handle<Font>) -> impl Scene {
    let radius = UCornerRadius::bottom(24.0);

    bsn! {
        UNode {
            height: UVal::Px(190.0),
            background_color: Color::srgb(0.16, 0.10, 0.12),
            border_radius: radius,
            shape_mode: UShapeMode::Round,
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgb(0.92, 0.40, 0.55),
            width: 1.5,
            radius,
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { "5. Bottom-Only Rounded".to_string() },
                        font_size: 17.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "border_radius: UCornerRadius::bottom(24.0)\nTop corners remain sharp (0.0)".to_string() },
                        font_size: 12.0,
                        font: { font.clone() },
                        color: Color::srgb(0.88, 0.65, 0.72),
                    })
                ]
            ),
            // Mini sheet preview inside
            (
                UNode {
                    width: UVal::Px(120.0),
                    height: UVal::Px(32.0),
                    background_color: Color::srgba(0.92, 0.40, 0.55, 0.20),
                    border_radius: UCornerRadius::bottom(12.0),
                    padding: USides::axes(8.0, 4.0),
                }
                UBorder {
                    color: Color::srgb(0.98, 0.50, 0.65),
                    width: 1.0,
                    radius: UCornerRadius::bottom(12.0),
                    offset: 0.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { "Drawer Sheet".to_string() },
                        font_size: 12.0,
                        font,
                        color: Color::srgb(1.0, 0.85, 0.90),
                    })
                ]
            )
        ]
    }
}

/// Card 6: Floating Outset Border (`UBorder::offset = 4.0`).
fn offset_border_card(font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            height: UVal::Px(190.0),
            background_color: Color::srgb(0.09, 0.15, 0.11),
            border_radius: UCornerRadius::all(16.0),
            shape_mode: UShapeMode::Round,
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgb(0.35, 0.85, 0.50),
            width: 2.0,
            radius: UCornerRadius::all(20.0),
            offset: 4.0, // Floating gap between node body and border!
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { "6. Floating Outset Border".to_string() },
                        font_size: 17.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "UBorder { offset: 4.0, width: 2.0 }\nCreates an air gap between body and border".to_string() },
                        font_size: 12.0,
                        font: { font.clone() },
                        color: Color::srgb(0.60, 0.82, 0.68),
                    })
                ]
            ),
            // Mini badge preview inside
            (
                UNode {
                    width: UVal::Px(110.0),
                    height: UVal::Px(32.0),
                    background_color: Color::srgba(0.35, 0.85, 0.50, 0.20),
                    border_radius: UCornerRadius::all(8.0),
                    padding: USides::axes(8.0, 4.0),
                }
                UBorder {
                    color: Color::srgb(0.45, 0.95, 0.60),
                    width: 1.5,
                    radius: UCornerRadius::all(10.0),
                    offset: 2.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { "Outset Gap".to_string() },
                        font_size: 12.0,
                        font,
                        color: Color::srgb(0.80, 1.0, 0.85),
                    })
                ]
            )
        ]
    }
}
