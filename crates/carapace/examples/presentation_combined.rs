#![allow(clippy::needless_pass_by_value)]
// Demonstrates CxPresentationTransform with scale and rotation combined.
//
// Left: static reference at native size.
// Right: continuous rotation (one full turn per 4 seconds) + scale pulse (50%..200%).

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
            CxPlugin::<Layer>::new(UVec2::new(128, 128), "palette/base.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .add_systems(Update, spin_and_pulse)
        .run();
}

#[derive(Component)]
struct Spinning;

fn bottom_left(top_left_offset: IVec2, part_height: i32) -> IVec2 {
    IVec2::new(top_left_offset.x, -(top_left_offset.y + part_height))
}

fn mosquiton_composite(assets: &Res<AssetServer>) -> CxCompositeSprite {
    let atlas: Handle<CxSpriteAtlasAsset> = assets.load("sprite/mosquiton/atlas.px_atlas.ron");

    CxCompositeSprite::new(vec![
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_offset(bottom_left(IVec2::new(-18, -1), 33)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(6))
            .with_offset(bottom_left(IVec2::new(0, 3), 9)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -1), 33)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_offset(bottom_left(IVec2::new(-11, -4), 25)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(1))
            .with_offset(bottom_left(IVec2::new(0, -4), 25)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -4), 25)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_offset(bottom_left(IVec2::new(-6, 2), 30)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(3))
            .with_offset(bottom_left(IVec2::new(0, 2), 30)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, 2), 30)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_offset(bottom_left(IVec2::new(-24, 5), 28)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, 5), 28)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(7))
            .with_offset(bottom_left(IVec2::new(-19, 17), 28)),
        CxCompositePart::atlas_region(atlas, AtlasRegionId(7))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(5, 17), 28)),
    ])
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    // Left: static reference on the back layer.
    commands.spawn((
        mosquiton_composite(&assets),
        CxPosition(IVec2::new(32, 64)),
        Layer::Back,
    ));

    // Right: continuous spin + scale pulse, on the front layer.
    commands.spawn((
        mosquiton_composite(&assets),
        CxPosition(IVec2::new(96, 64)),
        CxPresentationTransform::default(),
        Spinning,
        Layer::Front,
    ));
}

/// Continuous rotation (1 full turn per 4 seconds) + scale oscillation (50%..200%).
fn spin_and_pulse(time: Res<Time>, mut query: Query<&mut CxPresentationTransform, With<Spinning>>) {
    let t = time.elapsed_secs();

    // Continuous rotation: one full turn every 4 seconds.
    let angle = (t / 4.0) * std::f32::consts::TAU;

    // Scale pulse: 50%..200% over 4 seconds.
    let scale_phase = (t / 4.0) * std::f32::consts::TAU;
    let s = 1.25 + 0.75 * scale_phase.sin();

    for mut pt in &mut query {
        pt.rotation = angle;
        pt.scale = Vec2::splat(s);
    }
}

#[px_layer]
enum Layer {
    #[default]
    Back,
    Front,
}
