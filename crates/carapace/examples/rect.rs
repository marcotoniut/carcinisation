#![allow(clippy::needless_pass_by_value)]
// In this program, a rectangle filter is applied over a region

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
            PxPlugin::<Layer>::new(UVec2::splat(32), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let mage = assets.load("sprite/mage.px_sprite.png");

    commands.spawn((PxSprite(mage), PxPosition(IVec2::splat(16))));

    // Apply a filter only within a rectangular region.
    commands.spawn((
        PxRect(UVec2::new(12, 10)),
        PxPosition(IVec2::new(10, 22)),
        PxFilterLayers::single_over(Layer),
        PxFilter(assets.load("filter/invert.px_filter.png")),
    ));
}

#[px_layer]
struct Layer;
