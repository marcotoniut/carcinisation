#![allow(clippy::needless_pass_by_value)]
// Demonstrates CxPresentationTransform rotation on a composite sprite (Mosquiton).
//
// Left: static reference at native size.
// Right: oscillates rotation between -90° and +90° over 4 seconds.

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
        .add_systems(Update, oscillate_rotation)
        .run();
}

#[derive(Component)]
struct Oscillating;

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

    // Right: oscillating rotation (-90°..+90°), on the front layer.
    commands.spawn((
        mosquiton_composite(&assets),
        CxPosition(IVec2::new(96, 64)),
        CxPresentationTransform::default(),
        Oscillating,
        Layer::Front,
    ));
}

/// Oscillates rotation between -90° and +90° over a 4-second sine wave.
fn oscillate_rotation(
    time: Res<Time>,
    mut query: Query<&mut CxPresentationTransform, With<Oscillating>>,
) {
    let t = time.elapsed_secs();
    let phase = (t / 4.0) * std::f32::consts::TAU;
    let angle = 90.0_f32.to_radians() * phase.sin();

    for mut pt in &mut query {
        pt.rotation = angle;
    }
}

#[px_layer]
enum Layer {
    #[default]
    Back,
    Front,
}
