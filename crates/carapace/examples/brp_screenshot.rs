#![allow(clippy::needless_pass_by_value)]
// Minimal BRP screenshot target.
//
// Run:
//   cargo run --example brp_screenshot --features brp_extras
//
// Capture:
//   mkdir -p tmp
//   curl -s http://127.0.0.1:15702/jsonrpc \
//     -H 'content-type: application/json' \
//     -d '{"jsonrpc":"2.0","id":1,"method":"brp_extras/screenshot","params":{"path":"tmp/brp_screenshot.png"}}'

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
            CxPlugin::<Layer>::new(UVec2::splat(64), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Layer,
        CxUiRoot,
        CxText::new(
            "BRP screenshot target",
            assets.load("typeface/typeface.px_typeface.png"),
        ),
    ));
}

#[px_layer]
struct Layer;
