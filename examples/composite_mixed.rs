#![allow(clippy::needless_pass_by_value)]
// In this program, a composite sprite mixes a standalone sprite with an atlas-backed part.

use bevy::prelude::*;
use carapace::prelude::*;

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
            PxPlugin::<Layer>::new(UVec2::splat(16), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let body = assets.load("sprite/mage.px_sprite.png");
    let atlas = assets.load("atlas/example.px_atlas.ron");

    let composite = PxCompositeSprite::new(vec![
        PxCompositePart::new(body),
        PxCompositePart::atlas_region(atlas, AtlasRegionId(0))
            .with_offset(IVec2::new(4, 6))
            .with_flip(true, false),
    ]);

    commands.spawn((composite, PxPosition(IVec2::splat(8))));
}

#[px_layer]
struct Layer;
