#![allow(clippy::needless_pass_by_value)]
// Demonstrates the experimental GPU palette sprite path.

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
            CxPlugin::<Layer>::new(UVec2::new(48, 32), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let mage = assets.load("sprite/mage.px_sprite.png");
    let runner = assets.load("sprite/runner.px_sprite.png");
    let mage_cast = assets.load("sprite/mage_cast.px_sprite.png");

    // CPU sprite on the back layer (left).
    commands.spawn((
        CxSprite(mage_cast),
        CxPosition(IVec2::new(2, 10)),
        CxAnchor::BottomLeft,
        Layer::Back,
    ));

    // GPU palette sprite in the middle layer (center).
    commands.spawn((
        CxSprite(runner),
        CxGpuSprite,
        CxPosition(IVec2::new(18, 10)),
        CxAnchor::BottomLeft,
        Layer::Middle,
    ));

    // CPU sprite on the front layer (right) to demonstrate depth ordering.
    commands.spawn((
        CxSprite(mage),
        CxPosition(IVec2::new(34, 10)),
        CxAnchor::BottomLeft,
        Layer::Front,
    ));
}

// Layers are in render order: back to front.
#[px_layer]
enum Layer {
    #[default]
    Back,
    Middle,
    Front,
}
