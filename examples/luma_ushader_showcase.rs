//! # Luma UShader Showcase (`UShader`)
//!
//! Demonstrates the [`UShader`] component for replacing or injecting custom GPU
//! fragment shaders directly into [`UNode`] backgrounds while preserving all
//! Univis SDF features (rounded/chamfered corners, borders, inner glows,
//! outer shadows, and hierarchical scroll clipping).
//!
//! Showcases:
//! 1. **Built-in Procedural GPU Presets**:
//!    - [`UShader::aurora_beams`] (Intersecting diagonal prism light beams + optical ribs)
//!    - [`UShader::mesh_gradient`] (Domain-warped 4-point organic fluid mesh gradient)
//!    - [`UShader::dark_horizon`] (Pitch-black top void + rising multi-dome atmospheric glow)
//!    - [`UShader::plasma_energy`] (Multi-octave FBM nebula filaments)
//!    - [`UShader::cyber_matrix`] (Animated holographic radar grid + active data cells)
//!    - [`UShader::liquid_chrome`] (Iridescent metallic wave interference + specular highlights)
//! 2. **Inline Custom WGSL Injection (`UShader::from_wgsl`)**:
//!    - Custom retro synthwave sun & perspective grid written in a few lines of WGSL.
//!    - Additive (`UShaderBlendMode::Add`) optical rings injected over a `UGradient` node.

use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Component, Clone, Copy, Default)]
struct MouseReadoutLabel;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI — UShader GPU Background Injection Showcase".into(),
                resolution: (1440, 940).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_ushader_showcase)
        .add_systems(Update, update_mouse_readout)
        .run();
}

fn update_mouse_readout(
    windows: Query<&Window>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut labels: Query<&mut UText, With<MouseReadoutLabel>>,
) {
    let cursor = windows.iter().find_map(|w| w.cursor_position());
    let pressed = mouse_buttons.pressed(MouseButton::Left);

    for mut text in &mut labels {
        let next = match (cursor, pressed) {
            (Some(pos), true) => format!("MOUSE ({:.0}, {:.0}) • CLICK SHOCKWAVE", pos.x, pos.y),
            (Some(pos), false) => format!("MOUSE ({:.0}, {:.0}) • HOVER TRACKING", pos.x, pos.y),
            (None, _) => "MOVE MOUSE / CLICK CARDS".to_string(),
        };
        if text.text != next {
            text.text = next;
        }
    }
}

fn setup_ushader_showcase(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(ushader_showcase_scene(&theme));
}

fn ushader_showcase_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.025, 0.028, 0.042),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
        }
        Children [
            // =================================================================
            // Top Header Bar
            // =================================================================
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(74.0),
                padding: USides::axes(36.0, 0.0),
                background_color: Color::srgba(0.04, 0.045, 0.07, 0.94),
            }
            UBorder {
                color: Color::srgba(0.28, 0.36, 0.62, 0.28),
                width: 1.0,
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
            }
            Children [
                UNode { background_color: Color::NONE }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    align_items: UAlignItems::Center,
                    gap: 14.0,
                }
                Children [
                    // Animated shader badge icon in header
                    UNode {
                        width: UVal::Px(40.0),
                        height: UVal::Px(40.0),
                        border_radius: UCornerRadius::all(11.0),
                    }
                    UShader::mesh_gradient(
                        Color::srgb(0.08, 0.45, 1.0),
                        Color::srgb(0.98, 0.12, 0.45),
                        Color::srgb(0.0, 0.92, 0.78),
                        Color::srgb(0.55, 0.15, 0.98),
                    )
                    UBorder {
                        color: Color::srgba(1.0, 1.0, 1.0, 0.35),
                        width: 1.0,
                        radius: UCornerRadius::all(11.0),
                    }
                    ULayout {
                        display: UDisplay::Flex,
                        justify_content: UJustifyContent::Center,
                        align_items: UAlignItems::Center,
                    }
                    Children [
                        UText {
                            text: { "FX".to_string() },
                            font_size: 14.0,
                            font: { font.clone() },
                            color: Color::WHITE,
                            overflow: UTextOverflow::Visible,
                        }
                    ]

                    --

                    UNode { background_color: Color::NONE }
                    ULayout {
                        display: UDisplay::Flex,
                        flex_direction: UFlexDirection::Column,
                        gap: 2.0,
                    }
                    Children [
                        UText {
                            text: { "UShader — GPU Fragment Shader Injection".to_string() },
                            font_size: 20.0,
                            font: { font.clone() },
                            color: Color::srgb(0.96, 0.97, 1.0),
                            overflow: UTextOverflow::Visible,
                        }
                        --
                        UText {
                            text: { "Single-node WGSL backgrounds with SDF rounded/cut corners, borders, inner glow & clipping".to_string() },
                            font_size: 12.0,
                            font: { font.clone() },
                            color: Color::srgb(0.56, 0.62, 0.76),
                            overflow: UTextOverflow::Visible,
                        }
                    ]
                ]

                --

                // Right live mouse tracking pill
                UNode {
                    width: UVal::Px(255.0),
                    height: UVal::Px(34.0),
                    padding: USides::axes(14.0, 0.0),
                    border_radius: UCornerRadius::all(999.0),
                    background_color: Color::srgba(0.0, 0.78, 0.68, 0.18),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                UBorder {
                    color: Color::srgba(0.10, 0.96, 0.82, 0.50),
                    width: 1.0,
                    radius: UCornerRadius::all(999.0),
                }
                Children [
                    MouseReadoutLabel
                    UText {
                        text: { "MOVE MOUSE / CLICK CARDS".to_string() },
                        font_size: 11.0,
                        font: { font.clone() },
                        color: Color::srgb(0.42, 0.98, 0.88),
                        overflow: UTextOverflow::Visible,
                    }
                ]
            ]

            --

            // =================================================================
            // Scrollable Showcase Body
            // =================================================================
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Flex(1.0),
                background_color: Color::NONE,
            }
            UScrollContainer::vertical()
            UScrollKineticDrag::default()
            Children [
                UNode {
                    width: UVal::Percent(1.0),
                    padding: USides::axes(36.0, 26.0),
                    background_color: Color::NONE,
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 24.0,
                }
                Children [
                    // ---------------------------------------------------------
                    // Section 0: Interactive Mouse-Reactive Shaders (2-Column Hero Row)
                    // ---------------------------------------------------------
                    UNode {
                        width: UVal::Percent(1.0),
                        background_color: Color::NONE,
                    }
                    ULayout {
                        display: UDisplay::Grid,
                        grid_template_columns: vec![
                            UTrackSize::Fr(1.0),
                            UTrackSize::Fr(1.0),
                        ],
                        gap: 22.0,
                    }
                    Children [
                        // Interactive Card A: Magnetic Cursor Spotlight + Cyber Grid Reveal
                        @shader_preset_card(
                            "MOUSE INTERACTIVE • HOVER & CLICK",
                            "interactive_spotlight()",
                            "Tracks your mouse cursor (luma_cursor_uv) to reveal a hidden cybernetic micro-grid and emits a shockwave ring on click.",
                            UShader::interactive_spotlight(
                                Color::srgb(0.05, 0.68, 1.0),
                                Color::srgb(0.0, 1.0, 0.75),
                                Color::srgb(0.015, 0.02, 0.045),
                            ),
                            Color::srgba(0.0, 0.92, 0.85, 0.55),
                            UShapeMode::Round,
                            font.clone(),
                        )

                        --

                        // Interactive Card B: Gravitational Fluid Ripple Vortex
                        @shader_preset_card(
                            "MOUSE INTERACTIVE • FLUID VORTEX",
                            "interactive_fluid_ripple()",
                            "Bends domain-warped FBM plasma around the live pointer with gravitational swirl and click wave bursts.",
                            UShader::interactive_fluid_ripple(
                                Color::srgb(0.18, 0.06, 0.58),
                                Color::srgb(0.96, 0.10, 0.48),
                                Color::srgb(0.25, 0.92, 1.0),
                            )
                            .with_time_offset(1.7),
                            Color::srgba(1.0, 0.32, 0.68, 0.55),
                            UShapeMode::Round,
                            font.clone(),
                        )
                    ]

                    --

                    // ---------------------------------------------------------
                    // Section 1: Built-in Procedural Shader Presets (3x2 Grid)
                    // ---------------------------------------------------------
                    UNode {
                        width: UVal::Percent(1.0),
                        background_color: Color::NONE,
                    }
                    ULayout {
                        display: UDisplay::Grid,
                        grid_template_columns: vec![
                            UTrackSize::Fr(1.0),
                            UTrackSize::Fr(1.0),
                            UTrackSize::Fr(1.0),
                        ],
                        gap: 22.0,
                    }
                    Children [
                        // Card 1: Aurora Prism Beams
                        @shader_preset_card(
                            "01 • AURORA BEAMS",
                            "aurora_beams()",
                            "Intersecting cobalt & crimson light beams with micro-ribbed optical scanlines on a single UNode.",
                            UShader::aurora_beams(
                                Color::srgb(0.04, 0.38, 1.0),
                                Color::srgb(1.0, 0.08, 0.02),
                                Color::srgb(0.55, 0.15, 1.0),
                            )
                            .with_time_offset(1.2),
                            Color::srgba(0.25, 0.55, 1.0, 0.45),
                            UShapeMode::Round,
                            font.clone(),
                        )

                        --

                        // Card 2: Domain-Warped Fluid Mesh Gradient
                        @shader_preset_card(
                            "02 • MESH GRADIENT",
                            "mesh_gradient()",
                            "True 2D domain-warped FBM fluid interpolation across 4 dynamic color anchors on a single quad.",
                            UShader::mesh_gradient(
                                Color::srgb(0.02, 0.12, 0.68),
                                Color::srgb(0.96, 0.06, 0.28),
                                Color::srgb(1.0, 0.42, 0.04),
                                Color::srgb(0.18, 0.04, 0.48),
                            )
                            .with_time_offset(2.5),
                            Color::srgba(1.0, 0.35, 0.52, 0.45),
                            UShapeMode::Round,
                            font.clone(),
                        )

                        --

                        // Card 3: Dark Horizon (Pitch-black top + rising bottom glow)
                        @shader_preset_card(
                            "03 • DARK HORIZON",
                            "dark_horizon()",
                            "Pitch-black upper void (0.0..0.46) with multi-dome atmospheric aurora rising from the bottom edge.",
                            UShader::dark_horizon(
                                Color::srgb(0.0, 0.88, 0.98),
                                Color::srgb(1.0, 0.72, 0.12),
                                Color::srgb(0.95, 0.14, 0.58),
                            )
                            .with_time_offset(0.8),
                            Color::srgba(0.0, 0.85, 0.98, 0.42),
                            UShapeMode::Round,
                            font.clone(),
                        )

                        --

                        // Card 4: Plasma Energy Nebula
                        @shader_preset_card(
                            "04 • PLASMA NEBULA",
                            "plasma_energy()",
                            "Multi-octave fractional Brownian motion (luma_fbm) with glowing filaments and deep space contrast.",
                            UShader::plasma_energy(
                                Color::srgb(0.42, 0.12, 0.96),
                                Color::srgb(0.0, 0.96, 0.82),
                                Color::srgb(0.02, 0.02, 0.06),
                            )
                            .with_time_offset(3.4),
                            Color::srgba(0.0, 0.92, 0.82, 0.42),
                            UShapeMode::Round,
                            font.clone(),
                        )

                        --

                        // Card 5: Cyber Matrix Grid (with Cut/Chamfered corners!)
                        @shader_preset_card(
                            "05 • CYBER MATRIX",
                            "cyber_matrix()",
                            "Aspect-corrected sci-fi grid with radar sweep and pulsing data cells inside chamfered UShapeMode::Cut.",
                            UShader::cyber_matrix(
                                Color::srgb(0.0, 0.88, 1.0),
                                Color::srgb(0.15, 1.0, 0.55),
                                Color::srgb(0.02, 0.04, 0.08),
                            )
                            .with_time_offset(1.9),
                            Color::srgba(0.15, 1.0, 0.65, 0.50),
                            UShapeMode::Cut,
                            font.clone(),
                        )

                        --

                        // Card 6: Liquid Chrome Waves
                        @shader_preset_card(
                            "06 • LIQUID CHROME",
                            "liquid_chrome()",
                            "Iterative harmonic wave interference producing metallic ridges and sharp specular highlights.",
                            UShader::liquid_chrome(
                                Color::srgb(0.16, 0.32, 0.92),
                                Color::srgb(0.88, 0.16, 0.62),
                                Color::srgb(0.95, 0.98, 1.0),
                            )
                            .with_time_offset(4.1),
                            Color::srgba(0.75, 0.55, 1.0, 0.45),
                            UShapeMode::Round,
                            font.clone(),
                        )
                    ]

                    --

                    // ---------------------------------------------------------
                    // Section 2: Inline Custom WGSL & Blend Mode Compositing
                    // ---------------------------------------------------------
                    UNode {
                        width: UVal::Percent(1.0),
                        background_color: Color::NONE,
                    }
                    ULayout {
                        display: UDisplay::Grid,
                        grid_template_columns: vec![
                            UTrackSize::Fr(1.0),
                            UTrackSize::Fr(1.0),
                        ],
                        gap: 22.0,
                    }
                    Children [
                        // Left Banner: Inline Custom WGSL Synthwave Sun & Grid
                        UNode {
                            width: UVal::Percent(1.0),
                            height: UVal::Px(248.0),
                            padding: USides::all(22.0),
                            border_radius: UCornerRadius::all(22.0),
                        }
                        UShader::from_wgsl(
                            r#"
                            let aspect = size.x / max(size.y, 1.0);
                            let p = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.42);
                            let d = length(p);

                            // Retro striped sun disc
                            let sun_mask = smoothstep(0.29, 0.275, d);
                            let stripe = step(0.22, fract(uv.y * 22.0 - time * 0.6));
                            let sun_cut = mix(1.0, stripe, smoothstep(0.32, 0.55, uv.y));
                            let sun_col = mix(params[0].rgb, params[1].rgb, clamp((uv.y - 0.15) * 2.5, 0.0, 1.0));

                            // Perspective neon floor grid below horizon
                            let horizon = smoothstep(0.52, 0.54, uv.y);
                            let z = 1.0 / max(uv.y - 0.50, 0.02);
                            let gx = abs(fract(p.x * z * 0.45) - 0.5);
                            let gz = abs(fract(z * 0.35 - time * 1.2) - 0.5);
                            let grid_line = smoothstep(0.06, 0.0, min(gx, gz)) * horizon;

                            var rgb = params[3].rgb;
                            rgb = rgb + params[0].rgb * exp(-d * 3.2) * 0.55;
                            rgb = mix(rgb, sun_col, sun_mask * sun_cut);
                            rgb = rgb + params[2].rgb * grid_line * 0.85;
                            return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
                            "#,
                        )
                        .with_colors([
                            Color::srgb(1.0, 0.12, 0.52),
                            Color::srgb(1.0, 0.78, 0.10),
                            Color::srgb(0.0, 0.92, 1.0),
                            Color::srgb(0.03, 0.02, 0.08),
                        ])
                        UBorder {
                            color: Color::srgba(1.0, 0.25, 0.65, 0.52),
                            width: 1.5,
                            radius: UCornerRadius::all(22.0),
                        }
                        UInnerGlow::new(Color::srgba(1.0, 0.22, 0.62, 0.22), 18.0)
                        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.55), Vec2::new(0.0, 8.0), 18.0)
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Column,
                            justify_content: UJustifyContent::SpaceBetween,
                        }
                        Children [
                            UNode {
                                width: UVal::Content,
                                height: UVal::Content,
                                padding: USides::axes(12.0, 5.0),
                                border_radius: UCornerRadius::all(999.0),
                                background_color: Color::srgba(0.03, 0.03, 0.07, 0.78),
                            }
                            ULayout::default()
                            UBorder {
                                color: Color::srgba(1.0, 0.45, 0.75, 0.45),
                                width: 1.0,
                                radius: UCornerRadius::all(999.0),
                            }
                            Children [
                                UText {
                                    text: { "INLINE WGSL • UShader::from_wgsl(...)".to_string() },
                                    font_size: 11.0,
                                    font: { font.clone() },
                                    color: Color::srgb(1.0, 0.68, 0.86),
                                    overflow: UTextOverflow::Visible,
                                }
                            ]

                            --

                            UNode {
                                width: UVal::Percent(1.0),
                                height: UVal::Content,
                                padding: USides::axes(16.0, 12.0),
                                border_radius: UCornerRadius::all(14.0),
                                background_color: Color::srgba(0.02, 0.025, 0.05, 0.82),
                            }
                            UBorder {
                                color: Color::srgba(1.0, 1.0, 1.0, 0.14),
                                width: 1.0,
                                radius: UCornerRadius::all(14.0),
                            }
                            ULayout {
                                display: UDisplay::Flex,
                                flex_direction: UFlexDirection::Column,
                                gap: 4.0,
                            }
                            Children [
                                UText {
                                    text: { "Custom Synthwave Outrun Shader".to_string() },
                                    font_size: 15.0,
                                    font: { font.clone() },
                                    color: Color::WHITE,
                                    wrap_mode: UTextWrapMode::WrapToParent,
                                    overflow: UTextOverflow::Visible,
                                }
                                --
                                UText {
                                    text: { "Written inline in Rust BSN with aspect-corrected UV math, time animation, and 4 color uniforms.".to_string() },
                                    font_size: 11.5,
                                    font: { font.clone() },
                                    color: Color::srgb(0.74, 0.80, 0.92),
                                    wrap_mode: UTextWrapMode::WrapToParent,
                                    overflow: UTextOverflow::Visible,
                                }
                            ]
                        ]

                        --

                        // Right Banner: Additive Shader Injected Over a Multi-Stop UGradient!
                        UNode {
                            width: UVal::Percent(1.0),
                            height: UVal::Px(248.0),
                            padding: USides::all(22.0),
                            border_radius: UCornerRadius::all(22.0),
                        }
                        UGradient::linear_stops(
                            0.65,
                            vec![
                                (0.0, Color::srgb(0.03, 0.06, 0.18)),
                                (0.5, Color::srgb(0.08, 0.18, 0.42)),
                                (1.0, Color::srgb(0.24, 0.06, 0.36)),
                            ],
                        )
                        UShader::from_wgsl(
                            r#"
                            let aspect = size.x / max(size.y, 1.0);
                            let p = vec2<f32>((uv.x - 0.68) * aspect, uv.y - 0.38);
                            let d = length(p);
                            let rings = pow(0.5 + 0.5 * sin(d * 34.0 - time * 3.2), 4.0) * exp(-d * 2.2);
                            let angle = atan2(p.y, p.x);
                            let rays = pow(0.5 + 0.5 * cos(angle * 6.0 + time * 0.8), 6.0) * exp(-d * 1.8);
                            let col = params[0].rgb * rings * 0.95 + params[1].rgb * rays * 0.75;
                            return vec4<f32>(col, 1.0);
                            "#,
                        )
                        .with_colors([
                            Color::srgb(0.0, 0.95, 1.0),
                            Color::srgb(0.98, 0.32, 0.88),
                        ])
                        .with_blend_mode(UShaderBlendMode::Add)
                        UBorder {
                            color: Color::srgba(0.0, 0.88, 1.0, 0.50),
                            width: 1.5,
                            radius: UCornerRadius::all(22.0),
                        }
                        UInnerGlow::new(Color::srgba(0.0, 0.92, 1.0, 0.22), 18.0)
                        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.55), Vec2::new(0.0, 8.0), 18.0)
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Column,
                            justify_content: UJustifyContent::SpaceBetween,
                        }
                        Children [
                            UNode {
                                width: UVal::Content,
                                height: UVal::Content,
                                padding: USides::axes(12.0, 5.0),
                                border_radius: UCornerRadius::all(999.0),
                                background_color: Color::srgba(0.02, 0.05, 0.10, 0.78),
                            }
                            ULayout::default()
                            UBorder {
                                color: Color::srgba(0.0, 0.90, 1.0, 0.45),
                                width: 1.0,
                                radius: UCornerRadius::all(999.0),
                            }
                            Children [
                                UText {
                                    text: { "ADDITIVE BLEND • UGradient + UShader".to_string() },
                                    font_size: 11.0,
                                    font: { font.clone() },
                                    color: Color::srgb(0.52, 0.94, 1.0),
                                    overflow: UTextOverflow::Visible,
                                }
                            ]

                            --

                            UNode {
                                width: UVal::Percent(1.0),
                                height: UVal::Content,
                                padding: USides::axes(16.0, 12.0),
                                border_radius: UCornerRadius::all(14.0),
                                background_color: Color::srgba(0.02, 0.03, 0.06, 0.82),
                            }
                            UBorder {
                                color: Color::srgba(1.0, 1.0, 1.0, 0.14),
                                width: 1.0,
                                radius: UCornerRadius::all(14.0),
                            }
                            ULayout {
                                display: UDisplay::Flex,
                                flex_direction: UFlexDirection::Column,
                                gap: 4.0,
                            }
                            Children [
                                UText {
                                    text: { "Additive Optical Rings Over Multi-Stop Gradient".to_string() },
                                    font_size: 15.0,
                                    font: { font.clone() },
                                    color: Color::WHITE,
                                    wrap_mode: UTextWrapMode::WrapToParent,
                                    overflow: UTextOverflow::Visible,
                                }
                                --
                                UText {
                                    text: { "Combines a base UGradient with an additive UShader overlay on the same UNode entity.".to_string() },
                                    font_size: 11.5,
                                    font: { font.clone() },
                                    color: Color::srgb(0.74, 0.80, 0.92),
                                    wrap_mode: UTextWrapMode::WrapToParent,
                                    overflow: UTextOverflow::Visible,
                                }
                            ]
                        ]
                    ]
                ]
            ]
        ]
    }
}

fn shader_preset_card(
    badge: &'static str,
    api_call: &'static str,
    description: &'static str,
    shader: UShader,
    border_accent: Color,
    shape_mode: UShapeMode,
    font: Handle<Font>,
) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(252.0),
            padding: USides::all(18.0),
            border_radius: UCornerRadius::all(22.0),
            shape_mode: { shape_mode },
        }
        shader
        UBorder {
            color: { border_accent },
            width: 1.5,
            radius: UCornerRadius::all(22.0),
        }
        UInnerGlow::new(Color::srgba(1.0, 1.0, 1.0, 0.16), 16.0)
        UShadow::drop(Color::srgba(0.0, 0.0, 0.0, 0.55), Vec2::new(0.0, 8.0), 16.0)
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Start,
        }
        Children [
            // Top Badge Pill
            UNode {
                width: UVal::Content,
                height: UVal::Content,
                padding: USides::axes(12.0, 5.0),
                border_radius: UCornerRadius::all(999.0),
                background_color: Color::srgba(0.02, 0.025, 0.05, 0.82),
            }
            ULayout::default()
            UBorder {
                color: { border_accent },
                width: 1.0,
                radius: UCornerRadius::all(999.0),
            }
            Children [
                UText {
                    text: { badge.to_string() },
                    font_size: 10.5,
                    font: { font.clone() },
                    color: Color::srgb(0.94, 0.96, 1.0),
                    overflow: UTextOverflow::Visible,
                }
            ]

            --

            // Bottom Frosted Info Box (API Call + Description)
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Content,
                padding: USides::axes(14.0, 11.0),
                border_radius: UCornerRadius::all(14.0),
                background_color: Color::srgba(0.02, 0.025, 0.045, 0.84),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 4.0,
            }
            UBorder {
                color: Color::srgba(1.0, 1.0, 1.0, 0.14),
                width: 1.0,
                radius: UCornerRadius::all(14.0),
            }
            Children [
                UText {
                    text: { format!("UShader::{api_call}") },
                    font_size: 12.5,
                    font: { font.clone() },
                    color: Color::srgb(0.68, 0.88, 1.0),
                    overflow: UTextOverflow::Visible,
                }
                --
                UText {
                    text: { description.to_string() },
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.84, 0.88, 0.96),
                    wrap_mode: UTextWrapMode::WrapToParent,
                    overflow: UTextOverflow::Visible,
                }
            ]
        ]
    }
}
