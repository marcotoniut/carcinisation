#![allow(clippy::needless_pass_by_value)]
// In this program, clicking a sprite highlights the clicked pixel with a filter.

use bevy::prelude::*;
use bevy_picking::prelude::{Click, Pickable, Pointer, PointerButton};
use carapace::prelude::*;

#[derive(Resource, Default)]
struct PickedPixel(Option<Entity>);

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
    cursor: Res<CxCursorPosition>,
    camera: Res<CxCamera>,
    sprites: Query<(&Layer, &CxRenderSpace), With<CxPick>>,
    mut commands: Commands,
    mut picked: ResMut<PickedPixel>,
    assets: Res<AssetServer>,
) {
    let Some(cursor) = **cursor else {
        return;
    };
    let cursor = cursor.as_ivec2();

    for click in clicks.read() {
        if click.event.button != PointerButton::Primary {
            continue;
        }

        let Ok((layer, canvas)) = sprites.get(click.entity) else {
            continue;
        };

        let mut highlight_pos = cursor;
        if matches!(canvas, CxRenderSpace::World) {
            highlight_pos += **camera;
        }

        let entity = if let Some(entity) = picked.0 {
            entity
        } else {
            let entity = commands
                .spawn((
                    CxFilterRect(UVec2::ONE),
                    CxAnchor::BottomLeft,
                    CxFilter(assets.load("filter/invert.px_filter.png")),
                    Pickable::IGNORE,
                ))
                .id();
            picked.0 = Some(entity);
            entity
        };

        commands.entity(entity).insert((
            CxPosition(highlight_pos),
            layer.clone(),
            *canvas,
            CxFilterLayers::single_clip(layer.clone()),
        ));
    }
}

#[px_layer]
struct Layer;
