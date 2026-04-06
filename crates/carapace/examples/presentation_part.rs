#![allow(clippy::needless_pass_by_value)]
// Demonstrates PxPartTransform: per-part render-time transforms on a composite sprite.
//
// Left: static reference at native size (full mosquiton).
// Right: wings oscillate rotation around their shoulder joint.
//        Body, head, arms, and legs remain static.

use bevy::prelude::*;
use carapace::{atlas::AtlasRegionId, prelude::*};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: UVec2::splat(512).into(),
                    ..default()
                }),
                ..default()
            }),
            PxPlugin::<Layer>::new(UVec2::new(128, 128), "palette/base.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .add_systems(Update, animate_wings)
        .run();
}

#[derive(Component)]
struct Animated;

fn bottom_left(top_left_offset: IVec2, part_height: i32) -> IVec2 {
    IVec2::new(top_left_offset.x, -(top_left_offset.y + part_height))
}

// Part indices for animated parts (must match build order below).
const LEFT_WING: usize = 0;
const RIGHT_WING: usize = 2;

fn mosquiton_composite(assets: &Res<AssetServer>) -> PxCompositeSprite {
    let atlas: Handle<PxSpriteAtlasAsset> = assets.load("sprite/mosquiton/atlas.px_atlas.ron");

    PxCompositeSprite::new(vec![
        // 0: Left wing (18x33).
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_offset(bottom_left(IVec2::new(-18, -1), 33)),
        // 1: Wing centre strip.
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(6))
            .with_offset(bottom_left(IVec2::new(0, 3), 9)),
        // 2: Right wing (flipped, 18x33).
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -1), 33)),
        // 3-5: Body.
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_offset(bottom_left(IVec2::new(-11, -4), 25)),
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(1))
            .with_offset(bottom_left(IVec2::new(0, -4), 25)),
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -4), 25)),
        // 6-8: Head.
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_offset(bottom_left(IVec2::new(-6, 2), 30)),
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(3))
            .with_offset(bottom_left(IVec2::new(0, 2), 30)),
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, 2), 30)),
        // 9-10: Arms.
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_offset(bottom_left(IVec2::new(-24, 5), 28)),
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, 5), 28)),
        // 11-12: Legs.
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(7))
            .with_offset(bottom_left(IVec2::new(-19, 17), 28)),
        PxCompositePart::atlas_region(atlas, AtlasRegionId(7))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(5, 17), 28)),
    ])
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    // Left: static reference.
    commands.spawn((
        mosquiton_composite(&assets),
        PxPosition(IVec2::new(32, 64)),
        Layer::Back,
    ));

    // Right: animated wing flap.
    commands.spawn((
        mosquiton_composite(&assets),
        PxPosition(IVec2::new(96, 64)),
        Animated,
        Layer::Front,
    ));
}

fn animate_wings(time: Res<Time>, mut query: Query<&mut PxCompositeSprite, With<Animated>>) {
    let t = time.elapsed_secs();

    // Wing flap: oscillate ±25° at ~1.5 Hz.
    let wing_phase = (t * 1.5 * std::f32::consts::TAU).sin();
    let wing_angle = 25.0_f32.to_radians() * wing_phase;

    for mut composite in &mut query {
        // Left wing: pivot at shoulder (top-right, near body attachment).
        composite.parts[LEFT_WING].transform = Some(PxPartTransform {
            rotation: wing_angle,
            pivot: Vec2::new(1.0, 0.1),
            ..Default::default()
        });

        // Right wing (flipped): pivot at shoulder (top-left after flip).
        composite.parts[RIGHT_WING].transform = Some(PxPartTransform {
            rotation: -wing_angle,
            pivot: Vec2::new(0.0, 0.1),
            ..Default::default()
        });
    }
}

#[px_layer]
enum Layer {
    #[default]
    Back,
    Front,
}
