#![allow(clippy::needless_pass_by_value)]
// In this program, a filter is applied to a single sprite

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

    // Spawn a sprite
    commands.spawn((
        CxSprite(assets.load("sprite/mage.px_sprite.png")),
        CxPosition(IVec2::new(8, 16)),
    ));

    // Spawn a sprite with a filter
    commands.spawn((
        CxSprite(assets.load("sprite/mage.px_sprite.png")),
        CxPosition(IVec2::new(24, 16)),
        CxFilter(assets.load("filter/invert.px_filter.png")),
    ));
}

#[px_layer]
struct Layer;
