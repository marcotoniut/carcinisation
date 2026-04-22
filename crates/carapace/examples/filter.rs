#![allow(clippy::needless_pass_by_value)]
// In this program, a filter is used

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

    // Spawn some sprites
    commands.spawn((CxSprite(mage.clone()), CxPosition(IVec2::new(8, 16))));

    commands.spawn((CxSprite(mage), CxPosition(IVec2::new(24, 16))));

    // Spawn a filter
    commands.spawn((
        CxFilterLayers::<Layer>::default(),
        CxFilter(assets.load("filter/invert.px_filter.png")),
    ));
}

#[px_layer]
struct Layer;
