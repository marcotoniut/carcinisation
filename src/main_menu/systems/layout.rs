use super::super::components::*;
use crate::{
    globals::{
        mark_for_despawn_by_component_query, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    Layer,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{IRect, PxAnchor, PxAssets, PxCanvas, PxTextBundle, PxTypeface},
    sprite::{PxSprite, PxSpriteBundle},
};

pub fn spawn_main_menu(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut typefaces: PxAssets<PxTypeface>,
) {
    let entity = commands.spawn((MainMenu,)).id();
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let mut entity_commands = commands.entity(entity);
    entity_commands.with_children(|parent| {
        let background_sprite =
            assets_sprite.load(assert_assets_path!("ui/main_menu/background.png"));
        parent.spawn((
            PxSpriteBundle::<Layer> {
                sprite: background_sprite,
                anchor: PxAnchor::BottomLeft,
                layer: Layer::Hud,
                ..Default::default()
            },
            Name::new("MainMenuBackground"),
        ));

        parent.spawn((
            PxTextBundle::<Layer> {
                alignment: PxAnchor::Center,
                canvas: PxCanvas::Camera,
                // TODO Menu layers
                layer: Layer::Hud,
                rect: IRect::new(IVec2::ZERO, IVec2::new(SCREEN_RESOLUTION.x as i32, 60)).into(),
                text: "Press Start".into(),
                typeface: typeface.clone(),
                ..Default::default()
            },
            Name::new("Text<PressStart>"),
        ));
    });
}

pub fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenu>>) {
    mark_for_despawn_by_component_query(&mut commands, &query)
}
