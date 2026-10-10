//! Comprehensive image example for bevy_luma testing UImage rendering:
//! 1. Procedural dynamic texture (amber/indigo checkerboard)
//! 2. Real downloaded manga page from 3asq (Dr. Hound Chapter 1)

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_luma::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI - Image & Manga 3asq Showcase".into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .add_systems(Startup, setup_ui)
        .run();
}

fn create_checkerboard_texture(images: &mut Assets<Image>) -> Handle<Image> {
    let width = 256;
    let height = 256;
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let check = ((x / 32) + (y / 32)) % 2 == 0;
            if check {
                data.extend_from_slice(&[245, 158, 11, 255]); // Amber
            } else {
                data.extend_from_slice(&[99, 102, 241, 255]); // Indigo
            }
        }
    }

    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    images.add(image)
}

fn load_manga_texture(path: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let bytes = std::fs::read(path).ok()?;
    let dynamic_img = image::load_from_memory(&bytes).ok()?;
    let (w, h) = (dynamic_img.width(), dynamic_img.height());
    let target_h = 1024;
    let target_w = ((w as f32) * (target_h as f32 / h as f32)).round() as u32;
    let resized = dynamic_img.resize(target_w, target_h, image::imageops::FilterType::Triangle);
    let rgba = resized.to_rgba8();

    let image = Image::new(
        Extent3d {
            width: rgba.width(),
            height: rgba.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    Some(images.add(image))
}

fn setup_ui(mut commands: Commands, theme: Res<Theme>, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);

    let procedural_handle = create_checkerboard_texture(&mut images);
    let manga_handle = load_manga_texture("/tmp/dr_hound_001.jpg", &mut images);
    let font = theme.text.font.inter_regular.clone();

    // Screen UI Root
    let root = commands.spawn((
        URootUi::screen(),
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.06, 0.07, 0.10),
            padding: USides::all(24.0),
            ..default()
        },
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::SpaceBetween,
            align_items: UAlignItems::Center,
            gap: 16.0,
            ..default()
        },
    )).id();

    // Title & subtitle header
    let header_box = commands.spawn((
        UNode::default(),
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            align_items: UAlignItems::Center,
            gap: 4.0,
            ..default()
        },
    )).id();

    let title = commands.spawn(UText {
        text: "bevy_luma • Image Component & 3asq Manga Showcase".to_string(),
        font_size: 24.0,
        font: font.clone(),
        color: Color::WHITE,
        ..default()
    }).id();

    let subtitle = commands.spawn(UText {
        text: "Hardware SDF Rounded Box Clipping • Real-time Texture Sampling • 3asq Offline Source".to_string(),
        font_size: 13.0,
        font: font.clone(),
        color: Color::srgb(0.55, 0.65, 0.8),
        ..default()
    }).id();

    commands.entity(header_box).add_child(title);
    commands.entity(header_box).add_child(subtitle);
    commands.entity(root).add_child(header_box);

    // Main Row holding both showcase cards
    let row_box = commands.spawn((
        UNode::default(),
        USelf {
            flex_grow: Some(1.0),
            ..default()
        },
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Row,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
            gap: 36.0,
            ..default()
        },
    )).id();

    // --- CARD 1: Procedural Checkerboard ---
    let card1 = commands.spawn((
        UNode {
            width: UVal::Px(360.0),
            height: UVal::Px(560.0),
            background_color: Color::srgb(0.10, 0.12, 0.16),
            border_radius: UCornerRadius::all(16.0),
            padding: USides::all(18.0),
            ..default()
        },
        UBorder {
            color: Color::srgba(0.3, 0.45, 0.7, 0.35),
            width: 1.5,
            radius: UCornerRadius::all(16.0),
            ..default()
        },
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            align_items: UAlignItems::Center,
            justify_content: UJustifyContent::SpaceBetween,
            gap: 14.0,
            ..default()
        },
    )).id();

    let img1 = commands.spawn((
        UNode {
            width: UVal::Px(320.0),
            height: UVal::Px(450.0),
            border_radius: UCornerRadius::all(12.0),
            ..default()
        },
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.2),
            width: 1.0,
            radius: UCornerRadius::all(12.0),
            ..default()
        },
        UImage::new(procedural_handle)
            .with_size(UVal::Px(320.0), UVal::Px(450.0))
            .with_radius(UCornerRadius::all(12.0)),
    )).id();

    let cap1 = commands.spawn(UText {
        text: "Procedural Checkerboard (256x256)".to_string(),
        font_size: 13.0,
        font: font.clone(),
        color: Color::srgb(0.8, 0.85, 0.95),
        ..default()
    }).id();

    commands.entity(card1).add_child(img1);
    commands.entity(card1).add_child(cap1);
    commands.entity(row_box).add_child(card1);

    // --- CARD 2: 3asq Manga Page ---
    let card2 = commands.spawn((
        UNode {
            width: UVal::Px(360.0),
            height: UVal::Px(560.0),
            background_color: Color::srgb(0.10, 0.12, 0.16),
            border_radius: UCornerRadius::all(16.0),
            padding: USides::all(18.0),
            ..default()
        },
        UBorder {
            color: Color::srgba(0.85, 0.45, 0.2, 0.4),
            width: 1.5,
            radius: UCornerRadius::all(16.0),
            ..default()
        },
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            align_items: UAlignItems::Center,
            justify_content: UJustifyContent::SpaceBetween,
            gap: 14.0,
            ..default()
        },
    )).id();

    let final_manga_handle = manga_handle.unwrap_or_else(|| create_checkerboard_texture(&mut images));

    let img2 = commands.spawn((
        UNode {
            width: UVal::Px(320.0),
            height: UVal::Px(450.0),
            border_radius: UCornerRadius::all(12.0),
            ..default()
        },
        UBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 0.2),
            width: 1.0,
            radius: UCornerRadius::all(12.0),
            ..default()
        },
        UImage::new(final_manga_handle)
            .with_size(UVal::Px(320.0), UVal::Px(450.0))
            .with_radius(UCornerRadius::all(12.0)),
    )).id();

    let cap2 = commands.spawn(UText {
        text: "Manga 3asq: Dr. Hound (Ch. 1, P. 1)".to_string(),
        font_size: 13.0,
        font: font.clone(),
        color: Color::srgb(0.95, 0.85, 0.7),
        ..default()
    }).id();

    commands.entity(card2).add_child(img2);
    commands.entity(card2).add_child(cap2);
    commands.entity(row_box).add_child(card2);

    commands.entity(root).add_child(row_box);
}
