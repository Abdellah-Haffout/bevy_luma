//! Complex Color Synthesis & Aurora Mesh Gradients in Bevy Luma.
//!
//! Demonstrates how to create high-end complex color fields inspired by:
//! 1. **Multi-Point Aurora Mesh Gradient (Image 2)**:
//!    - Multi-layered 2D radial diffusion blending 4 distinct color focal points:
//!      Deep Electric Blue, Midnight Violet, Crimson Fire, and Warm Tangerine Ember.
//!    - Smooth harmonic floating drift animation simulating living aurora curtains.
//! 2. **Prism Light Beams & Micro-Ribbed Diffraction (Image 1)**:
//!    - Multi-directional anisotropic light beams slicing across deep obsidian black.
//!    - Micro-ribbed scanline texture simulating pleated fabric / diffraction grating.
//!    - Dynamic shimmer sweep animation.

use bevy::prelude::*;
use bevy_luma::prelude::*;
use core::f32::consts::FRAC_PI_4;

#[derive(Component, Clone, Copy, Default)]
struct AuroraBlob {
    base_center: Vec2,
    base_radius: f32,
    speed: f32,
    phase: f32,
    radius_amp: f32,
}

#[derive(Component, Clone, Copy, Default)]
struct PrismSweep {
    speed: f32,
    base_offset: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - Complex Color Synthesis (Aurora Mesh & Prism Beams)".into(),
                resolution: (1280, 840).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_complex_gradients_scene)
        .add_systems(Update, (animate_aurora_drift, animate_prism_sweep))
        .run();
}

fn setup_complex_gradients_scene(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(complex_gradients_scene(&theme));
}

fn complex_gradients_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();
    let icon_font = theme.icon.font.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.035, 0.038, 0.048),
            padding: USides::axes(32.0, 24.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            // --- Header Banner ---
            UNode {
                width: UVal::Percent(1.0),
                max_width: 1040.0,
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
            }
            Children [
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 4.0,
                }
                Children [
                    UNode::default()
                    ULayout {
                        display: UDisplay::Flex,
                        flex_direction: UFlexDirection::Row,
                        align_items: UAlignItems::Center,
                        gap: 10.0,
                    }
                    Children [
                        UText {
                            text: { "Luma UI • Complex Color Synthesis".to_string() },
                            font_size: 20.0,
                            font: { font.clone() },
                            color: Color::srgb(0.96, 0.97, 0.99),
                        }
                        --
                        UNode {
                            background_color: Color::srgba(0.30, 0.45, 1.0, 0.18),
                            border_radius: UCornerRadius::all(6.0),
                            padding: USides::axes(8.0, 3.0),
                        }
                        UBorder {
                            color: Color::srgba(0.40, 0.60, 1.0, 0.35),
                            width: 1.0,
                            radius: UCornerRadius::all(6.0),
                        }
                        Children [
                            UText {
                                text: { "SDF Multi-Layer Diffusion".to_string() },
                                font_size: 11.0,
                                font: { font.clone() },
                                color: Color::srgb(0.65, 0.80, 1.0),
                            }
                        ]
                    ]
                    --
                    UText {
                        text: { "Simulating 2D Mesh Gradients (Aurora) and Anisotropic Light Beams via Multi-Layer GPU Shading".to_string() },
                        font_size: 13.0,
                        font: { font.clone() },
                        color: Color::srgb(0.55, 0.58, 0.68),
                    }
                ]

                --

                // Top Right Indicators
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    gap: 12.0,
                }
                Children [
                    @tech_badge("2D Radial Diffusion", Color::srgb(0.20, 0.65, 1.0), font.clone())
                    --
                    @tech_badge("Anisotropic Beams", Color::srgb(1.0, 0.25, 0.35), font.clone())
                ]
            ]

            --

            // --- Main Showcase Gallery: Two Smartphone Glass Slates Side-by-Side ---
            UNode {
                width: UVal::Percent(1.0),
                max_width: 1040.0,
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                gap: 56.0,
            }
            Children [
                // 1. Left Device: Aurora Mesh Gradient
                @aurora_phone_slate(font.clone(), icon_font.clone())
                --
                // 2. Right Device: Prism Light Beams & Ribbed Scanlines
                @prism_phone_slate(font.clone(), icon_font.clone())
            ]

            --

            // --- Footer Info HUD ---
            UNode {
                width: UVal::Percent(1.0),
                max_width: 1040.0,
                background_color: Color::srgba(0.06, 0.07, 0.09, 0.70),
                border_radius: UCornerRadius::all(10.0),
                padding: USides::axes(20.0, 10.0),
            }
            UBorder {
                color: Color::srgba(1.0, 1.0, 1.0, 0.06),
                width: 1.0,
                radius: UCornerRadius::all(10.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
            }
            Children [
                UText {
                    text: { "• Left: 4 Overlapping Radial Blobs with Dynamic Harmonic Swirl   • Right: Slicing Dual Linear Beams with Micro-Ribbing".to_string() },
                    font_size: 12.0,
                    font: { font.clone() },
                    color: Color::srgb(0.50, 0.55, 0.65),
                }
                --
                UText {
                    text: { "Running in real-time GPU SDF shaders at native 60 FPS".to_string() },
                    font_size: 12.0,
                    font: { font.clone() },
                    color: Color::srgb(0.35, 0.75, 0.45),
                }
            ]
        ]
    }
}

/// Small info badge in the header.
fn tech_badge(title: &'static str, accent: Color, font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            background_color: Color::srgba(accent.to_srgba().red, accent.to_srgba().green, accent.to_srgba().blue, 0.12),
            border_radius: UCornerRadius::all(8.0),
            padding: USides::axes(12.0, 6.0),
        }
        UBorder {
            color: Color::srgba(accent.to_srgba().red, accent.to_srgba().green, accent.to_srgba().blue, 0.30),
            width: 1.0,
            radius: UCornerRadius::all(8.0),
        }
        Children [
            UText {
                text: { title.to_string() },
                font_size: 12.0,
                font: { font },
                color: accent,
            }
        ]
    }
}

// ---------------------------------------------------------------------------
// Left Device: Aurora Mesh Gradient Phone Slate (Image 2 Recreation)
// ---------------------------------------------------------------------------

fn aurora_phone_slate(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        // Outer device shadow container
        UNode {
            width: UVal::Px(340.0),
            height: UVal::Px(640.0),
            background_color: Color::srgb(0.015, 0.015, 0.02),
            border_radius: UCornerRadius::all(36.0),
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.15),
            width: 2.0,
            radius: UCornerRadius::all(36.0),
        }
        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.75), Vec2::new(0.0, 14.0), 32.0)
        UClip::enabled(true)
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            // ===============================================================
            // Multi-Layer Aurora Background (4 Overlapping Radial Blobs)
            // ===============================================================

            // Blob 1: Deep Electric Blue / Indigo Nebula in the upper-left
            AuroraBlob {
                base_center: Vec2::new(0.18, 0.16),
                base_radius: 0.58,
                speed: 0.85,
                phase: 0.0,
                radius_amp: 0.08,
            }
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            USelf::stretch_full()
            UGradient::radial_custom(
                Color::srgba(0.04, 0.22, 0.96, 0.92),
                Color::NONE,
                Vec2::new(0.18, 0.16),
                0.58,
            )

            --

            // Blob 2: Midnight Violet / Deep Purple Cloud in the center-right
            AuroraBlob {
                base_center: Vec2::new(0.62, 0.38),
                base_radius: 0.65,
                speed: 0.65,
                phase: 1.8,
                radius_amp: 0.06,
            }
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            USelf::stretch_full()
            UGradient::radial_custom(
                Color::srgba(0.38, 0.04, 0.70, 0.55),
                Color::NONE,
                Vec2::new(0.62, 0.38),
                0.65,
            )

            --

            // Blob 3: Vibrant Crimson Fire Flare in the lower-left
            AuroraBlob {
                base_center: Vec2::new(0.12, 0.72),
                base_radius: 0.52,
                speed: 1.10,
                phase: 3.2,
                radius_amp: 0.09,
            }
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            USelf::stretch_full()
            UGradient::radial_custom(
                Color::srgba(0.98, 0.02, 0.14, 0.96),
                Color::NONE,
                Vec2::new(0.12, 0.72),
                0.52,
            )

            --

            // Blob 4: Intense Warm Tangerine / Amber Hearth at the bottom edge
            AuroraBlob {
                base_center: Vec2::new(0.18, 0.98),
                base_radius: 0.40,
                speed: 0.95,
                phase: 4.5,
                radius_amp: 0.07,
            }
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            USelf::stretch_full()
            UGradient::radial_custom(
                Color::srgba(1.0, 0.40, 0.06, 0.95),
                Color::NONE,
                Vec2::new(0.18, 0.98),
                0.40,
            )

            --

            // ===============================================================
            // Foreground UI Overlay (Glassmorphism Clock, Island, and Controls)
            // ===============================================================

            // Top Status Bar with Dynamic Island
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
                UText {
                    text: { "09:41".to_string() },
                    font_size: 13.0,
                    font: { font.clone() },
                    color: Color::srgb(0.95, 0.96, 0.98),
                }

                --

                // Pill Notch (Dynamic Island)
                UNode {
                    width: UVal::Px(84.0),
                    height: UVal::Px(20.0),
                    background_color: Color::srgb(0.0, 0.0, 0.0),
                    border_radius: UCornerRadius::all(10.0),
                }
                UBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.08),
                    width: 1.0,
                    radius: UCornerRadius::all(10.0),
                }

                --

                // Right Status Icons
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    gap: 6.0,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: { Icon::WIFI.to_string() },
                        font_size: 13.0,
                        font: { icon_font.clone() },
                        color: Color::srgb(0.95, 0.96, 0.98),
                    }
                    --
                    UText {
                        text: { Icon::BATTERY_FULL.to_string() },
                        font_size: 13.0,
                        font: { icon_font.clone() },
                        color: Color::srgb(0.95, 0.96, 0.98),
                    }
                ]
            ]

            --

            // Large Lockscreen Clock
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                align_items: UAlignItems::Center,
                gap: 4.0,
            }
            Children [
                UText {
                    text: { Icon::LOCK.to_string() },
                    font_size: 14.0,
                    font: { icon_font.clone() },
                    color: Color::srgba(1.0, 1.0, 1.0, 0.65),
                }
                --
                UText {
                    text: { "09:41".to_string() },
                    font_size: 54.0,
                    font: { font.clone() },
                    color: Color::srgba(1.0, 1.0, 1.0, 0.95),
                }
                --
                UText {
                    text: { "Thursday, November 5".to_string() },
                    font_size: 13.0,
                    font: { font.clone() },
                    color: Color::srgba(1.0, 1.0, 1.0, 0.70),
                }
            ]

            --

            // Glassmorphism Floating Audio Player Card
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::srgba(0.08, 0.08, 0.14, 0.45),
                border_radius: UCornerRadius::all(20.0),
                padding: USides::all(14.0),
            }
            UBorder {
                color: Color::srgba(1.0, 1.0, 1.0, 0.14),
                width: 1.0,
                radius: UCornerRadius::all(20.0),
            }
            UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.40), Vec2::new(0.0, 6.0), 16.0)
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                justify_content: UJustifyContent::SpaceBetween,
            }
            Children [
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    align_items: UAlignItems::Center,
                    gap: 12.0,
                }
                Children [
                    // Album art square
                    UNode {
                        width: UVal::Px(42.0),
                        height: UVal::Px(42.0),
                        background_color: Color::srgb(0.95, 0.20, 0.15),
                        border_radius: UCornerRadius::all(10.0),
                    }
                    UGradient::linear(
                        Color::srgb(1.0, 0.35, 0.10),
                        Color::srgb(0.20, 0.10, 0.90),
                        FRAC_PI_4,
                    )
                    --
                    UNode::default()
                    ULayout {
                        display: UDisplay::Flex,
                        flex_direction: UFlexDirection::Column,
                        gap: 2.0,
                    }
                    Children [
                        UText {
                            text: { "Solar Flare".to_string() },
                            font_size: 13.0,
                            font: { font.clone() },
                            color: Color::srgb(1.0, 1.0, 1.0),
                        }
                        --
                        UText {
                            text: { "Luma Spatial Audio".to_string() },
                            font_size: 11.0,
                            font: { font.clone() },
                            color: Color::srgba(1.0, 1.0, 1.0, 0.60),
                        }
                    ]
                ]

                --

                // Play icon button
                UNode {
                    width: UVal::Px(34.0),
                    height: UVal::Px(34.0),
                    background_color: Color::srgba(1.0, 1.0, 1.0, 0.18),
                    border_radius: UCornerRadius::all(17.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: { Icon::PLAY.to_string() },
                        font_size: 15.0,
                        font: { icon_font.clone() },
                        color: Color::srgb(1.0, 1.0, 1.0),
                    }
                ]
            ]

            --

            // Bottom Shortcuts (Flashlight & Camera Glass Buttons)
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
                UNode {
                    width: UVal::Px(44.0),
                    height: UVal::Px(44.0),
                    background_color: Color::srgba(0.08, 0.08, 0.12, 0.50),
                    border_radius: UCornerRadius::all(22.0),
                }
                UBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.12),
                    width: 1.0,
                    radius: UCornerRadius::all(22.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: { Icon::FLASHLIGHT.to_string() },
                        font_size: 18.0,
                        font: { icon_font.clone() },
                        color: Color::srgb(1.0, 1.0, 1.0),
                    }
                ]

                --

                UNode {
                    width: UVal::Px(44.0),
                    height: UVal::Px(44.0),
                    background_color: Color::srgba(0.08, 0.08, 0.12, 0.50),
                    border_radius: UCornerRadius::all(22.0),
                }
                UBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.12),
                    width: 1.0,
                    radius: UCornerRadius::all(22.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: { Icon::CAMERA.to_string() },
                        font_size: 18.0,
                        font: { icon_font },
                        color: Color::srgb(1.0, 1.0, 1.0),
                    }
                ]
            ]
        ]
    }
}

// ---------------------------------------------------------------------------
// Right Device: Prism Light Beams & Ribbed Scanlines (Image 1 Recreation)
// ---------------------------------------------------------------------------

fn prism_phone_slate(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        // Outer device shadow container
        UNode {
            width: UVal::Px(340.0),
            height: UVal::Px(640.0),
            background_color: Color::srgb(0.005, 0.005, 0.008),
            border_radius: UCornerRadius::all(36.0),
            padding: USides::all(20.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.15),
            width: 2.0,
            radius: UCornerRadius::all(36.0),
        }
        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.75), Vec2::new(0.0, 14.0), 32.0)
        UClip::enabled(true)
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            // ===============================================================
            // Multi-Layer Anisotropic Beams & Pleated Ribbed Diffraction
            // ===============================================================

            // Beam 1: Deep Cyan / Electric Blue Beam slicing down diagonally
            PrismSweep {
                speed: 0.40,
                base_offset: 0.0,
            }
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            USelf::stretch_full()
            UGradient::linear_stops(
                1.18, // ~68 degrees angle
                [
                    UGradientStop::new(0.0, Color::NONE),
                    UGradientStop::new(0.18, Color::srgba(0.0, 0.45, 1.0, 0.98)),
                    UGradientStop::new(0.34, Color::srgba(0.10, 0.25, 0.90, 0.85)),
                    UGradientStop::new(0.48, Color::srgba(0.35, 0.05, 0.70, 0.45)),
                    UGradientStop::new(0.64, Color::NONE),
                ],
            )

            --

            // Beam 2: Razor Slicing Crimson Red Beam cutting from right
            PrismSweep {
                speed: 0.30,
                base_offset: 0.0,
            }
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            USelf::stretch_full()
            UGradient::linear_stops(
                2.42, // ~138 degrees angle
                [
                    UGradientStop::new(0.0, Color::NONE),
                    UGradientStop::new(0.42, Color::NONE),
                    UGradientStop::new(0.49, Color::srgba(1.0, 0.02, 0.08, 0.98)),
                    UGradientStop::new(0.58, Color::srgba(0.88, 0.01, 0.12, 0.70)),
                    UGradientStop::new(0.72, Color::NONE),
                ],
            )

            --

            // Layer 3: Micro-Ribbed Scanlines (Pleated Texture from Image 1)
            // Simulates fine diffraction grating lines across the violet interference zone
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::NONE,
            }
            USelf::stretch_full()
            UGradient::linear_stops(
                1.18,
                [
                    UGradientStop::new(0.0, Color::NONE),
                    UGradientStop::new(0.38, Color::srgba(0.05, 0.15, 0.50, 0.0)),
                    UGradientStop::new(0.42, Color::srgba(0.20, 0.35, 0.90, 0.28)),
                    UGradientStop::new(0.46, Color::srgba(0.05, 0.10, 0.40, 0.0)),
                    UGradientStop::new(0.50, Color::srgba(0.30, 0.20, 0.80, 0.30)),
                    UGradientStop::new(0.54, Color::srgba(0.05, 0.10, 0.40, 0.0)),
                    UGradientStop::new(0.58, Color::srgba(0.40, 0.15, 0.75, 0.25)),
                    UGradientStop::new(0.64, Color::NONE),
                ],
            ).stepped()

            --

            // ===============================================================
            // Foreground UI Overlay (Cyberpunk / Minimalist HUD)
            // ===============================================================

            // Top Status Bar
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
                UText {
                    text: { "09:41".to_string() },
                    font_size: 13.0,
                    font: { font.clone() },
                    color: Color::srgb(0.95, 0.96, 0.98),
                }

                --

                // Pill Notch
                UNode {
                    width: UVal::Px(84.0),
                    height: UVal::Px(20.0),
                    background_color: Color::srgb(0.0, 0.0, 0.0),
                    border_radius: UCornerRadius::all(10.0),
                }
                UBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.08),
                    width: 1.0,
                    radius: UCornerRadius::all(10.0),
                }

                --

                // Right Status Icons
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    gap: 6.0,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: { Icon::WIFI.to_string() },
                        font_size: 13.0,
                        font: { icon_font.clone() },
                        color: Color::srgb(0.95, 0.96, 0.98),
                    }
                    --
                    UText {
                        text: { Icon::BATTERY_FULL.to_string() },
                        font_size: 13.0,
                        font: { icon_font.clone() },
                        color: Color::srgb(0.95, 0.96, 0.98),
                    }
                ]
            ]

            --

            // Minimalist Modern Clock
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                align_items: UAlignItems::Center,
                gap: 4.0,
            }
            Children [
                UText {
                    text: { "PRISM SPECTRA".to_string() },
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.20, 0.65, 1.0),
                }
                --
                UText {
                    text: { "09:41".to_string() },
                    font_size: 54.0,
                    font: { font.clone() },
                    color: Color::srgba(1.0, 1.0, 1.0, 0.95),
                }
                --
                UText {
                    text: { "Thursday, November 5".to_string() },
                    font_size: 13.0,
                    font: { font.clone() },
                    color: Color::srgba(1.0, 1.0, 1.0, 0.70),
                }
            ]

            --

            // Sleek Cybernetic Notification Pill
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::srgba(0.04, 0.05, 0.08, 0.60),
                border_radius: UCornerRadius::all(18.0),
                padding: USides::axes(16.0, 12.0),
            }
            UBorder {
                color: Color::srgba(0.20, 0.50, 1.0, 0.25),
                width: 1.0,
                radius: UCornerRadius::all(18.0),
            }
            UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.45), Vec2::new(0.0, 6.0), 16.0)
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 12.0,
            }
            Children [
                UNode {
                    width: UVal::Px(10.0),
                    height: UVal::Px(10.0),
                    background_color: Color::srgb(0.20, 0.70, 1.0),
                    border_radius: UCornerRadius::all(5.0),
                }
                UShadow::glow(Color::srgb(0.20, 0.70, 1.0), 8.0)
                --
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 2.0,
                }
                Children [
                    UText {
                        text: { "Interference Grating Active".to_string() },
                        font_size: 12.0,
                        font: { font.clone() },
                        color: Color::srgb(0.95, 0.98, 1.0),
                    }
                    --
                    UText {
                        text: { "Birefringent Beam Synthesis: OK".to_string() },
                        font_size: 10.0,
                        font: { font.clone() },
                        color: Color::srgba(1.0, 1.0, 1.0, 0.50),
                    }
                ]
            ]

            --

            // Bottom Fingerprint Unlock Prompt
            UNode {
                width: UVal::Percent(1.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                align_items: UAlignItems::Center,
                gap: 8.0,
            }
            Children [
                UNode {
                    width: UVal::Px(48.0),
                    height: UVal::Px(48.0),
                    background_color: Color::srgba(1.0, 1.0, 1.0, 0.06),
                    border_radius: UCornerRadius::all(24.0),
                }
                UBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.15),
                    width: 1.0,
                    radius: UCornerRadius::all(24.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: { Icon::APERTURE.to_string() },
                        font_size: 22.0,
                        font: { icon_font },
                        color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                    }
                ]
                --
                UText {
                    text: { "Swipe up or hold to unlock".to_string() },
                    font_size: 11.0,
                    font: { font },
                    color: Color::srgba(1.0, 1.0, 1.0, 0.45),
                }
            ]
        ]
    }
}

// ---------------------------------------------------------------------------
// Animation Systems
// ---------------------------------------------------------------------------

/// Gently drifts the 4 radial aurora blobs to simulate living curtains of light.
fn animate_aurora_drift(time: Res<Time>, mut query: Query<(&AuroraBlob, &mut UGradient)>) {
    let t = time.elapsed_secs();
    for (blob, mut grad) in &mut query {
        if let UGradientKind::Radial { ref mut center, ref mut radius } = grad.kind {
            // Harmonic 2D drift
            let dx = (t * blob.speed + blob.phase).sin() * 0.05;
            let dy = (t * blob.speed * 0.8 + blob.phase).cos() * 0.05;
            let dr = (t * blob.speed * 1.2 + blob.phase).sin() * blob.radius_amp;

            *center = blob.base_center + Vec2::new(dx, dy);
            *radius = (blob.base_radius + dr).clamp(0.20, 0.95);
        }
    }
}

/// Gently sweeps the prism light beams to create an authentic shimmer/laser drift.
fn animate_prism_sweep(time: Res<Time>, mut query: Query<(&PrismSweep, &mut UGradient)>) {
    let t = time.elapsed_secs();
    for (sweep, mut grad) in &mut query {
        grad.offset = (t * sweep.speed * 0.15).sin() * 0.06 + sweep.base_offset;
    }
}
