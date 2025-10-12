use super::super::components::*;
use crate::{
    components::GBColor,
    game::resources::Difficulty,
    globals::{
        mark_for_despawn_by_query, SCREEN_RESOLUTION, SCREEN_RESOLUTION_F32, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    main_menu::{events::ChangeMainMenuScreenTrigger, MainMenuScreen},
    pixel::components::PxRectangle,
    pixel::{
        bundle::{PxSpriteBundle, PxTextBundle},
        PxAssets,
    },
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxCanvas, PxSprite, PxSubPosition, PxText, PxTypeface};
use strum::IntoEnumIterator;

pub fn on_change_main_menu_screen(
    trigger: Trigger<ChangeMainMenuScreenTrigger>,
    mut commands: Commands,
    difficulty_select_query: Query<Entity, With<DifficultySelectScreenEntity>>,
    press_start_query: Query<Entity, With<PressStartScreenEntity>>,
    main_menu_select_query: Query<Entity, With<MainMenuSelectScreenEntity>>,
    mut screen: ResMut<MainMenuScreen>,
) {
    let e = trigger.event();
    match e.0 {
        MainMenuScreen::DifficultySelect => {
            mark_for_despawn_by_query(&mut commands, &difficulty_select_query)
        }
        MainMenuScreen::MainMenuSelect => {
            mark_for_despawn_by_query(&mut commands, &main_menu_select_query)
        }
        MainMenuScreen::PressStart => mark_for_despawn_by_query(&mut commands, &press_start_query),
    }
    *screen = e.0.clone();
}

pub fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let entity = commands.spawn((MainMenu, Visibility::Visible)).id();

    let mut entity_commands = commands.entity(entity);
    entity_commands.with_children(|p0| {
        let background_sprite =
            asset_server.load(assert_assets_path!("ui/main_menu/background.px_sprite.png"));
        p0.spawn((
            PxSpriteBundle::<Layer> {
                sprite: PxSprite(background_sprite),
                anchor: PxAnchor::BottomLeft,
                layer: Layer::Hud,
                ..default()
            },
            Name::new("MainMenuBackground"),
        ));
    });
}

pub fn spawn_press_start_screen(
    mut commands: Commands,
    mut assets_typeface: PxAssets<PxTypeface>,
    screen: Res<MainMenuScreen>,
) {
    if screen.is_changed() && *screen.as_ref() == MainMenuScreen::PressStart {
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

        commands.spawn((
            MainMenuEntity,
            PressStartScreenEntity,
            PxTextBundle::<Layer> {
                alignment: PxAnchor::Center,
                canvas: PxCanvas::Camera,
                // TODO Menu layers
                layer: Layer::Hud,
                rect: IRect::new(0, 0, SCREEN_RESOLUTION.x as i32, 60).into(),
                text: PxText {
                    value: "Press Start".to_string(),
                    typeface: typeface.clone(),
                },
                ..default()
            },
            Name::new("Text<PressStart>"),
        ));
    }
}

/// @system Builds the difficulty selection UI when that screen activates.
pub fn spawn_game_difficulty_screen(
    mut commands: Commands,
    mut assets_typeface: PxAssets<PxTypeface>,
    screen: Res<MainMenuScreen>,
) {
    if screen.is_changed() && *screen.as_ref() == MainMenuScreen::DifficultySelect {
        let color = GBColor::White;
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

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
            Visibility::Visible,
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
                    text: PxText {
                        value: name.to_string(),
                        typeface: typeface.clone(),
                    },
                    ..default()
                },
                Name::new(format!("Text<{}>", name)),
            ));
        }
    }
}
