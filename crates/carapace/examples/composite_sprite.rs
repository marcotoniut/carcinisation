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
            PxPlugin::<Layer>::new(UVec2::new(96, 96), "palette/base.png"),
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

    let atlas: Handle<PxSpriteAtlasAsset> = assets.load("sprite/mosquiton/atlas.px_atlas.ron");

    // Idle stand frame 0 — twelve atlas-backed fragments composited in draw order.
    // Self-symmetric sprites (body, head, arms_overlay) are auto-canonicalised:
    // only the left half is stored, mirrored at render time via fragments.
    // Odd-width parts with filled centre columns (body, head) get a 3rd centre-strip
    // fragment to preserve the middle pixel column losslessly.
    // Wings and legs use authored split=mirror_x (one gameplay part each).
    let composite = PxCompositeSprite::new(vec![
        // Wing left (region 5, 18x33)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_offset(bottom_left(IVec2::new(-18, -1), 33)),
        // Wing right (region 5 flipped, 18x33)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(5))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -1), 33)),
        // Body left half (region 0, 11x25)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_offset(bottom_left(IVec2::new(-11, -4), 25)),
        // Body centre strip (region 1, 1x25)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(1))
            .with_offset(bottom_left(IVec2::new(0, -4), 25)),
        // Body right half (region 0 flipped, 11x25)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, -4), 25)),
        // Head left half (region 2, 6x30)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_offset(bottom_left(IVec2::new(5, 6), 30)),
        // Head centre strip (region 3, 1x30)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(3))
            .with_offset(bottom_left(IVec2::new(11, 6), 30)),
        // Head right half (region 2 flipped, 6x30)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(12, 6), 30)),
        // Arms overlay left half (region 4, 24x28)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_offset(bottom_left(IVec2::new(-24, 5), 28)),
        // Arms overlay right half (region 4 flipped, 24x28)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(4))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(1, 5), 28)),
        // Legs fragment 0 (region 6, 15x28)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(6))
            .with_offset(bottom_left(IVec2::new(-19, 17), 28)),
        // Legs fragment 1 (region 6 flipped, 15x28)
        PxCompositePart::atlas_region(atlas, AtlasRegionId(6))
            .with_flip(true, false)
            .with_offset(bottom_left(IVec2::new(5, 17), 28)),
    ]);

    commands.spawn((composite, PxPosition(IVec2::new(48, 48))));
}

#[px_layer]
struct Layer;
