#![allow(clippy::needless_pass_by_value)]
// In this program, clicking a sprite highlights the clicked pixel with a filter.
//
// TODO: CxFilterRect was removed from the public API. The pixel-highlight
// feature needs to be updated to use the replacement API.

use bevy::prelude::*;
use bevy_picking::prelude::{Click, Pointer, PointerButton};
use carapace::prelude::*;

#[derive(Resource, Default)]
struct PickedPixel(#[allow(dead_code)] Option<Entity>);

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: UVec2::splat(512).into(),
                ..default()
            }),
            ..default()
        }),
        CxPlugin::<Layer>::new(UVec2::splat(32), "palette/palette_1.palette.png"),
    ));

    app.insert_resource(ClearColor(Color::BLACK))
        .init_resource::<PickedPixel>()
        .add_systems(Startup, init)
        .add_systems(Update, highlight_on_click)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let sprite = assets.load("sprite/mage.px_sprite.png");

    commands.spawn((
        CxSprite(sprite),
        CxPick,
        CxPosition(IVec2::new(8, 8)),
        CxAnchor::BottomLeft,
        CxRenderSpace::Camera,
    ));
}

fn highlight_on_click(
    mut clicks: MessageReader<Pointer<Click>>,
    _cursor: Res<CxCursorPosition>,
    _camera: Res<CxCamera>,
    sprites: Query<(&Layer, &CxRenderSpace), With<CxPick>>,
    mut _commands: Commands,
    mut _picked: ResMut<PickedPixel>,
    _assets: Res<AssetServer>,
) {
    for click in clicks.read() {
        if click.event.button != PointerButton::Primary {
            continue;
        }

        let Ok((_layer, _canvas)) = sprites.get(click.entity) else {
            continue;
        };

        // TODO: CxFilterRect was removed. Pixel-highlight spawn logic needs
        // to be updated to use the replacement API.
    }
}

#[px_layer]
struct Layer;
