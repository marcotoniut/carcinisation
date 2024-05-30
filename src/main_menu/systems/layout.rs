use super::super::components::*;
use crate::{
    game::resources::Difficulty,
    globals::{
        mark_for_despawn_by_component_query, GBColor, SCREEN_RESOLUTION, SCREEN_RESOLUTION_F32,
        TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    main_menu::{events::ChangeMainMenuScreenEvent, MainMenuScreen},
    pixel::components::PxRectangle,
    Layer,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxCanvas, PxSubPosition, PxTextBundle, PxTypeface},
    sprite::{PxSprite, PxSpriteBundle},
};
use strum::IntoEnumIterator;

pub fn on_change_main_menu_screen(
    mut commands: Commands,
    difficulty_select_query: Query<Entity, With<DifficultySelectScreenEntity>>,
    press_start_query: Query<Entity, With<PressStartScreenEntity>>,
    main_menu_select_query: Query<Entity, With<MainMenuSelectScreenEntity>>,
    mut event_reader: EventReader<ChangeMainMenuScreenEvent>,
    mut screen: ResMut<MainMenuScreen>,
) {
    for e in event_reader.read() {
        match e.0 {
            MainMenuScreen::DifficultySelect => {
                mark_for_despawn_by_component_query(&mut commands, &difficulty_select_query)
            }
            MainMenuScreen::MainMenuSelect => {
                mark_for_despawn_by_component_query(&mut commands, &main_menu_select_query)
            }
            MainMenuScreen::PressStart => {
                mark_for_despawn_by_component_query(&mut commands, &press_start_query)
            }
        }
        *screen = e.0.clone();
    }
}

pub fn spawn_main_menu(mut commands: Commands, mut assets_sprite: PxAssets<PxSprite>) {
    let entity = commands.spawn(MainMenu).id();

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
    });
}

pub fn spawn_press_start_screen(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    screen: Res<MainMenuScreen>,
) {
    if screen.is_changed() && *screen.as_ref() == MainMenuScreen::PressStart {
        let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

        commands.spawn((
            MainMenuEntity,
            PressStartScreenEntity,
            PxTextBundle::<Layer> {
                alignment: PxAnchor::Center,
                canvas: PxCanvas::Camera,
                // TODO Menu layers
                layer: Layer::Hud,
                rect: IRect::new(0, 0, SCREEN_RESOLUTION.x as i32, 60).into(),
                text: "Press Start".into(),
                typeface: typeface.clone(),
                ..Default::default()
            },
            Name::new("Text<PressStart>"),
        ));
    }
}

pub fn spawn_game_difficulty_screen(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    screen: Res<MainMenuScreen>,
) {
    if screen.is_changed() && *screen.as_ref() == MainMenuScreen::DifficultySelect {
        let color = GBColor::White;
        let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

        commands.spawn((
            MainMenuEntity,
            DifficultySelectScreenEntity,
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
        ));

        for d in Difficulty::iter() {
            let name = match d {
                Difficulty::Easy => "Easy",
                Difficulty::Normal => "Normal",
                Difficulty::Hard => "Hard",
            };

            commands.spawn((
                MainMenuEntity,
                DifficultySelectScreenEntity,
                // Vertical offset
                PxSubPosition(Vec2::new(
                    SCREEN_RESOLUTION_F32.x / 2.,
                    SCREEN_RESOLUTION_F32.y / 2.,
                )),
                PxTextBundle::<Layer> {
                    alignment: PxAnchor::Center,
                    canvas: PxCanvas::Camera,
                    // TODO Menu layers
                    layer: Layer::Hud,
                    rect: IRect::new(0, 0, SCREEN_RESOLUTION.x as i32, 60).into(),
                    text: name.clone().into(),
                    typeface: typeface.clone(),
                    ..Default::default()
                },
                Name::new(format!("Text<{}>", name.clone())),
            ));
        }
    }
}
