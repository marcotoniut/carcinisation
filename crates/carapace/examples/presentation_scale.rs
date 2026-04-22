#![allow(clippy::needless_pass_by_value)]
// Demonstrates CxPresentationTransform scaling on a composite sprite (Mosquiton).
//
// Left: static reference at native size.
// Right: oscillates smoothly between 50% and 200% over 4 seconds.

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
        .add_systems(Update, oscillate_scale)
        .run();
}

#[derive(Component)]
struct Oscillating;

/// Convert a top-left image-space offset to bottom-left engine coordinates.
fn bottom_left(top_left_offset: IVec2, part_height: i32) -> IVec2 {
    IVec2::new(top_left_offset.x, -(top_left_offset.y + part_height))
}

/// Builds a mosquiton composite from `idle_stand` frame 0.
///
/// Atlas region mapping (from atlas.composed.ron `idle_stand` poses):
///   0 = body left half (11x25)
///   1 = body centre strip (1x25)
///   2 = head left half (6x30)
///   3 = head centre strip (1x30)
///   4 = arms overlay (24x28)
///   5 = wings (18x33)
///   6 = wing centre strip (1x9)
///   7 = legs (15x28)
fn mosquiton_composite(assets: &Res<AssetServer>) -> CxCompositeSprite {
    let atlas: Handle<CxSpriteAtlasAsset> = assets.load("sprite/mosquiton/atlas.px_atlas.ron");

    // Draw order matches the composed manifest: wings → body → head → arms → legs.
    CxCompositeSprite::new(vec![
        // Wings: left, centre strip, right (flipped)
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_offset(bottom_left(IVec2::new(-18, -1), 33)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(6))
            .with_offset(bottom_left(IVec2::new(0, 3), 9)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -1), 33)),
        // Body: left, centre strip, right (flipped)
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_offset(bottom_left(IVec2::new(-11, -4), 25)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(1))
            .with_offset(bottom_left(IVec2::new(0, -4), 25)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -4), 25)),
        // Head: left, centre strip, right (flipped)
        // Head is parented to body, so offsets are body_pivot + head_local_offset:
        // body pivot = (-11, -4), head offsets = (5,6), (11,6), (12,6)
        // absolute = (-6, 2), (0, 2), (1, 2)
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_offset(bottom_left(IVec2::new(-6, 2), 30)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(3))
            .with_offset(bottom_left(IVec2::new(0, 2), 30)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, 2), 30)),
        // Arms overlay: left, right (flipped)
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_offset(bottom_left(IVec2::new(-24, 5), 28)),
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, 5), 28)),
        // Legs: left, right (flipped)
        CxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(7))
            .with_offset(bottom_left(IVec2::new(-19, 17), 28)),
        CxCompositePart::atlas_region(atlas, AtlasRegionId(7))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(5, 17), 28)),
    ])
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    // Left: static reference at native size on the back layer.
    commands.spawn((
        mosquiton_composite(&assets),
        CxPosition(IVec2::new(32, 64)),
        Layer::Back,
    ));

    // Right: oscillating scale between 50% and 200%, on the front layer.
    commands.spawn((
        mosquiton_composite(&assets),
        CxPosition(IVec2::new(96, 64)),
        CxPresentationTransform::default(),
        Oscillating,
        Layer::Front,
    ));
}

/// Oscillates scale between 50% and 200% over a 4-second sine wave period.
fn oscillate_scale(
    time: Res<Time>,
    mut query: Query<&mut CxPresentationTransform, With<Oscillating>>,
) {
    let t = time.elapsed_secs();
    let phase = (t / 4.0) * std::f32::consts::TAU;
    // sin range [-1, 1] mapped to [0.5, 2.0]: midpoint 1.25, amplitude 0.75
    let s = 1.25 + 0.75 * phase.sin();

    for mut transform in &mut query {
        transform.scale = Vec2::splat(s);
    }
}

#[px_layer]
enum Layer {
    #[default]
    Back,
    Front,
}
