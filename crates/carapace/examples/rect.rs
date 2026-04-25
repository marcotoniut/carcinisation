#![allow(clippy::needless_pass_by_value)]
// In this program, a rectangle filter is applied over a region
//
// TODO: CxFilterRect was removed from the public API. This example needs to be
// updated to use the replacement API or removed entirely.

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
            CxPlugin::<Layer>::new(UVec2::splat(32), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let mage = assets.load("sprite/mage.px_sprite.png");

    commands.spawn((CxSprite(mage), CxPosition(IVec2::splat(16))));

    // TODO: CxFilterRect was removed. Rect filter application is no longer
    // supported through the render pipeline.
}

#[px_layer]
struct Layer;
