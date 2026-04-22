#![allow(clippy::needless_pass_by_value)]
// In this program, a line is spawned

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

    // Spawn a line. Layering and animation work the same as filters.
    commands.spawn((
        CxLine::from([(3, 22).into(), (31, 10).into()]),
        CxFilterLayers::single_over(Layer),
        CxFilter(assets.load("filter/invert.px_filter.png")),
    ));
}

#[px_layer]
struct Layer;
