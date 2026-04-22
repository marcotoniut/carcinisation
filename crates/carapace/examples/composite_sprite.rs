#![allow(clippy::needless_pass_by_value)]
// In this program, a mosquiton enemy is composed from atlas-backed sprite parts:
// wings, body, head, arms overlay, and legs — using idle_stand frame 0.

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
            CxPlugin::<Layer>::new(UVec2::new(96, 96), "palette/base.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

/// Convert a top-left image-space offset (as found in atlas JSON) to the engine's
/// bottom-left coordinate system.
fn bottom_left(top_left_offset: IVec2, part_height: i32) -> IVec2 {
    IVec2::new(top_left_offset.x, -(top_left_offset.y + part_height))
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let atlas: Handle<CxSpriteAtlasAsset> = assets.load("sprite/mosquiton/atlas.px_atlas.ron");

    // Idle stand frame 0 — thirteen atlas-backed fragments composited in draw order.
    // Self-symmetric sprites (body, head, arms_overlay) are auto-canonicalised:
    // only the left half is stored, mirrored at render time via fragments.
    // Odd-width parts with filled centre columns (body, head) get a centre-strip
    // fragment to preserve the middle pixel column losslessly.
    //
    // Atlas region mapping:
    //   0 = body left half (11x25)
    //   1 = body centre strip (1x25)
    //   2 = head left half (6x30)
    //   3 = head centre strip (1x30)
    //   4 = arms overlay (24x28)
    //   5 = wings (18x33)
    //   6 = wing centre strip (1x9)
    //   7 = legs (15x28)
    let composite = CxCompositeSprite::new(vec![
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
        // Head is parented to body — offsets resolved as body_pivot + head_local:
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
    ]);

    commands.spawn((composite, CxPosition(IVec2::new(48, 48))));
}

#[px_layer]
struct Layer;
