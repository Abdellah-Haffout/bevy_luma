//! # Luma UI - EditText / Text Input Showcase
//!
//! Visual Theme: Obsidian / Deep Slate with Cyan & Indigo Accent Focus Rings.
//!
//! Features demonstrated:
//! - Interactive text input fields (`UTextInput` and `UTextInputCaret`)
//! - Text caret cursor blinking and real-time horizontal positioning
//! - Text selection highlight (`UTextInputSelectionMarker`)
//! - Focus rings (`UFocusable` and `UFocusVisual::border`)
//! - Placeholder handling when empty vs. active text color
//! - Input modes: Normal text vs. Password masking (`TextInputMode::Password`)
//! - Input filtering: `TextInputFilter::AlphaNumeric`, `TextInputFilter::NumericOnly`, `TextInputFilter::SingleLine`
//! - Multilingual & Arabic text input (`noto_sans_arabic_regular`)
//! - Reactive events: `UTextInputChanged` (typing) and `UTextInputSubmit` (Enter key)
//! - Interactive Form buttons: "Submit Form" and "Clear All"

use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct EditTextDemoState {
    last_submitted: String,
    last_changed: String,
    submit_count: u32,
    focused_name: String,
}

impl Default for EditTextDemoState {
    fn default() -> Self {
        Self {
            last_submitted: "None (Press Enter in any field)".to_string(),
            last_changed: "Ready for input".to_string(),
            submit_count: 0,
            focused_name: "Click any input below to focus".to_string(),
        }
    }
}

/// Marker component for the live preview text label.
#[derive(Component, Clone, Default)]
struct LivePreviewText;

/// Marker component for the last submitted notification text.
#[derive(Component, Clone, Default)]
struct SubmittedBannerText;

/// Marker component for the focused field indicator text.
#[derive(Component, Clone, Default)]
struct FocusedFieldText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - EditText / Text Input Showcase".into(),
                resolution: (1000, 840).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .init_resource::<EditTextDemoState>()
        .add_systems(Startup, setup_ui)
        .add_systems(Update, (sync_demo_ui, track_focus_change))
        .add_observer(on_text_changed)
        .add_observer(on_text_submit)
        .run();
}

fn setup_ui(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(edit_text_showcase_scene(&theme));
}

fn edit_text_showcase_scene(theme: &Theme) -> impl Scene {
    let body_font = theme.text.font.inter_regular.clone();
    let arabic_font = theme.text.font.noto_sans_arabic_regular.clone();
    let icon_font = theme.icon.font.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.045, 0.055, 0.08),
            padding: USides::all(28.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            // Top Header
            @header_bar(body_font.clone(), icon_font.clone())
            --
            // Main Central Form Card
            @form_card(body_font.clone(), arabic_font, icon_font.clone())
            --
            // Bottom Status and Instructions Bar
            @footer_status_bar(body_font, icon_font)
        ]
    }
}

// ---------------------------------------------------------------------------
// 1. Header Bar
// ---------------------------------------------------------------------------

fn header_bar(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Px(820.0),
            background_color: Color::srgba(0.08, 0.10, 0.16, 0.9),
            border_radius: UCornerRadius::all(12.0),
            padding: USides::axes(20.0, 14.0),
        }
        UBorder {
            color: Color::srgba(0.25, 0.35, 0.55, 0.4),
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
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 12.0,
            }
            Children [
                UNode {
                    width: UVal::Px(40.0),
                    height: UVal::Px(40.0),
                    background_color: Color::srgba(0.0, 0.75, 1.0, 0.15),
                    border_radius: UCornerRadius::all(8.0),
                }
                UBorder {
                    color: Color::srgb(0.0, 0.75, 1.0),
                    width: 1.5,
                    radius: UCornerRadius::all(8.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                }
                Children [
                    UText {
                        text: { Icon::TYPE.to_string() },
                        font_size: 18.0,
                        font: { icon_font },
                        color: Color::srgb(0.2, 0.85, 1.0),
                    }
                ]
                --
                UNode::default()
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 2.0,
                }
                Children [
                    UText {
                        text: { "Luma UI — EditText / Text Input Architecture".to_string() },
                        font_size: 16.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }
                    --
                    UText {
                        text: { "Real-time Caret Blinking, Text Selection, Filter Validation & UTF-8".to_string() },
                        font_size: 11.5,
                        font: { font.clone() },
                        color: Color::srgb(0.55, 0.62, 0.75),
                    }
                ]
            ]
            --
            // Active Focus Badge
            UNode {
                background_color: Color::srgba(0.12, 0.16, 0.25, 0.8),
                border_radius: UCornerRadius::all(6.0),
                padding: USides::axes(12.0, 6.0),
            }
            UBorder {
                color: Color::srgba(0.3, 0.45, 0.7, 0.3),
                width: 1.0,
                radius: UCornerRadius::all(6.0),
            }
            Children [
                FocusedFieldText
                UText {
                    text: { "Ready • Select field".to_string() },
                    font_size: 11.0,
                    font: { font },
                    color: Color::srgb(0.4, 0.8, 1.0),
                }
            ]
        ]
    }
}

// ---------------------------------------------------------------------------
// 2. Main Central Form Card
// ---------------------------------------------------------------------------

fn form_card(
    body_font: Handle<Font>,
    arabic_font: Handle<Font>,
    icon_font: Handle<Font>,
) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Px(820.0),
            background_color: Color::srgb(0.07, 0.085, 0.13),
            border_radius: UCornerRadius::all(16.0),
            padding: USides::all(24.0),
        }
        UBorder {
            color: Color::srgba(0.25, 0.32, 0.48, 0.5),
            width: 1.5,
            radius: UCornerRadius::all(16.0),
        }
        UShadow {
            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
            offset: Vec2::new(0.0, 10.0),
            blur: 24.0,
            spread: 2.0,
            is_inner: false,
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            gap: 16.0,
        }
        Children [
            // Row 1: Username (AlphaNumeric Filter)
            @field_container(
                "USERNAME / اسم المستخدم",
                "Alphanumeric only (A-Z, 0-9, underscores)",
                edit_text_field(
                    "Enter your username...",
                    "alex_developer",
                    TextInputMode::Normal,
                    TextInputFilter::AlphaNumeric,
                    body_font.clone(),
                ),
                body_font.clone(),
            )
            --
            // Row 2: Password (Masked bullets ●)
            @field_container(
                "PASSWORD / كلمة المرور",
                "Masked with bullets ● (TextInputMode::Password)",
                edit_text_field(
                    "Enter secret password...",
                    "LumaSecurePass2026!",
                    TextInputMode::Password,
                    TextInputFilter::SingleLine,
                    body_font.clone(),
                ),
                body_font.clone(),
            )
            --
            // Row 3: Numeric Port (Numeric Only Filter)
            @field_container(
                "SERVER PORT / رقم المنفذ",
                "Digits only (TextInputFilter::NumericOnly)",
                edit_text_field(
                    "e.g. 8080",
                    "8080",
                    TextInputMode::Normal,
                    TextInputFilter::NumericOnly,
                    body_font.clone(),
                ),
                body_font.clone(),
            )
            --
            // Row 4: Arabic / Multilingual Note
            @field_container(
                "ARABIC NOTE / ملاحظة باللغة العربية",
                "Full UTF-8 support with Noto Sans Arabic",
                edit_text_field(
                    "اكتب رسالتك هنا...",
                    "واجهة مستخدم احترافية لمحرك Bevy",
                    TextInputMode::Normal,
                    TextInputFilter::SingleLine,
                    arabic_font,
                ),
                body_font.clone(),
            )
            --
            // Live Status Feedback Box
            @live_feedback_box(body_font.clone())
            --
            // Form Buttons (Clear All & Submit)
            @form_action_buttons(body_font, icon_font)
        ]
    }
}

/// A labeled container for an individual EditText input.
fn field_container(
    title: &'static str,
    hint: &'static str,
    field: impl Scene,
    font: Handle<Font>,
) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            gap: 5.0,
        }
        Children [
            UNode {
                width: UVal::Percent(1.0),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Baseline,
            }
            Children [
                UText {
                    text: { title.to_string() },
                    font_size: 11.5,
                    font: { font.clone() },
                    color: Color::srgb(0.78, 0.84, 0.95),
                }
                --
                UText {
                    text: { hint.to_string() },
                    font_size: 10.0,
                    font: { font },
                    color: Color::srgb(0.42, 0.48, 0.60),
                }
            ]
            --
            @{field}
        ]
    }
}

/// Standalone interactive EditText field conforming to Luma's `UTextInput` ECS architecture.
fn edit_text_field(
    placeholder: &'static str,
    initial: &'static str,
    mode: TextInputMode,
    filter: TextInputFilter,
    font: Handle<Font>,
) -> impl Scene {
    let font_size = 13.5f32;
    let initial_text = if initial.is_empty() {
        placeholder.to_string()
    } else if mode == TextInputMode::Password {
        "●".repeat(initial.chars().count())
    } else {
        initial.to_string()
    };
    let initial_color = if initial.is_empty() {
        Color::srgba(0.42, 0.48, 0.60, 0.65)
    } else {
        Color::srgb(0.92, 0.95, 0.98)
    };

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(40.0),
            padding: USides::axes(14.0, 8.0),
            background_color: Color::srgba(0.04, 0.05, 0.08, 0.85),
            border_radius: UCornerRadius::all(8.0),
        }
        UBorder {
            color: Color::srgba(0.22, 0.28, 0.42, 0.5),
            width: 1.5,
            radius: UCornerRadius::all(8.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            align_items: UAlignItems::Center,
        }
        UFocusable::new()
        UFocusVisual::border(Color::srgb(0.0, 0.85, 1.0), 2.0)
        UInteraction::default()
        UTextInput {
            placeholder: { placeholder.to_string() },
            value: { initial.to_string() },
            mode: { mode },
            filter: { filter },
            font_size: { font_size },
        }
        UTextInputCaret {
            color: Color::srgb(0.0, 0.85, 1.0),
            width: 2.0,
        }
        Children [
            // Selection highlight
            UNode {
                width: UVal::Px(0.0),
                height: { UVal::Px(font_size * 1.25) },
                background_color: Color::srgba(0.0, 0.5, 1.0, 0.35),
                border_radius: UCornerRadius::all(2.0),
            }
            USelf {
                position_type: UPositionType::Absolute,
                left: UVal::Px(14.0),
            }
            Visibility::Hidden
            UTextInputSelectionMarker
            --
            // Text Label
            UTextInputTextMarker
            UText {
                text: initial_text,
                font_size: font_size,
                font: { font },
                color: initial_color,
            }
            --
            // Caret cursor
            UNode {
                width: UVal::Px(2.0),
                height: { UVal::Px(font_size * 1.25) },
                background_color: Color::srgb(0.0, 0.85, 1.0),
                border_radius: UCornerRadius::all(1.0),
            }
            USelf {
                position_type: UPositionType::Absolute,
                left: UVal::Px(14.0),
            }
            Visibility::Hidden
            UTextInputCaretMarker
        ]
    }
}

// ---------------------------------------------------------------------------
// 3. Live Feedback HUD & Action Buttons
// ---------------------------------------------------------------------------

fn live_feedback_box(font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            background_color: Color::srgba(0.03, 0.04, 0.07, 0.8),
            border_radius: UCornerRadius::all(8.0),
            padding: USides::axes(14.0, 10.0),
        }
        UBorder {
            color: Color::srgba(0.18, 0.24, 0.36, 0.4),
            width: 1.0,
            radius: UCornerRadius::all(8.0),
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
                gap: 3.0,
            }
            Children [
                UText {
                    text: { "REAL-TIME INPUT STREAM:".to_string() },
                    font_size: 10.0,
                    font: { font.clone() },
                    color: Color::srgb(0.48, 0.55, 0.68),
                }
                --
                LivePreviewText
                UText {
                    text: { "Ready for input".to_string() },
                    font_size: 12.5,
                    font: { font.clone() },
                    color: Color::srgb(0.2, 0.85, 1.0),
                }
            ]
            --
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                align_items: UAlignItems::End,
                gap: 3.0,
            }
            Children [
                UText {
                    text: { "LAST SUBMITTED (ENTER):".to_string() },
                    font_size: 10.0,
                    font: { font.clone() },
                    color: Color::srgb(0.48, 0.55, 0.68),
                }
                --
                SubmittedBannerText
                UText {
                    text: { "None (Press Enter)".to_string() },
                    font_size: 12.5,
                    font: { font },
                    color: Color::srgb(0.4, 0.9, 0.55),
                }
            ]
        ]
    }
}

fn form_action_buttons(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            padding: USides::axes(0.0, 4.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::End,
            gap: 12.0,
        }
        Children [
            // Clear All Button
            UNode {
                height: UVal::Px(36.0),
                background_color: Color::srgba(0.12, 0.15, 0.22, 0.8),
                border_radius: UCornerRadius::all(8.0),
                padding: USides::axes(16.0, 8.0),
            }
            UBorder {
                color: Color::srgba(0.25, 0.3, 0.45, 0.4),
                width: 1.0,
                radius: UCornerRadius::all(8.0),
            }
            UInteraction::default()
            UInteractionColors {
                normal: Color::srgba(0.12, 0.15, 0.22, 0.8),
                hovered: Color::srgba(0.18, 0.22, 0.32, 0.9),
                pressed: Color::srgba(0.22, 0.28, 0.40, 1.0),
            }
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
            }
            Children [
                UText {
                    text: { "Clear All".to_string() },
                    font_size: 12.0,
                    font: { font.clone() },
                    color: Color::srgb(0.7, 0.75, 0.85),
                }
            ]
            on(|_event: On<PointerClick>, mut inputs: Query<&mut UTextInput>, mut state: ResMut<EditTextDemoState>| {
                for mut input in inputs.iter_mut() {
                    input.value.clear();
                    input.cursor_position = 0;
                    input.selection = None;
                }
                state.last_changed = "All inputs cleared".to_string();
                info!("Cleared all text inputs");
            })
            --
            // Submit Button
            UNode {
                height: UVal::Px(36.0),
                background_color: Color::srgb(0.0, 0.65, 0.95),
                border_radius: UCornerRadius::all(8.0),
                padding: USides::axes(18.0, 8.0),
            }
            UBorder {
                color: Color::srgb(0.3, 0.85, 1.0),
                width: 1.0,
                radius: UCornerRadius::all(8.0),
            }
            UInteraction::default()
            UInteractionColors {
                normal: Color::srgb(0.0, 0.65, 0.95),
                hovered: Color::srgb(0.1, 0.75, 1.0),
                pressed: Color::srgb(0.0, 0.50, 0.80),
            }
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                gap: 6.0,
            }
            Children [
                UText {
                    text: { Icon::SEND.to_string() },
                    font_size: 13.0,
                    font: { icon_font },
                    color: Color::WHITE,
                }
                --
                UText {
                    text: { "Submit Form".to_string() },
                    font_size: 12.5,
                    font: { font },
                    color: Color::WHITE,
                }
            ]
            on(|_event: On<PointerClick>, inputs: Query<&UTextInput>, mut state: ResMut<EditTextDemoState>| {
                state.submit_count += 1;
                let mut values: Vec<String> = Vec::new();
                for input in inputs.iter() {
                    values.push(input.value.clone());
                }
                state.last_submitted = format!("Form submitted ({} fields)", values.len());
                info!("Submit Form clicked: {:?}", values);
            })
        ]
    }
}

// ---------------------------------------------------------------------------
// 4. Footer Status Bar
// ---------------------------------------------------------------------------

fn footer_status_bar(font: Handle<Font>, icon_font: Handle<Font>) -> impl Scene {
    bsn! {
        UNode {
            width: UVal::Px(820.0),
            background_color: Color::srgba(0.05, 0.065, 0.10, 0.7),
            border_radius: UCornerRadius::all(10.0),
            padding: USides::axes(18.0, 8.0),
        }
        UBorder {
            color: Color::srgba(0.2, 0.25, 0.38, 0.25),
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
            UNode::default()
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 8.0,
            }
            Children [
                UText {
                    text: { Icon::CHECK.to_string() },
                    font_size: 13.0,
                    font: { icon_font },
                    color: Color::srgb(0.25, 0.85, 0.55),
                }
                --
                UText {
                    text: { "Features: Click to focus • Drag to select • Arrow keys • Enter to submit • Ctrl+A / Backspace".to_string() },
                    font_size: 11.0,
                    font: { font.clone() },
                    color: Color::srgb(0.52, 0.58, 0.70),
                }
            ]
            --
            UText {
                text: { "Luma UI • Bevy 0.20".to_string() },
                font_size: 10.5,
                font: { font },
                color: Color::srgb(0.38, 0.44, 0.55),
            }
        ]
    }
}

// ---------------------------------------------------------------------------
// 5. ECS Systems & Event Observers
// ---------------------------------------------------------------------------

/// Event observer triggered when text is modified in any `UTextInput`.
fn on_text_changed(trigger: On<UTextInputChanged>, mut state: ResMut<EditTextDemoState>) {
    state.last_changed = trigger.event().value.clone();
}

/// Event observer triggered when the user presses Enter on any `UTextInput`.
fn on_text_submit(trigger: On<UTextInputSubmit>, mut state: ResMut<EditTextDemoState>) {
    state.submit_count += 1;
    state.last_submitted = trigger.event().value.clone();
    info!("Submitted input [#{}] : '{}'", state.submit_count, state.last_submitted);
}

/// Synchronizes the state resource to the HUD preview labels.
fn sync_demo_ui(
    state: Res<EditTextDemoState>,
    mut preview_q: Query<&mut UText, (With<LivePreviewText>, Without<SubmittedBannerText>, Without<FocusedFieldText>)>,
    mut submitted_q: Query<&mut UText, (With<SubmittedBannerText>, Without<LivePreviewText>, Without<FocusedFieldText>)>,
    mut focused_q: Query<&mut UText, (With<FocusedFieldText>, Without<LivePreviewText>, Without<SubmittedBannerText>)>,
) {
    if state.is_changed() {
        for mut text in preview_q.iter_mut() {
            let display = if state.last_changed.is_empty() {
                "[empty]".to_string()
            } else {
                format!("\"{}\"", state.last_changed)
            };
            text.text = format!("Live: {} ({} chars)", display, state.last_changed.chars().count());
        }

        for mut text in submitted_q.iter_mut() {
            text.text = format!("[#{}] \"{}\"", state.submit_count, state.last_submitted);
        }

        for mut text in focused_q.iter_mut() {
            text.text = state.focused_name.clone();
        }
    }
}

/// Tracks which field is currently focused to show in the header badge.
fn track_focus_change(
    focus_state: Option<Res<UFocusState>>,
    inputs: Query<&UTextInput>,
    mut state: ResMut<EditTextDemoState>,
) {
    if let Some(focus) = focus_state {
        if focus.is_changed() {
            if let Some(focused_entity) = focus.focused {
                if let Ok(input) = inputs.get(focused_entity) {
                    state.focused_name = format!("Focus: {}", input.placeholder);
                }
            } else {
                state.focused_name = "No field focused".to_string();
            }
        }
    }
}
