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
            PxPlugin::<Layer>::new(UVec2::splat(16), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .add_systems(Update, animate_composite)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let runner = assets.load("sprite/runner.px_sprite.png");

    let composite = PxCompositeSprite::new(vec![
        PxCompositePart {
            sprite: runner.clone(),
            offset: IVec2::ZERO,
            frame: PxFrameBinding::default(),
            filter: None,
        },
        PxCompositePart {
            sprite: runner,
            offset: IVec2::new(8, 0),
            frame: PxFrameBinding::default(),
            filter: None,
        },
    ]);

    commands.spawn((
        composite,
        PxPosition(IVec2::splat(8)),
        PxFrameControl::from(PxFrameSelector::Normalized(0.)),
        CompositeAnimator,
    ));
}

fn animate_composite(
    time: Res<Time>,
    mut query: Query<&mut PxFrameControl, With<CompositeAnimator>>,
) {
    let progress = (time.elapsed_secs() * 1.5).fract();
    for mut frame in &mut query {
        frame.selector = PxFrameSelector::Normalized(progress);
        frame.transition = PxFrameTransition::None;
    }
}

#[px_layer]
struct Layer;
