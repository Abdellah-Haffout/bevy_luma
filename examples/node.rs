//! Visual Node Design - Blender 4.x & Unreal Engine 5 Node Specifications for Bevy Luma.
//!
//! Demonstrates authentic visual DNA of:
//! 1. Blender 4.x Shader & Geometry Nodes:
//!    - Matte dark-charcoal body with tight 6px corner radius.
//!    - Distinctive category headers (Forest-Green for Shaders, Indigo-Blue for Math).
//!    - Standard Blender socket colors (Green = Shader, Yellow = Color, Sky-Blue = Vector, Gray = Float).
//!    - Inline draggable slider fields (`0.500`) and dropdown selectors.
//!
//! 2. Unreal Engine 5 Blueprints:
//!    - Blueprint navy slate with 8px corner radius and deep drop shadow.
//!    - Iconic White Triangle Execution Pins (`▶`) for code flow.
//!    - Function blue header and Flow-control slate header with target subtitles.
//!    - Standard UE pin colors (Red = Boolean, Gold = Vector, Cyan = Object Reference, Green = Float).
//!    - Multi-axis vector inputs (`[ X: 0.0 | Y: 0.0 | Z: 0.0 ]`).

use bevy::prelude::*;
use bevy_luma::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - Node Models: Blender 4.x & Unreal Engine 5 (Pure Node Anatomy)".into(),
                resolution: (1380, 860).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_node_showcase)
        .run();
}

fn setup_node_showcase(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(node_showcase_scene(&theme));
}

/// Root scene presenting Blender and Unreal Engine node models side by side.
fn node_showcase_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();
    let icon_font = theme.icon.font.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.045, 0.052, 0.075),
            padding: USides::axes(20.0, 16.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
            gap: 14.0,
        }
        Children [
            // Top Header Bar
            @showcase_header(font.clone(), icon_font.clone())
            --
            // Main Columns Area (Blender Column on Left, Unreal Column on Right)
            UNode::default()
            USelf {
                flex_grow: { Some(1.0) },
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Start,
                gap: 36.0,
            }
            Children [
                // ==========================================
                // LEFT SECTION: BLENDER 4.x NODES
                // ==========================================
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    align_items: UAlignItems::Center,
                    gap: 16.0,
                }
                Children [
                    @section_title(
                        "BLENDER 4.x SHADER & GEOMETRY NODES",
                        Color::srgb(0.95, 0.65, 0.25),
                        font.clone(),
                    )
                    --
                    UNode::default()
                    ULayout {
                        display: UDisplay::Flex,
                        flex_direction: UFlexDirection::Row,
                        align_items: UAlignItems::Start,
                        gap: 20.0,
                    }
                    Children [
                        // Blender Model 1: Principled BSDF
                        @blender_principled_bsdf_node(font.clone(), icon_font.clone())
                        --
                        // Blender Model 2: Vector Math
                        @blender_vector_math_node(font.clone(), icon_font.clone())
                    ]
                ]
                --
                // Vertical Divider
                UNode {
                    width: UVal::Px(1.0),
                    height: UVal::Percent(0.9),
                    background_color: Color::srgba(0.25, 0.30, 0.45, 0.3),
                }
                --
                // ==========================================
                // RIGHT SECTION: UNREAL ENGINE 5 BLUEPRINTS
                // ==========================================
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    align_items: UAlignItems::Center,
                    gap: 16.0,
                }
                Children [
                    @section_title(
                        "UNREAL ENGINE 5 BLUEPRINT NODES",
                        Color::srgb(0.25, 0.75, 1.0),
                        font.clone(),
                    )
                    --
                    UNode::default()
                    ULayout {
                        display: UDisplay::Flex,
                        flex_direction: UFlexDirection::Row,
                        align_items: UAlignItems::Start,
                        gap: 20.0,
                    }
                    Children [
                        // UE Model 1: Set Actor Location
                        @ue_set_actor_location_node(font.clone(), icon_font.clone())
                        --
                        // UE Model 2: Branch (Flow Control)
                        @ue_branch_node(font.clone(), icon_font.clone())
                    ]
                ]
            ]
            --
            // Bottom Footer
            @showcase_footer(font, icon_font)
        ]
    }
}

/// Section title banner.
fn section_title(text: &str, accent: Color, font: Handle<Font>) -> impl Scene {
    let text = text.to_string();
    bsn! {
        UNode {
            background_color: Color::srgba(0.08, 0.10, 0.16, 0.9),
            border_radius: UCornerRadius::all(6.0),
            padding: USides::axes(14.0, 4.0),
        }
        UBorder {
            color: { accent.with_alpha(0.35) },
            width: 1.0,
            radius: UCornerRadius::all(6.0),
            offset: 0.0,
        }
        Children [
            (UText {
                text,
                font_size: 11.5,
                font: { font },
                color: accent,
            })
        ]
    }
}

/// Top studio branding header.
fn showcase_header(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(46.0),
            background_color: Color::srgba(0.08, 0.09, 0.14, 0.85),
            border_radius: UCornerRadius::all(10.0),
            padding: USides::axes(16.0, 8.0),
        }
        UBorder {
            color: Color::srgba(0.25, 0.30, 0.45, 0.35),
            width: 1.0,
            radius: UCornerRadius::all(10.0),
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    align_items: UAlignItems::Center,
                    gap: 10.0,
                }
                Children [
                    (UText {
                        text: { Icon::CODESANDBOX.to_string() },
                        font_size: 18.0,
                        font: { icon_font },
                        color: Color::srgb(0.35, 0.75, 1.0),
                    }),
                    (UText {
                        text: { "LUMA NODE DESIGN SYSTEMS".to_string() },
                        font_size: 13.5,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (
                        UNode {
                            background_color: Color::srgba(0.2, 0.5, 0.9, 0.2),
                            border_radius: UCornerRadius::all(4.0),
                            padding: USides::axes(6.0, 2.0),
                        }
                        Children [
                            (UText {
                                text: { "BLENDER vs UNREAL ENGINE".to_string() },
                                font_size: 9.0,
                                font: { font.clone() },
                                color: Color::srgb(0.4, 0.8, 1.0),
                            })
                        ]
                    ),
                ]
            ),
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    align_items: UAlignItems::Center,
                    gap: 14.0,
                }
                Children [
                    @legend_chip("Shader (Green)", Color::srgb(0.38, 0.85, 0.45), font.clone())
                    --
                    @legend_chip("Color (Yellow)", Color::srgb(0.92, 0.82, 0.25), font.clone())
                    --
                    @legend_chip("Vector (Blue)", Color::srgb(0.35, 0.65, 0.95), font.clone())
                    --
                    @legend_chip("Exec Pin (White ▶)", Color::WHITE, font)
                ]
            )
        ]
    }
}

/// Small legend chip in header.
fn legend_chip(label: &str, color: Color, font: Handle<Font>) -> impl Scene {
    let label = label.to_string();
    bsn! {
        UNode::default()
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            align_items: UAlignItems::Center,
            gap: 5.0,
        }
        Children [
            (
                UNode {
                    width: UVal::Px(7.0),
                    height: UVal::Px(7.0),
                    background_color: color,
                    border_radius: UCornerRadius::all(3.5),
                }
            ),
            (UText {
                text: label,
                font_size: 10.5,
                font: { font },
                color: Color::srgb(0.65, 0.70, 0.80),
            })
        ]
    }
}

// ============================================================================
// PART 1: BLENDER 4.x NODE SPECIFICATIONS
// ============================================================================

/// Blender 4.x Model A: Principled BSDF Surface Shader Node.
fn blender_principled_bsdf_node(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    let yellow = Color::srgb(0.92, 0.82, 0.25); // Color
    let gray = Color::srgb(0.68, 0.68, 0.72);   // Float
    let sky = Color::srgb(0.35, 0.65, 0.95);    // Vector
    let green = Color::srgb(0.38, 0.85, 0.45);  // Shader

    bsn! {
        UNode {
            width: UVal::Px(265.0),
            background_color: Color::srgb(0.14, 0.14, 0.15),
            border_radius: UCornerRadius::all(6.0),
        }
        UBorder {
            color: Color::srgba(0.08, 0.08, 0.09, 0.9),
            width: 1.0,
            radius: UCornerRadius::all(6.0),
            offset: 0.0,
        }
        UShadow {
            color: Color::srgba(0.0, 0.0, 0.0, 0.55),
            offset: Vec2::new(0.0, 6.0),
            blur: 16.0,
            spread: 0.0,
            is_inner: false,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
        }
        Children [
            // Blender Forest-Green Header
            @blender_header(
                "Principled BSDF",
                Color::srgb(0.18, 0.44, 0.28),
                font.clone(),
                icon_font.clone(),
            )
            --
            // Blender Body
            UNode {
                padding: USides::axes(10.0, 8.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 8.0,
            }
            Children [
                // Distribution Mode Dropdown
                @blender_dropdown("Distribution: GGX", icon_font, font.clone())
                --
                // Row 1: Base Color Input (Yellow) & BSDF Output (Green)
                @blender_dual_row(
                    "Base Color", yellow, None, true,
                    "BSDF", green, true,
                    font.clone(),
                )
                --
                // Row 2: Subsurface Weight (Float)
                @blender_input_row("Subsurface", gray, Some("0.000"), false, font.clone())
                --
                // Row 3: Metallic (Float) with inline slider field
                @blender_input_row("Metallic", gray, Some("0.000"), false, font.clone())
                --
                // Row 4: Roughness (Float)
                @blender_input_row("Roughness", gray, Some("0.500"), false, font.clone())
                --
                // Row 5: IOR (Float)
                @blender_input_row("IOR", gray, Some("1.450"), false, font.clone())
                --
                // Row 6: Alpha (Float)
                @blender_input_row("Alpha", gray, Some("1.000"), false, font.clone())
                --
                // Row 7: Normal Input (Vector / Sky Blue)
                @blender_input_row("Normal", sky, None, true, font)
            ]
        ]
    }
}

/// Blender 4.x Model B: Vector Math Converter Node.
fn blender_vector_math_node(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    let sky = Color::srgb(0.35, 0.65, 0.95);  // Vector
    let gray = Color::srgb(0.68, 0.68, 0.72); // Float

    bsn! {
        UNode {
            width: UVal::Px(250.0),
            background_color: Color::srgb(0.14, 0.14, 0.15),
            border_radius: UCornerRadius::all(6.0),
        }
        UBorder {
            color: Color::srgba(0.08, 0.08, 0.09, 0.9),
            width: 1.0,
            radius: UCornerRadius::all(6.0),
            offset: 0.0,
        }
        UShadow {
            color: Color::srgba(0.0, 0.0, 0.0, 0.55),
            offset: Vec2::new(0.0, 6.0),
            blur: 16.0,
            spread: 0.0,
            is_inner: false,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
        }
        Children [
            // Blender Dark-Blue Header (Vector Converter)
            @blender_header(
                "Vector Math",
                Color::srgb(0.18, 0.28, 0.52),
                font.clone(),
                icon_font.clone(),
            )
            --
            // Blender Body
            UNode {
                padding: USides::axes(10.0, 8.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 8.0,
            }
            Children [
                // Operation Dropdown
                @blender_dropdown("Operation: Multiply", icon_font, font.clone())
                --
                // Row 1: Vector Input 1 & Vector Output
                @blender_dual_row(
                    "Vector", sky, None, true,
                    "Vector", sky, true,
                    font.clone(),
                )
                --
                // Row 2: Vector Input 2 & Float Value Output
                @blender_dual_row(
                    "Vector", sky, Some("1.000"), false,
                    "Value", gray, false,
                    font.clone(),
                )
                --
                // Row 3: Multiplier Scale Input (Float)
                @blender_input_row("Scale", gray, Some("1.000"), false, font)
            ]
        ]
    }
}

/// Standardized Blender Header with collapse chevron and flat category color bar.
fn blender_header(
    title: &str,
    header_color: Color,
    font: Handle<Font>,
    icon_font: Handle<Font>,
) -> impl Scene {
    let title = title.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(30.0),
            background_color: header_color,
            border_radius: UCornerRadius::top(6.0),
            padding: USides::axes(8.0, 4.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    align_items: UAlignItems::Center,
                    gap: 6.0,
                }
                Children [
                    (UText {
                        text: { Icon::CHEVRON_DOWN.to_string() },
                        font_size: 11.0,
                        font: { icon_font },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: title,
                        font_size: 12.0,
                        font: { font },
                        color: Color::WHITE,
                    }),
                ]
            ),
            (
                UNode {
                    width: UVal::Px(6.0),
                    height: UVal::Px(6.0),
                    background_color: Color::srgba(1.0, 1.0, 1.0, 0.4),
                    border_radius: UCornerRadius::all(3.0),
                }
            )
        ]
    }
}

/// Blender socket pin (10px circle on the edge).
fn blender_socket_pin(color: Color, is_connected: bool) -> impl Scene {
    let pin_bg = if is_connected {
        color
    } else {
        Color::srgb(0.14, 0.14, 0.15)
    };

    bsn! {
        UNode {
            width: UVal::Px(10.0),
            height: UVal::Px(10.0),
            background_color: pin_bg,
            border_radius: UCornerRadius::all(5.0),
        }
        UBorder {
            color: { color },
            width: 1.5,
            radius: UCornerRadius::all(5.0),
            offset: 0.0,
        }
        UInteraction::default()
        UInteractionScale::default()
    }
}

/// Blender input socket row.
fn blender_input_row(
    label: &str,
    color: Color,
    default_val: Option<&str>,
    is_connected: bool,
    font: Handle<Font>,
) -> impl Scene {
    let label = label.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(22.0),
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
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 6.0,
            }
            Children [
                @blender_socket_pin(color, is_connected)
                --
                UText {
                    text: label,
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.85, 0.85, 0.85),
                }
            ]
            --
            @blender_value_field(default_val, font)
        ]
    }
}

/// Blender dual socket row (Input on Left, Output on Right).
#[allow(clippy::too_many_arguments)]
fn blender_dual_row(
    in_label: &str,
    in_color: Color,
    default_val: Option<&str>,
    in_connected: bool,
    out_label: &str,
    out_color: Color,
    out_connected: bool,
    font: Handle<Font>,
) -> impl Scene {
    let in_label = in_label.to_string();
    let out_label = out_label.to_string();
    let val_widget = blender_value_field(default_val, font.clone());

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(22.0),
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
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 6.0,
            }
            Children [
                @blender_socket_pin(in_color, in_connected)
                --
                UText {
                    text: in_label,
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.85, 0.85, 0.85),
                }
                --
                @val_widget
            ]
            --
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 6.0,
            }
            Children [
                UText {
                    text: out_label,
                    font_size: 11.0,
                    font: { font },
                    color: Color::srgb(0.85, 0.85, 0.85),
                }
                --
                @blender_socket_pin(out_color, out_connected)
            ]
        ]
    }
}

/// Blender-style inline draggable slider value field.
fn blender_value_field(val: Option<&str>, font: Handle<Font>) -> impl Scene {
    let is_some = val.is_some();
    let text_str = val.unwrap_or("").to_string();
    let bg = if is_some {
        Color::srgb(0.18, 0.18, 0.19)
    } else {
        Color::NONE
    };
    let width = if is_some { UVal::Px(52.0) } else { UVal::Px(0.0) };
    let height = if is_some { UVal::Px(18.0) } else { UVal::Px(0.0) };

    bsn! {
        UNode {
            width: width,
            height: height,
            background_color: bg,
            border_radius: UCornerRadius::all(3.0),
            padding: USides::axes(4.0, 1.0),
        }
        UBorder {
            color: { if is_some { Color::srgb(0.1, 0.1, 0.1) } else { Color::NONE } },
            width: 1.0,
            radius: UCornerRadius::all(3.0),
            offset: 0.0,
        }
        UInteraction::default()
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            (UText {
                text: text_str,
                font_size: 10.0,
                font: { font },
                color: Color::srgb(0.9, 0.9, 0.9),
            })
        ]
    }
}

/// Blender dropdown selector widget.
fn blender_dropdown(title: &str, icon_font: Handle<Font>, font: Handle<Font>) -> impl Scene {
    let title = title.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(24.0),
            background_color: Color::srgb(0.18, 0.18, 0.20),
            border_radius: UCornerRadius::all(4.0),
            padding: USides::axes(8.0, 3.0),
        }
        UBorder {
            color: Color::srgb(0.10, 0.10, 0.12),
            width: 1.0,
            radius: UCornerRadius::all(4.0),
            offset: 0.0,
        }
        UInteraction::default()
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            (UText {
                text: title,
                font_size: 10.5,
                font: { font },
                color: Color::srgb(0.9, 0.9, 0.9),
            }),
            (UText {
                text: { Icon::CHEVRON_DOWN.to_string() },
                font_size: 11.0,
                font: { icon_font },
                color: Color::srgb(0.7, 0.7, 0.7),
            })
        ]
    }
}

// ============================================================================
// PART 2: UNREAL ENGINE 5 BLUEPRINT SPECIFICATIONS
// ============================================================================

/// Unreal Engine 5 Model A: SetActorLocation Action Function Node.
fn ue_set_actor_location_node(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    let gold = Color::srgb(0.95, 0.75, 0.15); // Vector
    let cyan = Color::srgb(0.0, 0.65, 0.95);  // Object / Target
    let red = Color::srgb(0.90, 0.18, 0.18);  // Boolean

    bsn! {
        UNode {
            width: UVal::Px(275.0),
            background_color: Color::srgb(0.06, 0.07, 0.09),
            border_radius: UCornerRadius::all(8.0),
        }
        UBorder {
            color: Color::srgba(0.2, 0.24, 0.35, 0.6),
            width: 1.0,
            radius: UCornerRadius::all(8.0),
            offset: 0.0,
        }
        UShadow {
            color: Color::srgba(0.0, 0.0, 0.0, 0.75),
            offset: Vec2::new(0.0, 8.0),
            blur: 20.0,
            spread: 1.0,
            is_inner: false,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
        }
        Children [
            // UE Function Header (Cyan/Blue Gradient)
            @ue_header(
                "SetActorLocation",
                "Target is Actor",
                Color::srgb(0.0, 0.32, 0.65),
                Color::srgb(0.08, 0.52, 0.88),
                font.clone(),
                icon_font.clone(),
            )
            --
            // UE Body
            UNode {
                padding: USides::all(10.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 8.0,
            }
            Children [
                // Row 0: Execution Pins (White Triangles ▶)
                @ue_exec_row("In", "Out", icon_font.clone(), font.clone())
                --
                // Row 1: Target Pin (Object Cyan)
                @ue_input_pin_row("Target", cyan, font.clone())
                --
                // Row 2: New Location Pin (Vector Gold) + X/Y/Z sub-inputs
                @ue_vector_input_row("New Location", gold, font.clone())
                --
                // Row 3: Sweep (Boolean Red)
                @ue_input_pin_row("Sweep", red, font.clone())
                --
                // Row 4: Teleport (Boolean Red) & Return Value (Boolean Red)
                @ue_dual_pin_row("Teleport", red, "Return Value", red, font)
            ]
        ]
    }
}

/// Unreal Engine 5 Model B: Branch (If/Else) Flow Control Node.
fn ue_branch_node(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    let red = Color::srgb(0.90, 0.18, 0.18); // Boolean Condition

    bsn! {
        UNode {
            width: UVal::Px(240.0),
            background_color: Color::srgb(0.06, 0.07, 0.09),
            border_radius: UCornerRadius::all(8.0),
        }
        UBorder {
            color: Color::srgba(0.2, 0.24, 0.35, 0.6),
            width: 1.0,
            radius: UCornerRadius::all(8.0),
            offset: 0.0,
        }
        UShadow {
            color: Color::srgba(0.0, 0.0, 0.0, 0.75),
            offset: Vec2::new(0.0, 8.0),
            blur: 20.0,
            spread: 1.0,
            is_inner: false,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
        }
        Children [
            // UE Slate / Flow Control Header
            @ue_header(
                "Branch",
                "Flow Control",
                Color::srgb(0.22, 0.24, 0.28),
                Color::srgb(0.35, 0.38, 0.44),
                font.clone(),
                icon_font.clone(),
            )
            --
            // UE Body
            UNode {
                padding: USides::all(10.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 10.0,
            }
            Children [
                // Row 1: Exec In ▶ and True ▶
                @ue_branch_exec_row("True", icon_font.clone(), font.clone())
                --
                // Row 2: Condition (Boolean Red ●) and False ▶
                @ue_condition_and_false_row("Condition", red, "False", icon_font, font)
            ]
        ]
    }
}

/// Unreal Engine Header with gradient, title, and target subtitle.
fn ue_header(
    title: &str,
    subtitle: &str,
    start_color: Color,
    end_color: Color,
    font: Handle<Font>,
    icon_font: Handle<Font>,
) -> impl Scene {
    let title = title.to_string();
    let subtitle = subtitle.to_string();
    let stops = vec![
        UGradientStop::new(0.0, start_color),
        UGradientStop::new(1.0, end_color),
    ];
    let linear_kind = UGradientKind::Linear { angle: 0.0 };

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(42.0),
            border_radius: UCornerRadius::top(8.0),
            padding: USides::axes(10.0, 6.0),
        }
        UGradient {
            stops: stops,
            kind: linear_kind,
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.2),
            width: 1.0,
            radius: UCornerRadius::top(8.0),
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 1.0,
                }
                Children [
                    (UText {
                        text: title,
                        font_size: 13.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: subtitle,
                        font_size: 9.0,
                        font: { font },
                        color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                    }),
                ]
            ),
            (
                UNode {
                    width: UVal::Px(24.0),
                    height: UVal::Px(24.0),
                    background_color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                    border_radius: UCornerRadius::all(4.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    (UText {
                        text: { Icon::CPU.to_string() },
                        font_size: 13.0,
                        font: { icon_font },
                        color: Color::WHITE,
                    })
                ]
            )
        ]
    }
}

/// Unreal Engine Exec Pin (White Solid Arrow ▶).
fn ue_exec_pin(icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Px(16.0),
            height: UVal::Px(16.0),
            background_color: Color::srgba(1.0, 1.0, 1.0, 0.15),
            border_radius: UCornerRadius::all(3.0),
        }
        UInteraction::default()
        UInteractionScale::default()
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            (UText {
                text: { Icon::PLAY.to_string() },
                font_size: 10.0,
                font: { icon_font },
                color: Color::WHITE,
            })
        ]
    }
}

/// Unreal Engine circular data pin (●).
fn ue_data_pin(color: Color) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Px(11.0),
            height: UVal::Px(11.0),
            background_color: color,
            border_radius: UCornerRadius::all(5.5),
        }
        UBorder {
            color: Color::BLACK,
            width: 1.0,
            radius: UCornerRadius::all(5.5),
            offset: 0.0,
        }
        UShadow {
            color: { color.with_alpha(0.5) },
            offset: Vec2::ZERO,
            blur: 5.0,
            spread: 1.0,
            is_inner: false,
        }
        UInteraction::default()
        UInteractionScale::default()
    }
}

/// Unreal Engine execution row: Exec In ▶ and Exec Out ▶.
fn ue_exec_row(
    _in_label: &str,
    _out_label: &str,
    icon_font: Handle<Font>,
    _font: Handle<Font>,
) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(24.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            @ue_exec_pin(icon_font.clone())
            --
            @ue_exec_pin(icon_font)
        ]
    }
}

/// Unreal Engine input pin row.
fn ue_input_pin_row(label: &str, pin_color: Color, font: Handle<Font>) -> impl Scene {
    let label = label.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(22.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            align_items: UAlignItems::Center,
            gap: 7.0,
        }
        Children [
            @ue_data_pin(pin_color)
            --
            UText {
                text: label,
                font_size: 11.0,
                font: { font },
                color: Color::srgb(0.9, 0.9, 0.9),
            }
        ]
    }
}

/// Unreal Engine dual data pin row.
fn ue_dual_pin_row(
    in_label: &str,
    in_color: Color,
    out_label: &str,
    out_color: Color,
    font: Handle<Font>,
) -> impl Scene {
    let in_label = in_label.to_string();
    let out_label = out_label.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(22.0),
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
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 7.0,
            }
            Children [
                @ue_data_pin(in_color)
                --
                UText {
                    text: in_label,
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.9, 0.9, 0.9),
                }
            ]
            --
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 7.0,
            }
            Children [
                UText {
                    text: out_label,
                    font_size: 11.0,
                    font: { font },
                    color: Color::srgb(0.9, 0.9, 0.9),
                }
                --
                @ue_data_pin(out_color)
            ]
        ]
    }
}

/// Unreal Engine multi-axis vector input row: [ X: 0.0 | Y: 0.0 | Z: 0.0 ].
fn ue_vector_input_row(label: &str, pin_color: Color, font: Handle<Font>) -> impl Scene {
    let label = label.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
        }
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
                gap: 7.0,
            }
            Children [
                @ue_data_pin(pin_color)
                --
                UText {
                    text: label,
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.9, 0.9, 0.9),
                }
            ]
            --
            // Sub-axis boxes: X, Y, Z
            UNode {
                padding: USides { left: 18.0, right: 0.0, top: 0.0, bottom: 0.0 },
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                gap: 4.0,
            }
            Children [
                @ue_axis_box("X: 0.0", Color::srgb(0.85, 0.25, 0.25), font.clone())
                --
                @ue_axis_box("Y: 0.0", Color::srgb(0.25, 0.85, 0.35), font.clone())
                --
                @ue_axis_box("Z: 0.0", Color::srgb(0.25, 0.55, 0.95), font)
            ]
        ]
    }
}

/// Small colored axis coordinate input box for Unreal vectors.
fn ue_axis_box(text: &str, accent: Color, font: Handle<Font>) -> impl Scene {
    let text = text.to_string();
    bsn! {
        UNode {
            background_color: Color::srgb(0.04, 0.04, 0.05),
            border_radius: UCornerRadius::all(3.0),
            padding: USides::axes(5.0, 2.0),
        }
        UBorder {
            color: { accent.with_alpha(0.5) },
            width: 1.0,
            radius: UCornerRadius::all(3.0),
            offset: 0.0,
        }
        Children [
            (UText {
                text,
                font_size: 9.5,
                font: { font },
                color: Color::srgb(0.85, 0.85, 0.85),
            })
        ]
    }
}

/// Unreal Branch Exec row: In ▶ and True ▶.
fn ue_branch_exec_row(true_label: &str, icon_font: Handle<Font>, font: Handle<Font>) -> impl Scene {
    let true_label = true_label.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(24.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            @ue_exec_pin(icon_font.clone())
            --
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 6.0,
            }
            Children [
                UText {
                    text: true_label,
                    font_size: 11.5,
                    font: { font },
                    color: Color::WHITE,
                }
                --
                @ue_exec_pin(icon_font)
            ]
        ]
    }
}

/// Unreal Branch Condition (Red ●) and False ▶ row.
fn ue_condition_and_false_row(
    cond_label: &str,
    cond_color: Color,
    false_label: &str,
    icon_font: Handle<Font>,
    font: Handle<Font>,
) -> impl Scene {
    let cond_label = cond_label.to_string();
    let false_label = false_label.to_string();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(24.0),
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
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 7.0,
            }
            Children [
                @ue_data_pin(cond_color)
                --
                UText {
                    text: cond_label,
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.9, 0.9, 0.9),
                }
            ]
            --
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 6.0,
            }
            Children [
                UText {
                    text: false_label,
                    font_size: 11.5,
                    font: { font },
                    color: Color::WHITE,
                }
                --
                @ue_exec_pin(icon_font)
            ]
        ]
    }
}

/// Bottom footer.
fn showcase_footer(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(36.0),
            background_color: Color::srgba(0.07, 0.08, 0.12, 0.75),
            border_radius: UCornerRadius::all(8.0),
            padding: USides::axes(16.0, 6.0),
        }
        UBorder {
            color: Color::srgba(0.2, 0.25, 0.38, 0.25),
            width: 1.0,
            radius: UCornerRadius::all(8.0),
            offset: 0.0,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            (
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    align_items: UAlignItems::Center,
                    gap: 8.0,
                }
                Children [
                    (UText {
                        text: { Icon::INFO.to_string() },
                        font_size: 14.0,
                        font: { icon_font },
                        color: Color::srgb(0.4, 0.75, 1.0),
                    }),
                    (UText {
                        text: { "Comparing Node Paradigms: Blender Flat Geometry & Sliders vs. Unreal Engine Blueprints & Exec Flow Pins (▶).".to_string() },
                        font_size: 11.5,
                        font: { font.clone() },
                        color: Color::srgb(0.65, 0.70, 0.85),
                    }),
                ]
            ),
            (UText {
                text: { "Luma UI · Signed Distance Field Engine".to_string() },
                font_size: 11.0,
                font: { font },
                color: Color::srgb(0.45, 0.50, 0.65),
            })
        ]
    }
}
