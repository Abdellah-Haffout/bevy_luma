//! # Luma Art Mode Toggle Showcase (`examples/luma_art_mode_toggle.rs`)
//!
//! High-fidelity, cinematic recreation of the viral "BEFORE / AFTER ART MODE"
//! procedural toggle switch using Luma SDF layout and custom [`UShader`] WGSL pipelines.
//!
//! ### Features Demonstrated:
//! 1. **"BEFORE" State**:
//!    - Minimalist, ultra-crisp dark aesthetic on pitch-black void.
//!    - Smooth 3D pearl sphere knob with directional specular lighting.
//!    - Subtle dark matte pill track with hairline slate border.
//!    - Bold uppercase "BEFORE" typographic header.
//! 2. **"AFTER" (Art Mode) State**:
//!    - **Liquid Magma Plasma Interior**: Multi-octave domain-warped FBM fluid
//!      flowing inside the pill with fiery solar filaments and hot coral/magenta spectrum.
//!    - **Solar Eclipse Obsidian Knob**: Slides across the track and transforms into
//!      a pitch-black celestial sphere with an intense golden corona rim light
//!      and sharp neon-magenta halo.
//!    - **Volumetric Multi-Color Aura**: Diffuse outer atmospheric radiation glowing
//!      warm amber on the left and electric fuchsia on the right.
//!    - **Glossy Floor Reflection**: Soft elliptical ground reflection mirroring
//!      the luminous capsule onto the dark reflective surface below.
//!    - **"ART MODE" Label**: Widely-spaced tracked subtitle glowing into view.
//! 3. **Physical Spring Interaction**:
//!    - Critically damped spring physics for buttery 60+ FPS motion.
//!    - [Click Switch / Knob] or [Space]: Toggle between BEFORE and AFTER.
//!    - [A]: Toggle auto-loop showcase mode (cycles like the viral video).

use bevy::prelude::*;
use bevy_luma::prelude::*;

const PILL_WGSL: &str = r#"
let progress = params[0].x;
let t = time * 0.9;

// In BEFORE mode: sleek OLED black interior
let off_bg = vec3<f32>(0.002, 0.002, 0.004);

// Multi-octave organic liquid magma / plasma
let q = vec2<f32>(
    luma_fbm(uv * 2.8 + vec2<f32>(t * 0.5, -t * 0.35)),
    luma_fbm(uv * 2.5 + vec2<f32>(-t * 0.45, t * 0.45) + vec2<f32>(4.2, 1.9))
);

let r = vec2<f32>(
    luma_fbm(uv * 2.4 + 2.2 * q + vec2<f32>(1.7, 9.1) + vec2<f32>(t * 0.25, -t * 0.2)),
    luma_fbm(uv * 2.2 + 2.2 * q + vec2<f32>(8.5, 2.7) + vec2<f32>(-t * 0.3, t * 0.35))
);

let p = clamp(uv + (r - 0.5) * 0.32, vec2<f32>(0.0), vec2<f32>(1.0));

// Plasma energy filaments & fluid ribbons
let flame1 = sin((p.x * 3.6 + p.y * 1.8 + r.x * 2.8) * 6.28318 - t * 1.8);
let flame2 = cos((p.x * 2.2 - p.y * 3.2 + r.y * 2.2) * 6.28318 + t * 1.5);
let core_band = pow(clamp(0.5 + 0.5 * (flame1 * 0.6 + flame2 * 0.4), 0.0, 1.0), 2.0);

// Rich vibrant color spectrum:
// Left: Intense Solar Gold & Amber
// Center: Blazing Hot Flame
// Right: Electric Neon Fuchsia & Magenta
let grad_x = clamp(p.x + (r.x - 0.5) * 0.18, 0.0, 1.0);
let col_gold = vec3<f32>(1.0, 0.88, 0.10);
let col_amber = vec3<f32>(1.0, 0.50, 0.02);
let col_flame = vec3<f32>(1.0, 0.18, 0.04);
let col_magenta = vec3<f32>(0.98, 0.02, 0.58);
let col_pink = vec3<f32>(1.0, 0.15, 0.80);

var plasma = mix(col_gold, col_amber, smoothstep(0.05, 0.35, grad_x));
plasma = mix(plasma, col_flame, smoothstep(0.30, 0.65, grad_x));
plasma = mix(plasma, col_magenta, smoothstep(0.60, 0.90, grad_x));
plasma = mix(plasma, col_pink, smoothstep(0.88, 1.0, grad_x));

// Solar flare white-hot energy highlight in core
let flare = pow(core_band, 2.2) * (1.0 - smoothstep(0.40, 0.90, grad_x));
plasma = plasma + vec3<f32>(1.0, 0.98, 0.82) * flare * 1.6;

// Electric cyan accent on bottom-left curve (signature detail from video)
let cyan_dist = length((uv - vec2<f32>(0.10, 0.90)) * vec2<f32>(1.8, 1.0));
let cyan_glow = smoothstep(0.36, 0.0, cyan_dist) * 0.85;
plasma = mix(plasma, vec3<f32>(0.0, 0.85, 1.0), cyan_glow);

// Purple flare accent on top-left curve
let purple_dist = length((uv - vec2<f32>(0.10, 0.10)) * vec2<f32>(1.8, 1.0));
let purple_glow = smoothstep(0.32, 0.0, purple_dist) * 0.70;
plasma = mix(plasma, vec3<f32>(0.65, 0.10, 0.95), purple_glow);

// Inner rim vignette for optical depth
let edge_x = smoothstep(0.0, 0.06, uv.x) * smoothstep(1.0, 0.94, uv.x);
let edge_y = smoothstep(0.0, 0.12, uv.y) * smoothstep(1.0, 0.88, uv.y);
plasma = plasma * (0.75 + 0.25 * edge_x * edge_y);

let out_rgb = mix(off_bg, plasma, progress);
return vec4<f32>(out_rgb, 1.0);
"#;

const KNOB_WGSL: &str = r#"
let progress = params[0].x;
let d = uv - vec2<f32>(0.5, 0.5);
let r = length(d) * 2.0;

// 1. BEFORE STATE: Ultra-smooth 3D Pearl Sphere
let light_dir = normalize(vec2<f32>(-0.4, -0.6));
let sphere_z = sqrt(max(0.0, 1.0 - r * r));
let pearl_base = mix(vec3<f32>(0.86, 0.88, 0.92), vec3<f32>(1.0, 1.0, 1.0), sphere_z * 0.70 + (0.5 - uv.y) * 0.30);
let specular = pow(max(0.0, sphere_z * 0.80 + dot(d, light_dir) * 0.40), 6.0) * 0.45;
let before_col = clamp(pearl_base + vec3<f32>(specular), vec3<f32>(0.0), vec3<f32>(1.0));

// 2. AFTER STATE: Solar Eclipse Obsidian Sphere
let obsidian_core = vec3<f32>(0.001, 0.001, 0.002);

// Fiery left crescent corona (facing the flaming plasma)
let left_facing = clamp(-d.x * 2.4, 0.0, 1.0);
let crescent = smoothstep(0.65, 0.98, r) * left_facing;
let fiery_rim = vec3<f32>(1.0, 0.82, 0.22) * pow(crescent, 1.6) * 3.2;

// Right pink/magenta rim highlight
let right_facing = clamp(d.x * 2.4, 0.0, 1.0);
let pink_crescent = smoothstep(0.75, 0.98, r) * right_facing;
let pink_rim = vec3<f32>(1.0, 0.10, 0.65) * pow(pink_crescent, 1.9) * 2.5;

// Optical razor-thin outer halo ring
let halo = smoothstep(0.86, 0.98, r);
let halo_tint = mix(vec3<f32>(1.0, 0.65, 0.12), vec3<f32>(1.0, 0.08, 0.65), clamp(uv.x, 0.0, 1.0));

let after_col = obsidian_core + fiery_rim + pink_rim + halo_tint * halo * 0.95;

let out_col = mix(before_col, after_col, progress);
return vec4<f32>(out_col, 1.0);
"#;

const AURA_WGSL: &str = r#"
let progress = params[0].x;
let t = time * 0.6;

// Centered radial bloom with smooth windowed falloff to exactly zero at edges
let d = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 1.35);
let r = length(d) * 2.0;

let window = smoothstep(1.0, 0.35, r);
let falloff = exp(-pow(r * 1.8, 1.3)) * window;

let col_left = vec3<f32>(1.0, 0.70, 0.08);
let col_right = vec3<f32>(0.98, 0.04, 0.62);
let aura_col = mix(col_left, col_right, smoothstep(0.30, 0.70, uv.x));

let breathe = 0.94 + 0.06 * sin(t * 1.6);
let alpha = clamp(falloff * progress * breathe * 0.95, 0.0, 1.0);
return vec4<f32>(aura_col, alpha);
"#;

const REFLECTION_WGSL: &str = r#"
let progress = params[0].x;

let d = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 2.6);
let r = length(d) * 2.0;
let window = smoothstep(1.0, 0.3, r);
let falloff = exp(-pow(r * 1.8, 1.4)) * window;

let col_left = vec3<f32>(1.0, 0.62, 0.06);
let col_right = vec3<f32>(0.96, 0.04, 0.58);
let refl_col = mix(col_left, col_right, smoothstep(0.30, 0.70, uv.x));

let vert_fade = smoothstep(1.0, 0.05, uv.y * 0.8);
let alpha = clamp(falloff * vert_fade * progress * 0.60, 0.0, 1.0);
return vec4<f32>(refl_col, alpha);
"#;

#[derive(Resource)]
struct ArtToggleState {
    pub is_on: bool,
    pub progress: f32,
    pub target: f32,
    pub velocity: f32,
    pub auto_cycle: bool,
    pub cycle_timer: Timer,
}

impl Default for ArtToggleState {
    fn default() -> Self {
        let start_on = std::env::args().any(|a| a == "--art" || a == "--on" || a == "--after");
        let initial_p = if start_on { 1.0 } else { 0.0 };
        Self {
            is_on: start_on,
            progress: initial_p,
            target: initial_p,
            velocity: 0.0,
            auto_cycle: false,
            cycle_timer: Timer::from_seconds(2.8, TimerMode::Repeating),
        }
    }
}

#[derive(Component, Clone, Copy, Default)]
struct ArtTogglePill;

#[derive(Component, Clone, Copy, Default)]
struct ArtToggleKnob;

#[derive(Component, Clone, Copy, Default)]
struct ArtToggleAura;

#[derive(Component, Clone, Copy, Default)]
struct ArtToggleReflection;

#[derive(Component, Clone, Copy, Default)]
struct ArtToggleHeaderLabel;

#[derive(Component, Clone, Copy, Default)]
struct ArtToggleModeLabel;

#[derive(Component, Clone, Copy, Default)]
struct ArtToggleInteractiveZone;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI — ART MODE Toggle Showcase".into(),
                resolution: (720, 1280).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .init_resource::<ArtToggleState>()
        .add_systems(Startup, setup_art_mode_showcase)
        .add_systems(
            Update,
            (
                handle_input,
                update_spring_physics,
                update_toggle_visuals,
            ),
        )
        .run();
}

fn setup_art_mode_showcase(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2d);
    commands.spawn_scene(art_mode_scene(&theme));
}

fn art_mode_scene(theme: &Theme) -> impl Scene {
    let font = theme.text.font.inter_regular.clone();

    // Color definitions for plasma spectrum
    let yellow = Color::srgb(1.0, 0.84, 0.12);
    let orange = Color::srgb(1.0, 0.28, 0.04);
    let magenta = Color::srgb(0.96, 0.04, 0.60);

    bsn! {
        URootUi::screen()
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.022, 0.024, 0.030), // Deep cinematic dark background
        }
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
        }
        Children [
            // =================================================================
            // 1. Top Typographic Header ("BEFORE" / "AFTER")
            // =================================================================
            UNode {
                margin: USides::bottom(46.0),
            }
            UText {
                text: { "BEFORE".to_string() },
                font_size: 38.0,
                font: { font.clone() },
                color: Color::WHITE,
            }
            ArtToggleHeaderLabel
            --

            // =================================================================
            // 2. Center Stage: Volumetric Aura + Main Pill Switch + Knob
            // =================================================================
            UNode {
                width: UVal::Px(520.0),
                height: UVal::Px(320.0),
            }
            ULayout {
                display: UDisplay::Stack,
                align_items: UAlignItems::Center,
            }
            ArtToggleInteractiveZone
            on(|_event: On<PointerClick>, mut state: ResMut<ArtToggleState>| {
                state.is_on = !state.is_on;
                state.target = if state.is_on { 1.0 } else { 0.0 };
            })
            Children [
                // Layer 0: Volumetric Multi-Color Radial Aura (Behind pill)
                UNode {
                    width: UVal::Px(520.0),
                    height: UVal::Px(320.0),
                    border_radius: UCornerRadius::all(160.0),
                }
                UShader::from_wgsl(AURA_WGSL)
                    .with_blend_mode(UShaderBlendMode::AlphaBlend)
                    .with_speed(0.8)
                    .with_param(0, Vec4::ZERO)
                ArtToggleAura
                --

                // Layer 1: The Main Pill Switch Track
                UNode {
                    width: UVal::Px(240.0),
                    height: UVal::Px(120.0),
                    border_radius: UCornerRadius::all(60.0),
                }
                UBorder {
                    width: 1.8,
                    color: Color::srgba(0.28, 0.30, 0.38, 0.45),
                    radius: UCornerRadius::all(60.0),
                }
                UShader::from_wgsl(PILL_WGSL)
                    .with_colors([yellow, orange, magenta])
                    .with_param(0, Vec4::ZERO)
                    .with_speed(1.0)
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Row,
                    align_items: UAlignItems::Center,
                }
                ArtTogglePill
                Children [
                    // The Knob (Pearl sphere in BEFORE, Solar Eclipse in AFTER)
                    UNode {
                        width: UVal::Px(96.0),
                        height: UVal::Px(96.0),
                        border_radius: UCornerRadius::all(48.0),
                        margin: USides::left(12.0),
                    }
                    UBorder {
                        width: 1.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.22),
                        radius: UCornerRadius::all(48.0),
                    }
                    UShadow {
                        color: Color::srgba(0.0, 0.0, 0.0, 0.45),
                        offset: Vec2::new(3.0, 4.0),
                        blur: 14.0,
                        spread: 0.0,
                        is_inner: false,
                    }
                    UShader::from_wgsl(KNOB_WGSL)
                        .with_param(0, Vec4::ZERO)
                    ArtToggleKnob
                ]
            ]
            --

            // =================================================================
            // 3. Subtitle ("ART MODE")
            // =================================================================
            UNode {
                margin: USides { top: 12.0, bottom: 36.0, left: 0.0, right: 0.0 },
            }
            UText {
                text: { "A  R  T    M  O  D  E".to_string() },
                font_size: 15.0,
                font: { font.clone() },
                color: Color::srgba(0.9, 0.92, 0.98, 0.0), // Fades in with Art Mode
            }
            ArtToggleModeLabel
            --

            // =================================================================
            // 4. Glossy Floor Reflection
            // =================================================================
            UNode {
                width: UVal::Px(420.0),
                height: UVal::Px(64.0),
                border_radius: UCornerRadius::all(32.0),
            }
            UShader::from_wgsl(REFLECTION_WGSL)
                .with_blend_mode(UShaderBlendMode::AlphaBlend)
                .with_param(0, Vec4::ZERO)
            ArtToggleReflection
            --

            // =================================================================
            // 5. Instruction Footer
            // =================================================================
            UNode {
                margin: USides::top(64.0),
            }
            UText {
                text: { "[CLICK SWITCH OR PRESS SPACE TO TOGGLE • PRESS 'A' FOR AUTO-CYCLE]".to_string() },
                font_size: 11.0,
                font: { font },
                color: Color::srgba(0.48, 0.52, 0.65, 0.5),
            }
        ]
    }
}

/// Handles user input: keyboard shortcuts and mouse clicks.
fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut state: ResMut<ArtToggleState>,
) {
    // Space key triggers toggle
    if keyboard.just_pressed(KeyCode::Space) {
        state.is_on = !state.is_on;
        state.target = if state.is_on { 1.0 } else { 0.0 };
    }

    // 'A' key toggles auto-cycle loop
    if keyboard.just_pressed(KeyCode::KeyA) {
        state.auto_cycle = !state.auto_cycle;
    }

    // Auto-cycle timer
    if state.auto_cycle {
        state.cycle_timer.tick(time.delta());
        if state.cycle_timer.just_finished() {
            state.is_on = !state.is_on;
            state.target = if state.is_on { 1.0 } else { 0.0 };
        }
    }
}

/// Simulates critically damped spring physics for organic, physical switch response.
fn update_spring_physics(time: Res<Time>, mut state: ResMut<ArtToggleState>) {
    let delta = time.delta_secs().min(0.05);
    if delta <= 0.0 {
        return;
    }

    let stiffness = 210.0;
    let damping = 22.0;

    let displacement = state.progress - state.target;
    let spring_force = -stiffness * displacement;
    let damping_force = -damping * state.velocity;
    let acceleration = spring_force + damping_force;

    state.velocity += acceleration * delta;
    state.progress += state.velocity * delta;

    // Settle cleanly near targets
    if (state.progress - state.target).abs() < 0.001 && state.velocity.abs() < 0.005 {
        state.progress = state.target;
        state.velocity = 0.0;
    }
}

/// Synchronizes layout positions, shader uniforms, borders, and text labels with current animation state.
fn update_toggle_visuals(
    state: Res<ArtToggleState>,
    mut knob_query: Query<(&mut UNode, &mut UShader, &mut UBorder), (With<ArtToggleKnob>, Without<ArtTogglePill>, Without<ArtToggleAura>, Without<ArtToggleReflection>)>,
    mut pill_query: Query<(&mut UBorder, &mut UShader), (With<ArtTogglePill>, Without<ArtToggleKnob>, Without<ArtToggleAura>, Without<ArtToggleReflection>)>,
    mut aura_query: Query<&mut UShader, (With<ArtToggleAura>, Without<ArtTogglePill>, Without<ArtToggleKnob>, Without<ArtToggleReflection>)>,
    mut refl_query: Query<&mut UShader, (With<ArtToggleReflection>, Without<ArtTogglePill>, Without<ArtToggleKnob>, Without<ArtToggleAura>)>,
    mut header_query: Query<&mut UText, (With<ArtToggleHeaderLabel>, Without<ArtToggleModeLabel>)>,
    mut mode_query: Query<&mut UText, (With<ArtToggleModeLabel>, Without<ArtToggleHeaderLabel>)>,
) {
    let p = state.progress.clamp(0.0, 1.0);

    // 1. Knob slide position and eclipse shader
    let knob_start_x = 12.0;
    let knob_end_x = 132.0;
    let knob_x = knob_start_x + (knob_end_x - knob_start_x) * p;

    for (mut node, mut shader, mut border) in &mut knob_query {
        node.margin.left = knob_x;
        shader.params[0].x = p;
        border.color = Color::srgba(1.0, 1.0, 1.0, 0.22 * (1.0 - p));
    }

    // 2. Pill interior plasma and border glow
    let border_off = Color::srgba(0.28, 0.30, 0.38, 0.45);
    let border_on = Color::srgba(1.0, 0.60, 0.25, 0.85);
    let border_color = Color::srgba(
        border_off.to_srgba().red * (1.0 - p) + border_on.to_srgba().red * p,
        border_off.to_srgba().green * (1.0 - p) + border_on.to_srgba().green * p,
        border_off.to_srgba().blue * (1.0 - p) + border_on.to_srgba().blue * p,
        border_off.to_srgba().alpha * (1.0 - p) + border_on.to_srgba().alpha * p,
    );

    for (mut border, mut shader) in &mut pill_query {
        border.color = border_color;
        shader.params[0].x = p;
    }

    // 3. Volumetric Aura
    for mut shader in &mut aura_query {
        shader.params[0].x = p;
    }

    // 4. Ground Reflection
    for mut shader in &mut refl_query {
        shader.params[0].x = p;
    }

    // 5. Header Label ("BEFORE" <-> "AFTER")
    for mut text in &mut header_query {
        if p < 0.48 {
            text.text = "BEFORE".to_string();
            text.color = Color::WHITE;
        } else {
            text.text = "AFTER".to_string();
            text.color = Color::srgb(1.0, 0.97, 0.90);
        }
    }

    // 6. Mode Label ("ART MODE" alpha fade)
    let mode_alpha = ((p - 0.2) / 0.8).clamp(0.0, 1.0) * 0.85;
    for mut text in &mut mode_query {
        text.color = Color::srgba(0.90, 0.92, 0.98, mode_alpha);
    }
}
