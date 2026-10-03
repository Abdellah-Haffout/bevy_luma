# The Luma Developer Guide & Handbook

A practical guide designed to help developers build UI in Bevy with Luma using Bevy Scene Notation (BSN).

---

## Table of Contents
1. [The Mental Model](#1-the-mental-model)
2. [Component Reference (Cheat Sheet)](#2-component-reference)
3. [Common BSN Recipes (Copy-Paste)](#3-common-bsn-recipes)
4. [SDF Styling: Corners, Borders, Shadows & Glow](#4-sdf-styling)
5. [Handling User Input & Clicks](#5-handling-user-input--clicks)
6. [2D Screen UI vs. 3D In-World Spatial UI](#6-2d-screen-vs-3d-in-world-ui)
7. [Troubleshooting: Why Isn't It Working?](#7-troubleshooting)
8. [Example Directory](#8-example-directory)

---

## 1. The Mental Model

Luma is built on three simple, predictable concepts:

```
[Camera2d or Camera3d]
       │
[URootUi] ─── The Canvas (Screen, Virtual Canvas, or 3D World plane)
       │
    [UNode] ─── The Box (Size, Flexbox/Grid, Color, Rounded Corners, Shadows)
       │
    ├── [UText]  ─── Formatted text or Lucide vector icons
    └── [UNode]  ─── Child containers, buttons, cards...
```

### The 3 Rules:
1. **Always have a Camera**: Screen UI requires `Camera2d`; 3D in-world UI requires `Camera3d`.
2. **Every UI tree starts with a `URootUi`**: Use `URootUi::screen()` for 2D or `URootUi::world_3d(Vec2::new(w, h))` for 3D.
3. **Compose with BSN**: Use `bsn! { ... }` and `commands.spawn_scene(...)` for clean, declarative hierarchies.

---

## 2. Component Reference

| What you want to do | Component / Macro | Key properties |
| :--- | :--- | :--- |
| **Create a Screen UI canvas** | `URootUi::screen()` | Stretches to window size automatically |
| **Create a fixed-resolution HUD** | `URootUi::virtual_screen(Vec2::new(1920.0, 1080.0))` | Scales automatically across all monitors |
| **Create a 3D in-game panel** | `URootUi::world_3d(Vec2::new(w, h))` | Places UI into the physical 3D scene |
| **Size & position a box** | `UNode` | `width`, `height`, `padding`, `margin`, `background_color` |
| **Layout direction & alignment** | `ULayout` | `display`, `flex_direction`, `justify_content`, `align_items`, `gap` |
| **Render rounded corners** | `corner_radius: UCornerRadius::all(12.0)` | Independent corners or uniform radius |
| **Render borders** | `UBorder { width, color, radius }` | Crisp SDF anti-aliased border |
| **Render shadows** | `shadow: UShadow { ... }` | Offset, blur, spread, color |
| **Render glowing HUD edges** | `glow: UGlow { ... }` | Inner/outer holographic glows |
| **Display text or Lucide icons** | `UText` | `text`, `font_size`, `font`, `color` |
| **Make a node clickable/hoverable** | `UInteraction::default()` | Responds to cursor hover, press, release |
| **Auto-color on hover/press** | `UInteractionColors { normal, hovered, pressed }` | Automatic tinting without writing extra systems |
| **Hover scale bounce** | `UInteractionScale::default()` | Smooth physics-driven spring scale bump |
| **Gamepad / Keyboard focus** | `UFocusable::new()` | Tab, Arrow keys, Gamepad D-pad navigation |
| **Scrollable content list** | `UScrollContainer` | Kinetic momentum drag and smooth scrollbars |
| **Interactive text input field** | `UTextInput` | Caret cursor, selection, keyboard typing, filters |

---

## 3. Common BSN Recipes

### Recipe A: Centered Modal Dialog Card
```rust
fn modal_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.65),
        }
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            (
                UNode {
                    width: UVal::Px(420.0),
                    background_color: Color::srgb(0.12, 0.14, 0.20),
                    border_radius: UCornerRadius::all(16.0),
                    padding: USides::all(24.0),
                }
                UBorder {
                    color: Color::srgb(0.25, 0.30, 0.42),
                    width: 1.0,
                    radius: UCornerRadius::all(16.0),
                }
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    gap: 16.0,
                }
                Children [
                    (UText {
                        text: { "Confirm Action".to_string() },
                        font_size: 20.0,
                        font: { font.clone() },
                        color: Color::WHITE,
                    }),
                    (UText {
                        text: { "Are you sure you want to proceed with this operation?".to_string() },
                        font_size: 14.0,
                        font: { font },
                        color: Color::srgb(0.70, 0.75, 0.85),
                    }),
                ]
            )
        ]
    }
}
```

### Recipe B: Top Navigation Bar
```rust
fn navbar_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    bsn! {
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(60.0),
            background_color: Color::srgb(0.08, 0.10, 0.16),
            padding: USides::axes(24.0, 0.0),
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
        }
        Children [
            (UText {
                text: { "MY APP".to_string() },
                font_size: 20.0,
                font: { font.clone() },
                color: Color::WHITE,
            }),
            (
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    gap: 16.0,
                }
                Children [
                    (UText { text: { "Dashboard".to_string() }, font_size: 14.0, font: font.clone(), color: Color::WHITE }),
                    (UText { text: { "Settings".to_string() }, font_size: 14.0, font, color: Color::WHITE }),
                ]
            )
        ]
    }
}
```

### Recipe C: Responsive 3-Column CSS Grid
```rust
fn grid_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

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
            ],
            gap: 16.0,
            padding: USides::all(20.0),
        }
        Children [
            (grid_card("Card 1", font.clone())),
            (grid_card("Card 2", font.clone())),
            (grid_card("Card 3", font)),
        ]
    }
}

fn grid_card(title: &str, font: Handle<Font>) -> impl Scene {
    let title = title.to_string();
    bsn! {
        UNode {
            height: UVal::Px(120.0),
            background_color: Color::srgb(0.14, 0.17, 0.24),
            border_radius: UCornerRadius::all(12.0),
            padding: USides::all(16.0),
        }
        ULayout {
            display: UDisplay::Flex,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            (UText { text: title, font_size: 16.0, font, color: Color::WHITE })
        ]
    }
}
```

---

## 4. SDF Styling

Luma renders UI primitives mathematically on the GPU. Borders, rounded corners, and shadows do not use textures or nine-slice sprites.

```rust
bsn! {
    UNode {
        width: UVal::Px(300.0),
        height: UVal::Px(150.0),
        background_color: Color::srgb(0.12, 0.15, 0.22),
        border_radius: UCornerRadius::all(16.0),
    }
    UBorder {
        color: Color::srgb(0.3, 0.5, 0.9),
        width: 1.5,
        radius: UCornerRadius::all(16.0),
    }
}
```

---

## 5. Handling User Input & Clicks

### 1. Automatic Color Feedback
Attach `UInteractionColors` to automatically update node colors on hover and press without writing custom systems:

```rust
bsn! {
    UNode {
        width: UVal::Px(160.0),
        height: UVal::Px(44.0),
        background_color: Color::srgb(0.20, 0.35, 0.75),
        border_radius: UCornerRadius::all(8.0),
    }
    UInteraction::default()
    UInteractionColors {
        normal: Color::srgb(0.20, 0.35, 0.75),
        hovered: Color::srgb(0.28, 0.45, 0.90),
        pressed: Color::srgb(0.14, 0.25, 0.55),
    }
}
```

### 2. Inline BSN Click Observers
Attach click handlers directly within your BSN tree using `on(...)`:

```rust
bsn! {
    (
        action_button("Confirm")
        on(|_event: On<Pointer<Click>>| {
            info!("Confirm button clicked via BSN observer!");
        })
    )
}
```

### 3. Keyboard & Gamepad Focus Navigation
Add `UFocusable::new()`:

```rust
bsn! {
    UNode { ... }
    UFocusable::new().with_tab_index(1)
    UFocusVisual::glow(Color::srgb(0.2, 0.8, 1.0))
}
```
Users can navigate automatically using Tab, arrow keys, or a gamepad D-pad.

---

## 6. 2D Screen vs. 3D In-World UI

The same component tree works in both screen space and 3D scene space:

| Use Case | Root Component | Required Camera |
| :--- | :--- | :--- |
| **Standard Screen HUD** | `URootUi::screen()` | `Camera2d` |
| **Fixed Game Canvas (1920x1080)** | `URootUi::virtual_screen(Vec2::new(1920.0, 1080.0))` | `Camera2d` |
| **In-World Floating Display** | `URootUi::world_3d(Vec2::new(width, height))` | `Camera3d` |

Example of in-world 3D UI:
```rust
commands.spawn_scene(bsn! {
    URootUi::world_3d(Vec2::new(500.0, 320.0))
    Transform::from_xyz(0.0, 1.5, -2.0)
    UNode {
        width: UVal::Px(500.0),
        height: UVal::Px(320.0),
        background_color: Color::srgba(0.08, 0.12, 0.20, 0.90),
        border_radius: UCornerRadius::all(16.0),
    }
});
```

---

## 7. Troubleshooting

- **Nothing appears on screen**: Ensure a camera is spawned (`commands.spawn(Camera2d);`) and `LumaPlugin` is added to your app.
- **Top-level layout is broken**: Ensure your UI root uses `URootUi::screen()` and has `width: UVal::Percent(1.0)` and `height: UVal::Percent(1.0)`.
- **Text does not render**: Ensure a font handle is provided via `theme.text.font.inter_regular.clone()`.
- **Items shrink unexpectedly in Flexbox**: Set `flex_shrink: Some(0.0)` or an explicit `min_width` on elements that should not shrink.

---

## 8. Example Directory

### Practical Examples (Fast, concise, BSN-focused):
- `cargo run --example button`: Interactive button with hover feedback and click observer.
- `cargo run --example modal`: Centered modal dialog card with Confirm / Cancel.
- `cargo run --example counter`: Stateful counter (+/-) with live reactive text updates.
- `cargo run --example grid`: Responsive 3-column CSS Grid layout.
- `cargo run --example world_3d`: In-world 3D spatial interactive UI panel.

### Comprehensive Showcases:
- `cargo run --example luma_flex_dashboard`: Cloud Analytics Dashboard.
- `cargo run --example luma_grid_ecommerce`: Nordic Boutique Store.
- `cargo run --example luma_fantasy_rpg_hud`: Fantasy RPG Game HUD.
- `cargo run --example luma_editorial_typography`: Editorial Magazine & Arabic RTL.
- `cargo run --example luma_scroll_social_feed`: Kinetic Momentum Scroll Feed.
- `cargo run --example luma_focus_game_menu`: Gamepad & Keyboard Focus Navigation.
