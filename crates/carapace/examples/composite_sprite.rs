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

    // Idle stand frame 0 — five atlas-backed parts composited in draw order.
    // The JSON local_offset for each part is relative to its parent's resolved pivot.
    // Root parts (parent=root, non-visual) resolve to their local_offset directly.
    // Child parts (e.g. head, parent=body) accumulate: parent_pivot + local_offset.
    //
    // Resolved top-left positions (from atlas.json pivot hierarchy):
    //   wings_visual: parent=root(non-visual) → pivot = (-19,-3)    → top_left = (-19,-3)
    //   body:         parent=root(non-visual) → pivot = (-11,-4)    → top_left = (-11,-4)
    //   head:         parent=body(visual)     → pivot = (-11,-4)+(5,6) = (-6,2)  → top_left = (-6,2)
    //   arms_overlay: parent=root(non-visual) → pivot = (-24,5)     → top_left = (-24,5)
    //   legs_visual:  parent=root(non-visual) → pivot = (-19,17)    → top_left = (-19,17)
    let composite = PxCompositeSprite::new(vec![
        // Wings (region 3, 37x35)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(3))
            .with_offset(bottom_left(IVec2::new(-19, -3), 35)),
        // Body (region 0, 23x25)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(0))
            .with_offset(bottom_left(IVec2::new(-11, -4), 25)),
        // Head (region 1, 13x30) — parent is body, so offset accumulates
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(1))
            .with_offset(bottom_left(IVec2::new(-6, 2), 30)),
        // Arms overlay (region 2, 49x28)
        PxCompositePart::atlas_region(atlas.clone(), AtlasRegionId(2))
            .with_offset(bottom_left(IVec2::new(-24, 5), 28)),
        // Legs (region 4, 39x28)
        PxCompositePart::atlas_region(atlas, AtlasRegionId(4))
            .with_offset(bottom_left(IVec2::new(-19, 17), 28)),
    ]);

    commands.spawn((composite, PxPosition(IVec2::new(48, 48))));
}

#[px_layer]
struct Layer;
