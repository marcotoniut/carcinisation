#![allow(clippy::needless_pass_by_value)]
// In this program, a composite sprite is rendered via the GPU palette path.

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
            CxPlugin::<Layer>::new(UVec2::splat(16), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let base = assets.load("sprite/mage.px_sprite.png");
    let overlay = assets.load("sprite/snow_2.px_sprite.png");

    let composite = CxCompositeSprite::new(vec![
        CxCompositePart::new(base),
        CxCompositePart::new(overlay).with_offset(IVec2::new(2, 6)),
    ]);

    commands.spawn((composite, CxGpuComposite, CxPosition(IVec2::splat(8))));
}

#[px_layer]
struct Layer;
