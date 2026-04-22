#![allow(clippy::needless_pass_by_value)]
// In this program, animated tilemaps are spawned

use bevy::prelude::*;
use carapace::prelude::*;
use rand::{RngExt, rng};

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
            CxPlugin::<Layer>::new(UVec2::splat(16), "palette/palette_1.palette.png"),
            CxAnimationPlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let mut tiles = CxTiles::new(UVec2::new(2, 4));
    let mut rng = rng();

    for x in 0..2 {
        for y in 0..4 {
            tiles.set(
                Some(commands.spawn(CxTile::from(rng.random_range(0..4))).id()),
                UVec2::new(x, y),
            );
        }
    }

    let tileset = assets.load("tileset/tileset.px_tileset.png");

    // Spawn the map
    commands.spawn((
        CxTilemap {
            tiles: tiles.clone(),
            tileset: tileset.clone(),
        },
        CxAnimation {
            // Use millis_per_animation to have each tile loop at the same time
            duration: CxAnimationDuration::millis_per_frame(250),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    commands.spawn((
        CxPosition(IVec2::new(8, 0)),
        CxFrameView {
            transition: CxFrameTransition::Dither,
            ..default()
        },
        CxAnimation {
            // Use millis_per_animation to have each tile loop at the same time
            duration: CxAnimationDuration::millis_per_frame(250),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
        CxTilemap { tiles, tileset },
    ));
}

#[px_layer]
struct Layer;
