//! 150 Dark Color Gradients Gallery in Bevy Luma.
//!
//! A comprehensive showcase gallery of handcrafted dark-mode color gradients
//! faithfully inspired by professional digital asset packs:
//! - Pitch black across the top half of each card, with vibrant luminous color
//!   rising from the bottom edge and corners into the darkness.
//! - 60 curated dark gradient swatch cards in a responsive 6-column grid.
//! - Multi-stop linear horizons, arched radial auras, and angled corner flares.
//! - Large hero feature cards showing full-scale animated ambient gradients.
//! - Interactive hover and focus ring navigation via keyboard arrows & gamepad.

use bevy::prelude::*;
use bevy_luma::prelude::*;
use core::f32::consts::FRAC_PI_2;

const OBSIDIAN: Color = Color::srgb(0.015, 0.015, 0.022);

#[derive(Component, Clone, Copy, Default)]
struct HeroAmbientDrift {
    speed: f32,
    base_offset: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - 150 Dark Color Gradients Gallery".into(),
                resolution: (1380, 880).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_dark_gradients_gallery)
        .add_systems(Update, animate_hero_ambient_drift)
        .run();
}

fn setup_dark_gradients_gallery(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(dark_gradients_scene(&theme));
}

fn dark_gradients_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.04, 0.04, 0.05),
            padding: USides::axes(28.0, 20.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
        }
        Children [
            // Scrollable gallery container
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            UScrollContainer::vertical()
            UScrollKineticDrag::default()
            Children [
                UNode {
                    width: UVal::Percent(1.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 22.0,
                }
                Children [
                    // Top Hero Section: Title Banner + Large Showcase Cards
                    @gallery_hero_header(font.clone())

                    --

                    // Section 1: DARK GRADIENTS 01 - 30
                    @gallery_section_header("DARK GRADIENTS", "01 - 30", font.clone())

                    --

                    // Grid 01 - 30
                    @swatches_grid_1_to_30(font.clone())

                    --

                    // Section 2: DARK GRADIENTS 31 - 60
                    @gallery_section_header("DARK GRADIENTS", "31 - 60", font.clone())

                    --

                    // Grid 31 - 60
                    @swatches_grid_31_to_60(font.clone())

                    --

                    // Footer credits
                    UNode {
                        width: UVal::Percent(1.0),
                        background_color: Color::srgba(0.08, 0.08, 0.10, 0.50),
                        border_radius: UCornerRadius::all(12.0),
                        padding: USides::axes(24.0, 16.0),
                        margin: USides::bottom(24.0),
                    }
                    UBorder {
                        color: Color::srgba(1.0, 1.0, 1.0, 0.06),
                        width: 1.0,
                        radius: UCornerRadius::all(12.0),
                    }
                    ULayout {
                        display: UDisplay::Flex,
                        flex_direction: UFlexDirection::Row,
                        justify_content: UJustifyContent::SpaceBetween,
                        align_items: UAlignItems::Center,
                    }
                    Children [
                        UText {
                            text: { "150 Dark Color Gradients • Dark Obsidian Top with Bottom Rising Color Horizons • Pure GPU SDF".to_string() },
                            font_size: 13.0,
                            font: { font.clone() },
                            color: Color::srgb(0.55, 0.58, 0.68),
                        }
                        --
                        UText {
                            text: { "Press Tab or Arrow Keys to navigate swatches • Smooth Mouse Scroll".to_string() },
                            font_size: 12.0,
                            font: { font.clone() },
                            color: Color::srgb(0.35, 0.75, 0.50),
                        }
                    ]
                ]
            ]
        ]
    }
}

// ---------------------------------------------------------------------------
// Gradient Construction Helpers: Bottom Glow with Black Top
// ---------------------------------------------------------------------------

/// 1. Classic Bottom Horizon: Pure pitch black across top 50%, glowing vibrant color at the bottom.
fn bottom_horizon(color: Color) -> UGradient {
    let srgba = color.to_srgba();
    let mid_color = Color::srgba(srgba.red * 0.42, srgba.green * 0.42, srgba.blue * 0.42, 1.0);
    UGradient::linear_stops(
        FRAC_PI_2, // Top to Bottom
        [
            UGradientStop::new(0.0, OBSIDIAN),
            UGradientStop::new(0.40, OBSIDIAN),
            UGradientStop::new(0.70, mid_color),
            UGradientStop::new(1.0, color),
        ],
    )
}

/// 2. Arching Bottom Radial Dome: Curved luminous aurora rising from the bottom edge into black.
fn bottom_dome(color: Color, mid_color: Color) -> UGradient {
    UGradient::radial_stops_custom(
        Vec2::new(0.5, 1.20),
        0.88,
        [
            UGradientStop::new(0.0, color),
            UGradientStop::new(0.38, mid_color),
            UGradientStop::new(0.70, OBSIDIAN),
            UGradientStop::new(1.0, OBSIDIAN),
        ],
    )
}

/// 3. Angled Bottom-Left Flare: Glow rises diagonally from bottom-left, top is pitch black.
fn bottom_left_flare(color: Color, mid_color: Color) -> UGradient {
    UGradient::linear_stops(
        1.95, // ~112 degrees angle
        [
            UGradientStop::new(0.0, OBSIDIAN),
            UGradientStop::new(0.44, OBSIDIAN),
            UGradientStop::new(0.72, mid_color),
            UGradientStop::new(1.0, color),
        ],
    )
}

/// 4. Angled Bottom-Right Flare: Glow rises diagonally from bottom-right, top is pitch black.
fn bottom_right_flare(color: Color, mid_color: Color) -> UGradient {
    UGradient::linear_stops(
        1.18, // ~68 degrees angle
        [
            UGradientStop::new(0.0, OBSIDIAN),
            UGradientStop::new(0.44, OBSIDIAN),
            UGradientStop::new(0.72, mid_color),
            UGradientStop::new(1.0, color),
        ],
    )
}

/// 5. Dual-Color Bottom Horizon: Bottom edge blends two vivid colors together, rising into black.
fn dual_horizon(color_left: Color, color_right: Color) -> UGradient {
    UGradient::radial_stops_custom(
        Vec2::new(0.5, 1.15),
        0.85,
        [
            UGradientStop::new(0.0, color_left),
            UGradientStop::new(0.42, color_right),
            UGradientStop::new(0.72, OBSIDIAN),
            UGradientStop::new(1.0, OBSIDIAN),
        ],
    )
}

// ---------------------------------------------------------------------------
// Hero Header: Title + Badges + 2 Large Feature Gradient Previews
// ---------------------------------------------------------------------------

fn gallery_hero_header(font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            background_color: Color::srgba(0.06, 0.06, 0.08, 0.70),
            border_radius: UCornerRadius::all(20.0),
            padding: USides::all(22.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
            radius: UCornerRadius::all(20.0),
        }
        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.40), Vec2::new(0.0, 8.0), 20.0)
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
            gap: 28.0,
        }
        Children [
            // Left Branding Column
            UNode {
                width: UVal::Px(360.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 6.0,
            }
            Children [
                // "150" Large Display Number
                UText {
                    text: { "150".to_string() },
                    font_size: 64.0,
                    font: { font.clone() },
                    color: Color::srgb(0.96, 0.65, 0.25),
                }

                --

                // Title
                UText {
                    text: { "DARK COLOR\nGRADIENTS".to_string() },
                    font_size: 26.0,
                    font: { font.clone() },
                    color: Color::srgb(0.98, 0.98, 1.0),
                }

                --

                // Format Badges (.PNG & .JPEG)
                UNode {
                    margin: USides::top(6.0),
                    ..UNode::default()
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    gap: 8.0,
                }
                Children [
                    UNode {
                        background_color: Color::srgb(0.95, 0.40, 0.20),
                        border_radius: UCornerRadius::all(14.0),
                        padding: USides::axes(14.0, 5.0),
                    }
                    Children [
                        UText {
                            text: { ".PNG".to_string() },
                            font_size: 11.0,
                            font: { font.clone() },
                            color: Color::srgb(1.0, 1.0, 1.0),
                        }
                    ]

                    --

                    UNode {
                        background_color: Color::srgb(0.45, 0.15, 0.95),
                        border_radius: UCornerRadius::all(14.0),
                        padding: USides::axes(14.0, 5.0),
                    }
                    Children [
                        UText {
                            text: { ".JPEG".to_string() },
                            font_size: 11.0,
                            font: { font.clone() },
                            color: Color::srgb(1.0, 1.0, 1.0),
                        }
                    ]
                ]

                --

                UText {
                    text: { "Obsidian black top with radiant colored horizons rising from below".to_string() },
                    font_size: 12.0,
                    font: { font.clone() },
                    color: Color::srgb(0.50, 0.54, 0.64),
                }
            ]

            --

            // Right Column: Two Large Showcase Landscape Cards
            UNode::default()
            USelf {
                flex_grow: { Some(1.0) },
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                gap: 16.0,
                justify_content: UJustifyContent::End,
            }
            Children [
                // Feature Card 1: Electric Violet Horizon (Black Top, Glowing Violet Bottom)
                @hero_showcase_card(
                    "FEATURE 01 • VOID AURORA",
                    UGradient::linear_stops(
                        FRAC_PI_2,
                        [
                            UGradientStop::new(0.0, OBSIDIAN),
                            UGradientStop::new(0.38, OBSIDIAN),
                            UGradientStop::new(0.68, Color::srgb(0.55, 0.05, 0.65)),
                            UGradientStop::new(1.0, Color::srgb(0.95, 0.15, 0.55)),
                        ],
                    ),
                    0.75,
                    font.clone(),
                )

                --

                // Feature Card 2: Solar Prism Horizon (Black Top, Glowing Cyan & Orange Bottom)
                @hero_showcase_card(
                    "FEATURE 02 • SOLAR PRISM",
                    UGradient::radial_stops_custom(
                        Vec2::new(0.5, 1.15),
                        0.85,
                        [
                            UGradientStop::new(0.0, Color::srgb(1.0, 0.80, 0.20)),
                            UGradientStop::new(0.40, Color::srgb(1.0, 0.40, 0.05)),
                            UGradientStop::new(0.70, OBSIDIAN),
                            UGradientStop::new(1.0, OBSIDIAN),
                        ],
                    ),
                    1.10,
                    font,
                )
            ]
        ]
    }
}

/// Large showcase card at the top.
fn hero_showcase_card(
    label: &'static str,
    gradient: UGradient,
    drift_speed: f32,
    font: Handle<Font>,
) -> impl Scene {
    bsn! {
        HeroAmbientDrift {
            speed: drift_speed,
            base_offset: 0.0,
        }
        UNode {
            width: UVal::Px(340.0),
            height: UVal::Px(130.0),
            background_color: OBSIDIAN,
            border_radius: UCornerRadius::all(14.0),
            padding: USides::all(14.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.12),
            width: 1.5,
            radius: UCornerRadius::all(14.0),
        }
        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.60), Vec2::new(0.0, 8.0), 24.0)
        gradient
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            // Top Badge (In the black zone)
            UNode {
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.65),
                border_radius: UCornerRadius::all(6.0),
                padding: USides::axes(10.0, 4.0),
            }
            UBorder {
                color: Color::srgba(1.0, 1.0, 1.0, 0.15),
                width: 1.0,
                radius: UCornerRadius::all(6.0),
            }
            Children [
                UText {
                    text: { label.to_string() },
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.95, 0.96, 0.98),
                }
            ]

            --

            // Bottom Subtext
            UNode {
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.50),
                border_radius: UCornerRadius::all(6.0),
                padding: USides::axes(8.0, 3.0),
            }
            Children [
                UText {
                    text: { "Pitch Black Top • Radiant Bottom Horizon".to_string() },
                    font_size: 10.0,
                    font: { font },
                    color: Color::srgba(1.0, 1.0, 1.0, 0.75),
                }
            ]
        ]
    }
}

// ---------------------------------------------------------------------------
// Section Header: Pill Label + Range Indicator (e.g. 01 - 30)
// ---------------------------------------------------------------------------

fn gallery_section_header(
    category: &'static str,
    range: &'static str,
    font: Handle<Font>,
) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            // Left Pill: DARK GRADIENTS
            UNode {
                background_color: Color::srgb(0.40, 0.12, 0.90),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::axes(18.0, 6.0),
            }
            UShadow::glow(Color::srgba(0.40, 0.12, 0.90, 0.45), 10.0)
            Children [
                UText {
                    text: { category.to_string() },
                    font_size: 12.0,
                    font: { font.clone() },
                    color: Color::srgb(1.0, 1.0, 1.0),
                }
            ]

            --

            // Right Pill: Range (01 - 30)
            UNode {
                background_color: Color::srgb(0.95, 0.96, 0.98),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::axes(20.0, 6.0),
            }
            Children [
                UText {
                    text: { range.to_string() },
                    font_size: 12.0,
                    font: { font },
                    color: Color::srgb(0.08, 0.08, 0.10),
                }
            ]
        ]
    }
}

// ---------------------------------------------------------------------------
// Helper: Interactive Swatch Card (Number placed in upper black region)
// ---------------------------------------------------------------------------

fn swatch_card(
    id: &'static str,
    gradient: UGradient,
    font: Handle<Font>,
) -> impl Scene {
    bsn! {
        UNode {
            height: UVal::Px(82.0),
            background_color: OBSIDIAN,
            border_radius: UCornerRadius::all(12.0),
            padding: USides::axes(12.0, 10.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
            radius: UCornerRadius::all(12.0),
        }
        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.50), Vec2::new(0.0, 4.0), 10.0)
        gradient
        UFocusable::new()
        UFocusVisual::ring(Color::srgba(1.0, 1.0, 1.0, 0.65))
            .with_contract(4.0, 0.0)
            .with_width(2.0)
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::Start,
            align_items: UAlignItems::Center,
        }
        Children [
            // The number sits in the upper black zone, perfectly clear and readable
            UText {
                text: { id.to_string() },
                font_size: 14.0,
                font: { font },
                color: Color::srgb(0.96, 0.97, 1.0),
            }
        ]
    }
}

// ---------------------------------------------------------------------------
// 6-Column Grid: Swatches 01 to 30 (Curated Bottom Horizons)
// ---------------------------------------------------------------------------

fn swatches_grid_1_to_30(font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
        }
        ULayout {
            display: UDisplay::Grid,
            grid_template_columns: vec![
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
            ],
            gap: 12.0,
        }
        Children [
            // Row 1 (01 - 06)
            @swatch_card("01", dual_horizon(Color::srgb(0.05, 0.35, 1.0), Color::srgb(0.85, 0.10, 0.70)), font.clone())
            --
            @swatch_card("02", bottom_dome(Color::srgb(0.05, 0.95, 0.55), Color::srgb(0.05, 0.35, 0.50)), font.clone())
            --
            @swatch_card("03", bottom_horizon(Color::srgb(0.80, 0.10, 0.65)), font.clone())
            --
            @swatch_card("04", bottom_horizon(Color::srgb(0.98, 0.20, 0.05)), font.clone())
            --
            @swatch_card("05", bottom_horizon(Color::srgb(0.75, 0.08, 0.18)), font.clone())
            --
            @swatch_card("06", dual_horizon(Color::srgb(0.95, 0.45, 0.60), Color::srgb(1.0, 0.65, 0.45)), font.clone())

            --

            // Row 2 (07 - 12)
            @swatch_card("07", dual_horizon(Color::srgb(0.05, 0.85, 0.92), Color::srgb(0.92, 0.12, 0.68)), font.clone())
            --
            @swatch_card("08", bottom_dome(Color::srgb(0.0, 0.85, 1.0), Color::srgb(0.05, 0.20, 0.75)), font.clone())
            --
            @swatch_card("09", bottom_horizon(Color::srgb(0.90, 0.25, 0.80)), font.clone())
            --
            @swatch_card("10", bottom_left_flare(Color::srgb(0.0, 0.95, 1.0), Color::srgb(0.05, 0.20, 0.55)), font.clone())
            --
            @swatch_card("11", dual_horizon(Color::srgb(1.0, 0.45, 0.08), Color::srgb(0.92, 0.10, 0.50)), font.clone())
            --
            @swatch_card("12", dual_horizon(Color::srgb(0.95, 0.25, 0.45), Color::srgb(1.0, 0.75, 0.30)), font.clone())

            --

            // Row 3 (13 - 18)
            @swatch_card("13", bottom_horizon(Color::srgb(1.0, 0.42, 0.06)), font.clone())
            --
            @swatch_card("14", bottom_horizon(Color::srgb(0.90, 0.08, 0.60)), font.clone())
            --
            @swatch_card("15", bottom_dome(Color::srgb(1.0, 0.60, 0.05), Color::srgb(0.50, 0.20, 0.02)), font.clone())
            --
            @swatch_card("16", dual_horizon(Color::srgb(0.0, 0.75, 1.0), Color::srgb(1.0, 0.40, 0.15)), font.clone())
            --
            @swatch_card("17", bottom_horizon(Color::srgb(0.75, 0.02, 0.18)), font.clone())
            --
            @swatch_card("18", bottom_dome(Color::srgb(0.08, 0.45, 0.95), Color::srgb(0.02, 0.10, 0.35)), font.clone())

            --

            // Row 4 (19 - 24)
            @swatch_card("19", bottom_horizon(Color::srgb(0.98, 0.12, 0.25)), font.clone())
            --
            @swatch_card("20", bottom_dome(Color::srgb(0.08, 0.88, 0.65), Color::srgb(0.02, 0.35, 0.30)), font.clone())
            --
            @swatch_card("21", dual_horizon(Color::srgb(0.05, 0.75, 1.0), Color::srgb(1.0, 0.75, 0.10)), font.clone())
            --
            @swatch_card("22", dual_horizon(Color::srgb(0.96, 0.40, 0.35), Color::srgb(0.98, 0.88, 0.88)), font.clone())
            --
            @swatch_card("23", dual_horizon(Color::srgb(0.25, 0.95, 0.25), Color::srgb(1.0, 0.95, 0.45)), font.clone())
            --
            @swatch_card("24", bottom_horizon(Color::srgb(0.78, 0.55, 0.95)), font.clone())

            --

            // Row 5 (25 - 30)
            @swatch_card("25", bottom_horizon(Color::srgb(0.98, 0.12, 0.75)), font.clone())
            --
            @swatch_card("26", dual_horizon(Color::srgb(0.12, 0.30, 0.95), Color::srgb(0.96, 0.25, 0.70)), font.clone())
            --
            @swatch_card("27", bottom_dome(Color::srgb(0.02, 0.92, 0.95), Color::srgb(0.05, 0.35, 0.50)), font.clone())
            --
            @swatch_card("28", bottom_horizon(Color::srgb(0.12, 0.98, 0.40)), font.clone())
            --
            @swatch_card("29", dual_horizon(Color::srgb(0.98, 0.65, 0.72), Color::srgb(0.35, 0.25, 0.65)), font.clone())
            --
            @swatch_card("30", dual_horizon(Color::srgb(1.0, 0.70, 0.28), Color::srgb(0.20, 0.08, 0.50)), font)
        ]
    }
}

// ---------------------------------------------------------------------------
// 6-Column Grid: Swatches 31 to 60 (Curated Bottom Horizons)
// ---------------------------------------------------------------------------

fn swatches_grid_31_to_60(font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
        }
        ULayout {
            display: UDisplay::Grid,
            grid_template_columns: vec![
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
                UTrackSize::Fr(1.0),
            ],
            gap: 12.0,
        }
        Children [
            // Row 6 (31 - 36)
            @swatch_card("31", dual_horizon(Color::srgb(0.02, 0.95, 0.85), Color::srgb(0.90, 0.18, 0.75)), font.clone())
            --
            @swatch_card("32", bottom_dome(Color::srgb(0.35, 0.98, 0.75), Color::srgb(0.05, 0.30, 0.45)), font.clone())
            --
            @swatch_card("33", dual_horizon(Color::srgb(0.10, 0.35, 1.0), Color::srgb(1.0, 0.80, 0.20)), font.clone())
            --
            @swatch_card("34", bottom_left_flare(Color::srgb(0.98, 0.15, 0.65), Color::srgb(0.40, 0.05, 0.25)), font.clone())
            --
            @swatch_card("35", dual_horizon(Color::srgb(0.96, 0.28, 0.18), Color::srgb(0.08, 0.30, 0.80)), font.clone())
            --
            @swatch_card("36", dual_horizon(Color::srgb(0.08, 0.35, 0.45), Color::srgb(0.98, 0.88, 0.75)), font.clone())

            --

            // Row 7 (37 - 42)
            @swatch_card("37", dual_horizon(Color::srgb(1.0, 0.72, 0.10), Color::srgb(0.45, 0.08, 0.55)), font.clone())
            --
            @swatch_card("38", bottom_dome(Color::srgb(0.02, 0.92, 0.98), Color::srgb(0.15, 0.25, 0.85)), font.clone())
            --
            @swatch_card("39", dual_horizon(Color::srgb(1.0, 0.50, 0.15), Color::srgb(0.85, 0.08, 0.65)), font.clone())
            --
            @swatch_card("40", bottom_horizon(Color::srgb(0.85, 0.75, 0.98)), font.clone())
            --
            @swatch_card("41", dual_horizon(Color::srgb(0.12, 0.92, 0.55), Color::srgb(1.0, 0.52, 0.10)), font.clone())
            --
            @swatch_card("42", bottom_horizon(Color::srgb(0.80, 0.85, 0.92)), font.clone())

            --

            // Row 8 (43 - 48)
            @swatch_card("43", dual_horizon(Color::srgb(0.55, 0.98, 0.90), Color::srgb(0.65, 0.40, 0.90)), font.clone())
            --
            @swatch_card("44", dual_horizon(Color::srgb(0.45, 0.90, 0.85), Color::srgb(1.0, 0.55, 0.20)), font.clone())
            --
            @swatch_card("45", bottom_horizon(Color::srgb(0.90, 0.35, 0.50)), font.clone())
            --
            @swatch_card("46", bottom_right_flare(Color::srgb(0.98, 0.12, 0.55), Color::srgb(0.35, 0.05, 0.35)), font.clone())
            --
            @swatch_card("47", bottom_horizon(Color::srgb(0.60, 0.08, 0.98)), font.clone())
            --
            @swatch_card("48", bottom_dome(Color::srgb(0.55, 0.90, 1.0), Color::srgb(0.15, 0.45, 0.98)), font.clone())

            --

            // Row 9 (49 - 54)
            @swatch_card("49", bottom_horizon(Color::srgb(0.95, 0.18, 0.70)), font.clone())
            --
            @swatch_card("50", dual_horizon(Color::srgb(0.98, 0.55, 0.45), Color::srgb(0.35, 0.08, 0.35)), font.clone())
            --
            @swatch_card("51", bottom_horizon(Color::srgb(0.98, 0.45, 0.65)), font.clone())
            --
            @swatch_card("52", dual_horizon(Color::srgb(0.92, 0.92, 0.20), Color::srgb(0.15, 0.85, 0.75)), font.clone())
            --
            @swatch_card("53", dual_horizon(Color::srgb(0.98, 0.60, 0.50), Color::srgb(0.45, 0.08, 0.20)), font.clone())
            --
            @swatch_card("54", bottom_dome(Color::srgb(0.15, 0.95, 0.98), Color::srgb(0.08, 0.45, 0.60)), font.clone())

            --

            // Row 10 (55 - 60)
            @swatch_card("55", bottom_horizon(Color::srgb(0.90, 0.55, 0.85)), font.clone())
            --
            @swatch_card("56", dual_horizon(Color::srgb(0.98, 0.65, 0.55), Color::srgb(0.40, 0.08, 0.45)), font.clone())
            --
            @swatch_card("57", dual_horizon(Color::srgb(0.10, 0.85, 0.95), Color::srgb(1.0, 0.75, 0.20)), font.clone())
            --
            @swatch_card("58", bottom_horizon(Color::srgb(0.90, 0.08, 0.30)), font.clone())
            --
            @swatch_card("59", dual_horizon(Color::srgb(1.0, 0.85, 0.35), Color::srgb(0.95, 0.20, 0.25)), font.clone())
            --
            @swatch_card("60", dual_horizon(Color::srgb(0.15, 0.90, 1.0), Color::srgb(0.98, 0.15, 0.80)), font)
        ]
    }
}

// ---------------------------------------------------------------------------
// Animation System: Ambient Drift on Hero Feature Cards
// ---------------------------------------------------------------------------

fn animate_hero_ambient_drift(
    time: Res<Time>,
    mut query: Query<(&HeroAmbientDrift, &mut UGradient)>,
) {
    let t = time.elapsed_secs();
    for (drift, mut grad) in &mut query {
        grad.offset = (t * drift.speed * 0.15).sin() * 0.05 + drift.base_offset;
    }
}
