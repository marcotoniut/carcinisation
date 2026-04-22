#![allow(clippy::needless_pass_by_value)]
// In this program, animated filters are demonstrated

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

    let mage = assets.load("sprite/mage.px_sprite.png");

    // Spawn a bunch of sprites on different layers
    for layer in 0..8 {
        commands.spawn((
            CxSprite(mage.clone()),
            CxPosition(IVec2::new(layer % 4 * 13, layer / 4 * 18)),
            CxAnchor::BottomLeft,
            Layer(layer),
        ));
    }

    // Load the filter
    let fade_to_black = assets.load("filter/fade_to_black.px_filter.png");

    // Despawn at the end
    commands.spawn((
        CxFilter(fade_to_black.clone()),
        CxFilterLayers::single_clip(Layer(0)),
        CxAnimation::default(),
    ));

    // Add the `CxAnimationFinished` component at the end
    commands.spawn((
        CxFilter(fade_to_black.clone()),
        CxFilterLayers::single_clip(Layer(1)),
        CxAnimation {
            on_finish: CxAnimationFinishBehavior::Mark,
            ..default()
        },
    ));

    // Loop
    commands.spawn((
        CxFilter(fade_to_black.clone()),
        CxFilterLayers::single_clip(Layer(2)),
        CxAnimation {
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Backward
    commands.spawn((
        CxFilter(fade_to_black.clone()),
        CxFilterLayers::single_clip(Layer(3)),
        CxAnimation {
            direction: CxAnimationDirection::Backward,
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Faster
    commands.spawn((
        CxFilter(fade_to_black.clone()),
        CxFilterLayers::single_clip(Layer(5)),
        CxAnimation {
            duration: CxAnimationDuration::millis_per_animation(500),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Slower
    commands.spawn((
        CxFilter(fade_to_black.clone()),
        CxFilterLayers::single_clip(Layer(4)),
        CxAnimation {
            duration: CxAnimationDuration::millis_per_animation(2000),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Duration per frame
    commands.spawn((
        CxFilter(fade_to_black.clone()),
        CxFilterLayers::single_clip(Layer(6)),
        CxAnimation {
            duration: CxAnimationDuration::millis_per_frame(1000),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
    ));

    // Dither between frames
    commands.spawn((
        CxFilter(fade_to_black),
        CxFilterLayers::single_clip(Layer(7)),
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
struct Layer(i32);
