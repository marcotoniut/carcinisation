#![allow(clippy::needless_pass_by_value)]
// In this program, animated text is spawned

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
            CxAnimationPlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn text(
    value: impl Into<String>,
    transition: CxFrameTransition,
    assets: &AssetServer,
) -> impl Bundle {
    (
        CxText::new(
            value,
            assets.load("typeface/animated_typeface.px_typeface.png"),
        ),
        CxAnimation {
            // Use millis_per_animation to have each character loop at the same time
            duration: CxAnimationDuration::millis_per_frame(333),
            on_finish: CxAnimationFinishBehavior::Loop,
            ..default()
        },
        CxFrameView {
            transition,
            ..default()
        },
    )
}

fn init(assets: Res<AssetServer>, mut cmd: Commands) {
    cmd.spawn(Camera2d);

    cmd.spawn((
        Layer,
        CxUiRoot,
        CxRow {
            vertical: true,
            ..default()
        },
        children![
            text("LOOPED ANIMATION ⭐🙂⭐", CxFrameTransition::None, &assets),
            CxRowSlot { stretch: true },
            text(
                "DITHERED ANIMATION 🙂⭐🙂",
                CxFrameTransition::Dither,
                &assets
            ),
        ],
    ));
}

#[px_layer]
struct Layer;
