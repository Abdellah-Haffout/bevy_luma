//! Multi-Root Layering & Depth Stacking Example
//!
//! Demonstrates:
//! - **Per-Root Batching**: Multiple independent `URootUi::screen()` root hierarchies.
//! - **No Interleaving / Bleed-Through**: When one root overlaps another, the front root's
//!   `UNode` backgrounds and borders cleanly obscure all `UNode` panels AND `UText` labels
//!   of the rear root (no text punch-through artifacts).
//! - **Dynamic Root Z-Indexing (`UZIndex`)**: Bringing a window to the front by updating its
//!   `UZIndex::Global` dynamically on click or keyboard press.
//! - **Animated Motion**: Smooth sliding animation demonstrating real-time depth sorting.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct LayerDemoState {
    auto_slide: bool,
    top_window: usize, // 1 for Root A, 2 for Root B
}

impl Default for LayerDemoState {
    fn default() -> Self {
        Self {
            auto_slide: true,
            top_window: 2, // Start with Window B on top
        }
    }
}

#[derive(Component, Clone, Default)]
struct WindowRootA;

#[derive(Component, Clone, Default)]
struct WindowRootB;

#[derive(Component, Clone, Default)]
struct SlidingWindow;

#[derive(Component, Clone, Default)]
struct StatusText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - Multi-Root Layering & Depth Stacking".into(),
                resolution: WindowResolution::new(1280, 800),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .init_resource::<LayerDemoState>()
        .add_systems(Startup, setup_demo)
        .add_systems(
            Update,
            (
                handle_keyboard_inputs,
                animate_sliding_window,
                sync_root_z_indices,
                update_status_display,
            ),
        )
        .run();
}

fn setup_demo(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);

    let font_body = theme.text.font.inter_regular.clone();
    let font_code = theme.text.font.fira_sans_regular.clone();

    // -------------------------------------------------------------------------
    // Root 0: HUD / Controls Bar (Always on top with Z=100)
    // -------------------------------------------------------------------------
    commands
        .spawn_scene(hud_scene(font_body.clone()))
        .insert(UZIndex::Global(100));

    // -------------------------------------------------------------------------
    // Root 1: Window 1 (Sapphire / Diagnostics Workspace)
    // -------------------------------------------------------------------------
    commands
        .spawn_scene(window_1_scene(font_body.clone(), font_code.clone()))
        .insert((WindowRootA, UZIndex::Global(1)));

    // -------------------------------------------------------------------------
    // Root 2: Window 2 (Emerald / Performance Inspector)
    // -------------------------------------------------------------------------
    commands
        .spawn_scene(window_2_scene(font_body.clone()))
        .insert((WindowRootB, UZIndex::Global(2)));
}

fn hud_scene(font: Handle<Font>) -> impl Scene {
    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            padding: USides::all(20.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
        }
        Children [
            // Top Header Banner
            (
                UNode {
                    width: UVal::Percent(1.0),
                    background_color: Color::srgba(0.06, 0.08, 0.12, 0.92),
                    border_radius: UCornerRadius::all(14.0),
                    padding: USides::axes(24.0, 16.0),
                }
                UBorder {
                    color: Color::srgb(0.20, 0.25, 0.35),
                    width: 1.0,
                    radius: UCornerRadius::all(14.0),
                }
                UShadow {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.40),
                    offset: Vec2::new(0.0, 6.0),
                    blur: 16.0,
                    spread: 0.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    justify_content: UJustifyContent::SpaceBetween,
                    align_items: UAlignItems::Center,
                }
                Children [
                    // Title and Description
                    (
                        UNode {}
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Column,
                            gap: 4.0,
                        }
                        Children [
                            (UText {
                                text: { "Luma UI — Multi-Root Layering & Depth Stacking".to_string() },
                                font_size: 18.0,
                                font: { font.clone() },
                                color: Color::WHITE,
                            }),
                            (UText {
                                text: { "Per-Root Batching: When roots overlap, child UNode panels & UText labels never interleave.".to_string() },
                                font_size: 13.0,
                                font: { font.clone() },
                                color: Color::srgb(0.60, 0.65, 0.78),
                            }),
                        ]
                    ),
                    // Action Buttons / Shortcuts
                    (
                        UNode {}
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Row,
                            gap: 12.0,
                            align_items: UAlignItems::Center,
                        }
                        Children [
                            @control_button("[1] Bring Window 1 Front", Color::srgb(0.18, 0.28, 0.45), font.clone())
                            on(|_event: On<PointerClick>, mut state: ResMut<LayerDemoState>| {
                                state.top_window = 1;
                            })
                            --
                            @control_button("[2] Bring Window 2 Front", Color::srgb(0.14, 0.38, 0.30), font.clone())
                            on(|_event: On<PointerClick>, mut state: ResMut<LayerDemoState>| {
                                state.top_window = 2;
                            })
                            --
                            @control_button("[Space] Toggle Motion", Color::srgb(0.25, 0.22, 0.35), font.clone())
                            on(|_event: On<PointerClick>, mut state: ResMut<LayerDemoState>| {
                                state.auto_slide = !state.auto_slide;
                            })
                        ]
                    ),
                ]
            ),
            // Bottom Status Legend Bar
            (
                UNode {
                    background_color: Color::srgba(0.06, 0.08, 0.12, 0.85),
                    border_radius: UCornerRadius::all(10.0),
                    padding: USides::axes(16.0, 10.0),
                }
                UBorder {
                    color: Color::srgb(0.18, 0.22, 0.30),
                    width: 1.0,
                    radius: UCornerRadius::all(10.0),
                }
                Children [
                    (
                        StatusText
                        UText {
                            text: { "Top Window: Window 2 (Emerald) | Motion: Active | Shortcuts: [1], [2], [Space]".to_string() },
                            font_size: 13.0,
                            font: { font.clone() },
                            color: Color::srgb(0.75, 0.80, 0.90),
                        }
                    )
                ]
            )
        ]
    }
}

fn window_1_scene(font_body: Handle<Font>, font_code: Handle<Font>) -> impl Scene {
    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
        }
        ULayout {
            display: UDisplay::Flex,
        }
        Children [
            (
                USelf {
                    position_type: UPositionType::Absolute,
                    left: UVal::Px(100.0),
                    top: UVal::Px(140.0),
                }
                UNode {
                    width: UVal::Px(560.0),
                    height: UVal::Px(460.0),
                    background_color: Color::srgb(0.09, 0.11, 0.17),
                    border_radius: UCornerRadius::all(16.0),
                    padding: USides::all(24.0),
                }
                UBorder {
                    color: Color::srgb(0.24, 0.38, 0.65),
                    width: 1.5,
                    radius: UCornerRadius::all(16.0),
                }
                UShadow {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.55),
                    offset: Vec2::new(0.0, 12.0),
                    blur: 28.0,
                    spread: 2.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 16.0,
                }
                on(|_event: On<PointerClick>, mut state: ResMut<LayerDemoState>| {
                    state.top_window = 1;
                })
                Children [
                    // Window Title Bar
                    (
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
                            (
                                UNode {}
                                ULayout {
                                    display: UDisplay::Flex,
                                    flex_direction: UFlexDirection::Row,
                                    gap: 10.0,
                                    align_items: UAlignItems::Center,
                                }
                                Children [
                                    (
                                        UNode {
                                            width: UVal::Px(12.0),
                                            height: UVal::Px(12.0),
                                            background_color: Color::srgb(0.30, 0.55, 0.95),
                                            border_radius: UCornerRadius::all(6.0),
                                        }
                                    ),
                                    (UText {
                                        text: { "Root 1: Engine Diagnostics & Code".to_string() },
                                        font_size: 16.0,
                                        font: { font_body.clone() },
                                        color: Color::WHITE,
                                    }),
                                ]
                            ),
                            (
                                UNode {
                                    background_color: Color::srgba(0.24, 0.38, 0.65, 0.30),
                                    border_radius: UCornerRadius::all(6.0),
                                    padding: USides::axes(8.0, 4.0),
                                }
                                Children [
                                    (UText {
                                        text: { "LAYER 1".to_string() },
                                        font_size: 11.0,
                                        font: { font_body.clone() },
                                        color: Color::srgb(0.60, 0.75, 1.0),
                                    })
                                ]
                            ),
                        ]
                    ),
                    // Inner Panel 1: Code preview
                    (
                        UNode {
                            width: UVal::Percent(1.0),
                            background_color: Color::srgb(0.05, 0.06, 0.10),
                            border_radius: UCornerRadius::all(10.0),
                            padding: USides::all(14.0),
                        }
                        UBorder {
                            color: Color::srgb(0.16, 0.20, 0.30),
                            width: 1.0,
                            radius: UCornerRadius::all(10.0),
                        }
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Column,
                            gap: 6.0,
                        }
                        Children [
                            (UText {
                                text: { "// Transparent2d Batching Phase".to_string() },
                                font_size: 13.0,
                                font: { font_code.clone() },
                                color: Color::srgb(0.40, 0.50, 0.65),
                            }),
                            (UText {
                                text: { "fn prepare_univis_nodes(mut meta: ResMut<UnivisBatchMeta>) {".to_string() },
                                font_size: 13.0,
                                font: { font_code.clone() },
                                color: Color::srgb(0.85, 0.70, 0.40),
                            }),
                            (UText {
                                text: { "    // Slices are partitioned strictly per root_entity".to_string() },
                                font_size: 13.0,
                                font: { font_code.clone() },
                                color: Color::srgb(0.50, 0.75, 0.55),
                            }),
                            (UText {
                                text: { "    meta.slices.push(UnivisBatchSlice { root_entity, range });".to_string() },
                                font_size: 13.0,
                                font: { font_code.clone() },
                                color: Color::srgb(0.70, 0.85, 1.0),
                            }),
                            (UText {
                                text: { "}".to_string() },
                                font_size: 13.0,
                                font: { font_code.clone() },
                                color: Color::srgb(0.85, 0.70, 0.40),
                            }),
                        ]
                    ),
                    // Inner Panel 2: Stats & Metrics Grid
                    (
                        UNode {
                            width: UVal::Percent(1.0),
                        }
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Row,
                            gap: 12.0,
                        }
                        Children [
                            @info_card("Batch Draw Calls", "1 per Root", Color::srgb(0.20, 0.45, 0.85), font_body.clone())
                            --
                            @info_card("Z Capsule Width", "1.000", Color::srgb(0.35, 0.60, 0.95), font_body.clone())
                            --
                            @info_card("Z-Buffer Loss", "0.00 %", Color::srgb(0.40, 0.80, 0.60), font_body.clone())
                        ]
                    ),
                    // Bottom Bar
                    (
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
                                text: { "Click inside this window to bring Root 1 to front".to_string() },
                                font_size: 12.0,
                                font: { font_body.clone() },
                                color: Color::srgb(0.55, 0.60, 0.72),
                            }
                            --
                            @control_button("▲ Focus Window 1", Color::srgb(0.20, 0.40, 0.75), font_body.clone())
                            on(|_event: On<PointerClick>, mut state: ResMut<LayerDemoState>| {
                                state.top_window = 1;
                            })
                        ]
                    )
                ]
            )
        ]
    }
}

fn window_2_scene(font: Handle<Font>) -> impl Scene {
    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
        }
        ULayout {
            display: UDisplay::Flex,
        }
        Children [
            (
                SlidingWindow
                USelf {
                    position_type: UPositionType::Absolute,
                    left: UVal::Px(420.0),
                    top: UVal::Px(220.0),
                }
                UNode {
                    width: UVal::Px(520.0),
                    height: UVal::Px(420.0),
                    background_color: Color::srgb(0.08, 0.14, 0.13),
                    border_radius: UCornerRadius::all(16.0),
                    padding: USides::all(24.0),
                }
                UBorder {
                    color: Color::srgb(0.20, 0.65, 0.45),
                    width: 1.5,
                    radius: UCornerRadius::all(16.0),
                }
                UShadow {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.60),
                    offset: Vec2::new(0.0, 14.0),
                    blur: 32.0,
                    spread: 3.0,
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 16.0,
                }
                on(|_event: On<PointerClick>, mut state: ResMut<LayerDemoState>| {
                    state.top_window = 2;
                })
                Children [
                    // Window Title Bar
                    (
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
                            (
                                UNode {}
                                ULayout {
                                    display: UDisplay::Flex,
                                    flex_direction: UFlexDirection::Row,
                                    gap: 10.0,
                                    align_items: UAlignItems::Center,
                                }
                                Children [
                                    (
                                        UNode {
                                            width: UVal::Px(12.0),
                                            height: UVal::Px(12.0),
                                            background_color: Color::srgb(0.25, 0.85, 0.55),
                                            border_radius: UCornerRadius::all(6.0),
                                        }
                                    ),
                                    (UText {
                                        text: { "Root 2: Live Inspector & Metrics".to_string() },
                                        font_size: 16.0,
                                        font: { font.clone() },
                                        color: Color::WHITE,
                                    }),
                                ]
                            ),
                            (
                                UNode {
                                    background_color: Color::srgba(0.20, 0.65, 0.45, 0.30),
                                    border_radius: UCornerRadius::all(6.0),
                                    padding: USides::axes(8.0, 4.0),
                                }
                                Children [
                                    (UText {
                                        text: { "LAYER 2".to_string() },
                                        font_size: 11.0,
                                        font: { font.clone() },
                                        color: Color::srgb(0.55, 0.95, 0.75),
                                    })
                                ]
                            ),
                        ]
                    ),
                    // Description callout
                    (
                        UNode {
                            width: UVal::Percent(1.0),
                            background_color: Color::srgb(0.05, 0.09, 0.08),
                            border_radius: UCornerRadius::all(10.0),
                            padding: USides::all(14.0),
                        }
                        UBorder {
                            color: Color::srgb(0.12, 0.28, 0.20),
                            width: 1.0,
                            radius: UCornerRadius::all(10.0),
                        }
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Column,
                            gap: 6.0,
                        }
                        Children [
                            (UText {
                                text: { "Layer Separation Guarantee".to_string() },
                                font_size: 14.0,
                                font: { font.clone() },
                                color: Color::srgb(0.35, 0.85, 0.60),
                            }),
                            (UText {
                                text: { "This entire window (background, borders, text, and sub-panels) belongs to a distinct URootUi. Notice how its dark background cleanly occludes Window 1's code text below with zero bleeding!".to_string() },
                                font_size: 13.0,
                                font: { font.clone() },
                                color: Color::srgb(0.70, 0.82, 0.78),
                            }),
                        ]
                    ),
                    // Inspector metrics cards
                    (
                        UNode {
                            width: UVal::Percent(1.0),
                        }
                        ULayout {
                            display: UDisplay::Flex,
                            flex_direction: UFlexDirection::Row,
                            gap: 12.0,
                        }
                        Children [
                            @info_card("FPS", "60.0", Color::srgb(0.20, 0.75, 0.50), font.clone())
                            --
                            @info_card("Draw Calls", "2 batches", Color::srgb(0.30, 0.80, 0.65), font.clone())
                            --
                            @info_card("Z Ordering", "Isolated", Color::srgb(0.40, 0.90, 0.55), font.clone())
                        ]
                    ),
                    // Bottom Bar
                    (
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
                                text: { "Click inside this window to bring Root 2 to front".to_string() },
                                font_size: 12.0,
                                font: { font.clone() },
                                color: Color::srgb(0.50, 0.70, 0.62),
                            }
                            --
                            @control_button("▲ Focus Window 2", Color::srgb(0.18, 0.50, 0.36), font.clone())
                            on(|_event: On<PointerClick>, mut state: ResMut<LayerDemoState>| {
                                state.top_window = 2;
                            })
                        ]
                    )
                ]
            )
        ]
    }
}

// -----------------------------------------------------------------------------
// Helper UI Builders
// -----------------------------------------------------------------------------

fn control_button(label: &str, bg: Color, font: Handle<Font>) -> impl Scene {
    let label = label.to_string();
    bsn! {
        UNode {
            background_color: bg,
            border_radius: UCornerRadius::all(8.0),
            padding: USides::axes(14.0, 8.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.15),
            width: 1.0,
            radius: UCornerRadius::all(8.0),
        }
        Children [
            (UText {
                text: label,
                font_size: 13.0,
                font: font,
                color: Color::WHITE,
            })
        ]
    }
}

fn info_card(title: &str, value: &str, accent: Color, font: Handle<Font>) -> impl Scene {
    let title = title.to_string();
    let value = value.to_string();
    bsn! {
        UNode {
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.25),
            border_radius: UCornerRadius::all(8.0),
            padding: USides::axes(12.0, 10.0),
        }
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
            radius: UCornerRadius::all(8.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            gap: 4.0,
        }
        Children [
            (UText {
                text: title,
                font_size: 11.0,
                font: { font.clone() },
                color: Color::srgb(0.55, 0.60, 0.70),
            }),
            (UText {
                text: value,
                font_size: 16.0,
                font: { font.clone() },
                color: accent,
            })
        ]
    }
}

// -----------------------------------------------------------------------------
// Interactive Systems
// -----------------------------------------------------------------------------

fn handle_keyboard_inputs(keyboard: Res<ButtonInput<KeyCode>>, mut state: ResMut<LayerDemoState>) {
    if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
        state.top_window = 1;
    }
    if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2) {
        state.top_window = 2;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        state.auto_slide = !state.auto_slide;
    }
}

fn animate_sliding_window(
    time: Res<Time>,
    state: Res<LayerDemoState>,
    mut query: Query<&mut USelf, With<SlidingWindow>>,
) {
    if !state.auto_slide {
        return;
    }

    let t = time.elapsed_secs();
    // Glide horizontally between 220px and 620px, and vertically between 180px and 320px
    let target_x = 420.0 + (t * 0.9).sin() * 200.0;
    let target_y = 220.0 + (t * 0.7).cos() * 70.0;

    for mut uself in query.iter_mut() {
        uself.left = UVal::Px(target_x);
        uself.top = UVal::Px(target_y);
    }
}

fn sync_root_z_indices(
    state: Res<LayerDemoState>,
    mut root_a: Query<&mut UZIndex, (With<WindowRootA>, Without<WindowRootB>)>,
    mut root_b: Query<&mut UZIndex, (With<WindowRootB>, Without<WindowRootA>)>,
) {
    if !state.is_changed() {
        return;
    }

    let (z_a, z_b) = if state.top_window == 1 {
        (UZIndex::Global(10), UZIndex::Global(1))
    } else {
        (UZIndex::Global(1), UZIndex::Global(10))
    };

    for mut z in root_a.iter_mut() {
        if *z != z_a {
            *z = z_a;
        }
    }
    for mut z in root_b.iter_mut() {
        if *z != z_b {
            *z = z_b;
        }
    }
}

fn update_status_display(state: Res<LayerDemoState>, mut query: Query<&mut UText, With<StatusText>>) {
    if !state.is_changed() {
        return;
    }

    let window_desc = if state.top_window == 1 {
        "Window 1 (Sapphire)"
    } else {
        "Window 2 (Emerald)"
    };
    let motion_desc = if state.auto_slide { "Active" } else { "Paused" };

    let msg = format!("Top Window: {window_desc} | Motion: {motion_desc} | Shortcuts: [1], [2], [Space]");

    for mut text in query.iter_mut() {
        text.text = msg.clone();
    }
}
