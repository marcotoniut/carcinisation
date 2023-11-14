use super::super::components::*;
use crate::{
    cutscene::data::CutsceneLayer,
    globals::{
        mark_for_despawn_by_component_query, GBColor, SCREEN_RESOLUTION, SCREEN_RESOLUTION_F32,
        TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    pixel::components::PxRectangle,
    Layer,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxCanvas, PxPosition, PxSubPosition, PxTextBundle, PxTypeface,
    },
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

pub fn spawn_main_menu_select(mut commands: Commands) {
    let color = GBColor::White;

    commands
        .spawn((
            MainMenuSelect,
            MainMenuEntity,
            PxSubPosition(Vec2::new(
                SCREEN_RESOLUTION_F32.x / 2.,
                SCREEN_RESOLUTION_F32.y / 2.,
            )),
            PxRectangle {
                anchor: PxAnchor::Center,
                canvas: PxCanvas::Camera,
                color,
                width: SCREEN_RESOLUTION.x - 50,
                height: SCREEN_RESOLUTION.y - 50,
                layer: Layer::Hud,
            },
        ))
        .id();
}
