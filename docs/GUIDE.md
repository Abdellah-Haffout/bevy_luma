# 🧭 The Luma Developer Guide & Handbook

> A practical, no-nonsense guide designed so you never get stuck or lost while building UI in Bevy.

---

## Table of Contents
1. [The 5-Minute Mental Model](#1-the-5-minute-mental-model)
2. [Quick Component Reference (Cheat Sheet)](#2-quick-component-reference)
3. [Common Layout Recipes (Copy-Paste)](#3-common-layout-recipes)
4. [SDF Styling: Corners, Borders, Shadows & Glow](#4-sdf-styling)
5. [Handling User Input & Clicks](#5-handling-user-input--clicks)
6. [2D Screen UI vs. 3D In-World Spatial UI](#6-2d-screen-vs-3d-in-world-ui)
7. [The "Why Isn't It Working?" Troubleshooting Checklist](#7-troubleshooting-checklist)
8. [Which Example Should I Look At?](#8-which-example-should-i-look-at)

---

## 1. The 5-Minute Mental Model

Luma is built on three simple, predictable concepts:

```
[Camera2d or Camera3d]
       │
[URootUi] ─── The Canvas (Screen, Virtual Canvas, or 3D World plane)
       │
    [UNode] ─── The Box (Size, Flexbox/Grid, Color, Rounded Corners, Shadows)
       │
    ├── [UText]  ─── Formatted text or vector icons
    └── [UNode]  ─── Child containers, buttons, cards...
```

### The 3 Rules:
1. **Always have a Camera**: Screen UI needs `Camera2d`; 3D world UI needs `Camera3d`.
2. **Every UI tree starts with a `URootUi`**: Never spawn loose `UNode` entities as top-level entities without a root.
3. **Nodes flow naturally**: By default, child nodes arrange in a column or row using standard Flexbox or CSS Grid.

---

## 2. Quick Component Reference

| What you want to do | Component to use | Key properties |
| :--- | :--- | :--- |
| **Create a Screen UI canvas** | `URootUi::screen()` | Stretches to window size automatically |
| **Create a fixed-resolution HUD** | `URootUi::virtual_screen(Vec2::new(1920.0, 1080.0))` | Scales automatically across all monitors |
| **Create a 3D in-game panel** | `URootUi::world_3d()` | Places UI into the physical 3D scene |
| **Size & position a box** | `UNode` | `width`, `height`, `padding`, `margin`, `flex_direction` |
| **Render rounded corners** | `corner_radius: UCornerRadius::all(12.0)` | Independent corners or uniform radius |
| **Render borders** | `border: UBorder::all(1.5, color)` | Crisp SDF anti-aliased border |
| **Render shadows** | `shadow: UShadow { ... }` | Offset, blur, spread, color |
| **Render glowing HUD edges** | `glow: UGlow { ... }` | Inner/outer holographic glows |
| **Display text or Lucide icons** | `UText` | `.with_font()`, `.with_font_size()`, `.with_color()` |
| **Make a node clickable/hoverable** | `UInteraction::default()` | Responds to cursor hover, press, release |
| **Auto-color on hover/press** | `UInteractionColors { ... }` | Automatic tinting without writing extra systems |
| **Hover scale bounce** | `UInteractionScale { ... }` | Smooth physics-driven spring scale bump |
| **Gamepad / Keyboard focus** | `UFocusable::new()` | Tab, Arrow keys, Gamepad D-pad navigation |
| **Scrollable content list** | `UScrollContainer` | Kinetic momentum drag and smooth scrollbars |
| **Interactive text input field** | `UTextInput` | Caret cursor, selection, keyboard typing, filters |

---

## 3. Common Layout Recipes

### Recipe A: Centered Modal / Dialog Card
```rust
commands.spawn((
    URootUi::screen(),
    UNode {
        width: UVal::Percent(1.0),
        height: UVal::Percent(1.0),
        justify_content: UJustifyContent::Center, // Center horizontally
        align_items: UAlignItems::Center,         // Center vertically
        background_color: Color::srgba(0.0, 0.0, 0.0, 0.6), // Dimmed backdrop
        ..default()
    },
)).with_children(|root| {
    root.spawn((
        UNode {
            width: UVal::Px(420.0),
            padding: USides::all(24.0),
            flex_direction: UFlexDirection::Column,
            gap: 16.0,
            background_color: Color::srgb(0.12, 0.14, 0.18),
            corner_radius: UCornerRadius::all(16.0),
            border: UBorder::all(1.0, Color::srgb(0.25, 0.30, 0.40)),
            shadow: UShadow {
                offset: Vec2::new(0.0, 12.0),
                blur: 32.0,
                spread: 0.0,
                color: Color::srgba(0.0, 0.0, 0.0, 0.6),
            },
            ..default()
        },
    )).with_children(|card| {
        card.spawn(UText::from("Confirm Action").with_font_size(20.0));
        card.spawn(UText::from("Are you sure you want to proceed?").with_font_size(14.0));
    });
});
```

### Recipe B: Top Navigation Bar with Left Logo & Right Buttons
```rust
root.spawn((
    UNode {
        width: UVal::Percent(1.0),
        height: UVal::Px(64.0),
        padding: USides::symmetric(24.0, 0.0),
        flex_direction: UFlexDirection::Row,
        justify_content: UJustifyContent::SpaceBetween, // Pushes items to opposite edges
        align_items: UAlignItems::Center,
        background_color: Color::srgb(0.08, 0.10, 0.14),
        ..default()
    },
)).with_children(|nav| {
    // Left: Brand
    nav.spawn(UText::from("MY GAME").with_font_size(22.0).with_color(Color::WHITE));

    // Right: Action Links
    nav.spawn((
        UNode {
            flex_direction: UFlexDirection::Row,
            gap: 16.0,
            align_items: UAlignItems::Center,
            ..default()
        },
    )).with_children(|links| {
        links.spawn(UText::from("Settings").with_font_size(15.0));
        links.spawn(UText::from("Profile").with_font_size(15.0));
    });
});
```

### Recipe C: 3-Column Responsive CSS Grid
```rust
root.spawn((
    UNode {
        width: UVal::Percent(1.0),
        display: UDisplay::Grid,
        grid_template_columns: vec![
            UTrackSize::Fr(1.0),
            UTrackSize::Fr(1.0),
            UTrackSize::Fr(1.0),
        ],
        gap: 16.0,
        padding: USides::all(20.0),
        ..default()
    },
)).with_children(|grid| {
    for i in 1..=6 {
        grid.spawn((
            UNode {
                height: UVal::Px(140.0),
                background_color: Color::srgb(0.15, 0.18, 0.24),
                corner_radius: UCornerRadius::all(12.0),
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
            UText::from(format!("Card #{i}")),
        ));
    }
});
```

---

## 4. SDF Styling: Crisp Visuals with Zero Assets

Luma does not use raster images for rounded corners or borders. Everything is computed mathematically in GPU shaders:

```rust
UNode {
    // 1. Independent or Uniform Corner Radius
    corner_radius: UCornerRadius::all(16.0),
    // Or: UCornerRadius { top_left: 20.0, top_right: 0.0, bottom_right: 20.0, bottom_left: 0.0 }

    // 2. Anti-aliased Border
    border: UBorder::all(2.0, Color::srgb(0.3, 0.6, 1.0)),

    // 3. Drop Shadow
    shadow: UShadow {
        offset: Vec2::new(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: Color::srgba(0.0, 0.0, 0.0, 0.5),
    },

    // 4. Holographic Glow (Sci-Fi / Cyberpunk HUDs)
    glow: UGlow {
        color: Color::srgb(0.2, 0.8, 1.0),
        size: 16.0,
        intensity: 1.5,
    },
    ..default()
}
```

---

## 5. Handling User Input & Clicks

### Approach 1: Automatic Color Hover Feedback (Zero Rust Code)
Just attach `UInteractionColors`:
```rust
commands.spawn((
    UNode {
        width: UVal::Px(160.0),
        height: UVal::Px(48.0),
        background_color: Color::srgb(0.20, 0.25, 0.35),
        corner_radius: UCornerRadius::all(8.0),
        ..default()
    },
    UInteraction::default(),
    UInteractionColors {
        hover: Some(Color::srgb(0.28, 0.35, 0.48)),
        pressed: Some(Color::srgb(0.15, 0.18, 0.25)),
        ..default()
    },
    UInteractionScale::default(), // Optional: adds a smooth spring zoom-bump
));
```

### Approach 2: Listening to Clicks via Bevy Picking Observers
```rust
commands.spawn((
    UNode { ... },
    UInteraction::default(),
))
.observe(|trigger: Trigger<Pointer<Click>>| {
    println!("Entity {:?} was clicked!", trigger.target());
});
```

### Approach 3: Keyboard & Gamepad Focus Navigation
Add `UFocusable::new()`:
```rust
commands.spawn((
    UNode { ... },
    UFocusable::new().with_tab_index(1),
    UFocusVisual::glow(Color::srgb(0.2, 0.8, 1.0)), // Shows automatic focus halo
));
```
Users can now navigate with <kbd>Tab</kbd>, Arrow keys, or Gamepad D-pad automatically.

---

## 6. 2D Screen vs. 3D In-World UI

The beauty of Luma is that 2D screen UI and 3D in-world UI share the exact same components and layout logic.

| Use Case | Root Component | Camera Required |
| :--- | :--- | :--- |
| **Standard Screen HUD** | `URootUi::screen()` | `Camera2d` |
| **Fixed Game Canvas (e.g. 1920x1080)** | `URootUi::virtual_screen(Vec2::new(1920.0, 1080.0))` | `Camera2d` |
| **In-World Floating Display / Terminal** | `URootUi::world_3d()` | `Camera3d` |

```rust
// Spawning a floating holographic kiosk in 3D:
commands.spawn((
    URootUi::world_3d(),
    Transform::from_xyz(0.0, 1.5, -2.0)
        .with_rotation(Quat::from_rotation_y(0.2)),
    UNode {
        width: UVal::Px(600.0),
        height: UVal::Px(400.0),
        background_color: Color::srgba(0.05, 0.08, 0.14, 0.85),
        border: UBorder::all(2.0, Color::srgb(0.0, 0.8, 1.0)),
        corner_radius: UCornerRadius::all(20.0),
        ..default()
    },
));
```

---

## 7. Troubleshooting Checklist: "Why Isn't My UI Appearing?"

1. ❌ **Did you add `LumaPlugin`?**
   Make sure `.add_plugins(LumaPlugin)` is added to your Bevy `App`.

2. ❌ **Did you spawn a camera?**
   Screen UI requires `commands.spawn(Camera2d);`. Without a camera, Bevy will render a black screen.

3. ❌ **Did you forget `URootUi`?**
   All top-level nodes must be children of an entity with `URootUi::screen()`, `URootUi::virtual_screen(...)`, or `URootUi::world_3d()`.

4. ❌ **Is your root sized properly?**
   Give your root node `width: UVal::Percent(1.0), height: UVal::Percent(1.0)` so it spans the full canvas.

5. ❌ **Is text invisible?**
   Did you assign a font handle? Use `theme.text.font.inter_regular.clone()` from `Res<Theme>`.

6. ❌ **Why is my flex child shrinking unexpectedly?**
   By default, Flexbox items can shrink. Set `flex_shrink: 0.0` or explicit `min_width` on elements you never want squished.

---

## 8. Which Example Should I Look At?

| If you want to build... | Run this example | File path |
| :--- | :--- | :--- |
| **Modern Flexbox dashboard or sidebar layout** | `cargo run --example luma_flex_dashboard` | `examples/luma_flex_dashboard.rs` |
| **Responsive store / inventory item grid** | `cargo run --example luma_grid_ecommerce` | `examples/luma_grid_ecommerce.rs` |
| **Fancy music player with overlay stacks** | `cargo run --example luma_stack_music_player` | `examples/luma_stack_music_player.rs` |
| **Glows, gradients, and custom visual SDFs** | `cargo run --example luma_visual_creative_studio` | `examples/luma_visual_creative_studio.rs` |
| **Fintech app with hover states & buttons** | `cargo run --example luma_interaction_banking` | `examples/luma_interaction_banking.rs` |
| **Settings menu with Text Inputs & forms** | `cargo run --example luma_form_profile` | `examples/luma_form_profile.rs` |
| **Infinite feed with smooth kinetic scrolling** | `cargo run --example luma_scroll_social_feed` | `examples/luma_scroll_social_feed.rs` |
| **Game menu with Gamepad / Keyboard Focus** | `cargo run --example luma_focus_game_menu` | `examples/luma_focus_game_menu.rs` |
| **Animated FAQ accordion with spring physics** | `cargo run --example luma_spring_faq_accordion` | `examples/luma_spring_faq_accordion.rs` |
| **Interactive 3D museum kiosk in world space** | `cargo run --example luma_world_3d_gallery` | `examples/luma_world_3d_gallery.rs` |
| **Detailed Fantasy / Sci-Fi RPG game HUD** | `cargo run --example luma_fantasy_rpg_hud` | `examples/luma_fantasy_rpg_hud.rs` |
| **Multi-script typography (Arabic + Latin)** | `cargo run --example luma_editorial_typography` | `examples/luma_editorial_typography.rs` |
| **Rich multi-span text with real-time outlines** | `cargo run --example luma_rich_typography` | `examples/luma_rich_typography.rs` |
