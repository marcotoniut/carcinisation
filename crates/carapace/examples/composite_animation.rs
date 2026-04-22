#![allow(clippy::needless_pass_by_value)]
// In this program, a composed sprite is animated by driving a master frame manually.

use bevy::prelude::*;
use carapace::prelude::*;

#[derive(Component)]
struct CompositeAnimator;

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
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .add_systems(Update, animate_composite)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let runner = assets.load("sprite/runner.px_sprite.png");

    let composite = CxCompositeSprite::new(vec![
        CxCompositePart::new(runner.clone()),
        CxCompositePart::new(runner).with_offset(IVec2::new(8, 0)),
    ]);

    commands.spawn((
        composite,
        CxPosition(IVec2::splat(8)),
        CxFrameControl::from(CxFrameSelector::Normalized(0.)),
        CompositeAnimator,
    ));
}

fn animate_composite(
    time: Res<Time>,
    mut query: Query<&mut CxFrameControl, With<CompositeAnimator>>,
) {
    let progress = (time.elapsed_secs() * 1.5).fract();
    for mut frame in &mut query {
        frame.selector = CxFrameSelector::Normalized(progress);
        frame.transition = CxFrameTransition::None;
    }
}

#[px_layer]
struct Layer;
