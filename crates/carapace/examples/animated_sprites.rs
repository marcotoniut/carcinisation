#![allow(clippy::needless_pass_by_value)]
// In this program, animated sprites are spawned

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
            CxPlugin::<Layer>::new(UVec2::new(51, 35), "palette/palette_1.palette.png"),
            CxAnimationPlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    // Load an animated sprite with `add_animated`
    let runner = assets.load("sprite/runner.px_sprite.png");

    // Despawn at the end
    commands.spawn((
        CxSprite(runner.clone()),
        CxAnchor::BottomLeft,
        CxAnimation::default(),
    ));

    // Add the `CxAnimationFinished` component at the end
    commands.spawn((
        CxSprite(runner.clone()),
        CxPosition(IVec2::new(13, 0)),
        CxAnchor::BottomLeft,
        CxAnimation {
            on_finish: CxAnimationFinishBehavior::Mark,
            ..default()
        },
    ));

    // Loop
    commands.spawn((
        CxSprite(runner.clone()),
        CxPosition(IVec2::new(26, 0)),
        CxAnchor::BottomLeft,
        CxAnimation {
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Backward
    commands.spawn((
        CxSprite(runner.clone()),
        CxPosition(IVec2::new(39, 0)),
        CxAnchor::BottomLeft,
        CxAnimation {
            direction: CxAnimationDirection::Backward,
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Faster
    commands.spawn((
        CxSprite(runner.clone()),
        CxPosition(IVec2::new(13, 18)),
        CxAnchor::BottomLeft,
        CxAnimation {
            duration: CxAnimationDuration::millis_per_animation(500),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Slower
    commands.spawn((
        CxSprite(runner.clone()),
        CxPosition(IVec2::new(0, 18)),
        CxAnchor::BottomLeft,
        CxAnimation {
            duration: CxAnimationDuration::millis_per_animation(2000),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Duration per frame
    commands.spawn((
        CxSprite(runner.clone()),
        CxPosition(IVec2::new(26, 18)),
        CxAnchor::BottomLeft,
        CxAnimation {
            duration: CxAnimationDuration::millis_per_frame(1000),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Dither between frames
    commands.spawn((
        CxSprite(runner),
        CxPosition(IVec2::new(39, 18)),
        CxAnchor::BottomLeft,
        CxAnimation {
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
        CxFrameView {
            transition: CxFrameTransition::Dither,
            ..default()
        },
    ));
}

#[px_layer]
struct Layer;
