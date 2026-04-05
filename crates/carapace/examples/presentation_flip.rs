#![allow(clippy::needless_pass_by_value)]
// Demonstrates PxPresentationTransform flip / mirroring via negative scale.
//
// Uses the mage sprite (8x16, visually asymmetric) so flips are clearly visible.
//
// Four instances arranged in a 2x2 grid:
//   Top-left:     unflipped reference
//   Top-right:    horizontal flip (scale.x = -1)
//   Bottom-left:  vertical flip (scale.y = -1)
//   Bottom-right: both axes flipped + 45° rotation

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
            PxPlugin::<Layer>::new(UVec2::new(64, 64), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let mage: Handle<PxSpriteAsset> = assets.load("sprite/mage.px_sprite.png");

    // Top-left: unflipped reference.
    commands.spawn((
        PxSprite(mage.clone()),
        PxPosition(IVec2::new(16, 48)),
        Layer::Back,
    ));

    // Top-right: horizontal flip.
    commands.spawn((
        PxSprite(mage.clone()),
        PxPosition(IVec2::new(48, 48)),
        PxPresentationTransform::flipped(true, false),
        Layer::Front,
    ));

    // Bottom-left: vertical flip.
    commands.spawn((
        PxSprite(mage.clone()),
        PxPosition(IVec2::new(16, 16)),
        PxPresentationTransform::flipped(false, true),
        Layer::Front,
    ));

    // Bottom-right: both axes flipped + 45° rotation.
    commands.spawn((
        PxSprite(mage),
        PxPosition(IVec2::new(48, 16)),
        PxPresentationTransform {
            scale: Vec2::new(-1.0, -1.0),
            rotation: std::f32::consts::FRAC_PI_4,
            ..Default::default()
        },
        Layer::Front,
    ));
}

#[px_layer]
enum Layer {
    #[default]
    Back,
    Front,
}
