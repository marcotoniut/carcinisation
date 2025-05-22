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
    let entity = commands.spawn(MainMenu).id();

    let mut entity_commands = commands.entity(entity);
    entity_commands.with_children(|p0| {
        let background_sprite =
            PxSprite(asset_server.load(assert_assets_path!("ui/main_menu/background.png")));
        p0.spawn((
            background_sprite,
            PxAnchor::BottomLeft,
            Layer::Hud,
            Name::new("MainMenuBackground"),
        ));
    });
}

pub fn spawn_press_start_screen(
    mut commands: Commands,
    asset_server: &Res<AssetServer>,
    screen: Res<MainMenuScreen>,
) {
    if screen.is_changed() && *screen.as_ref() == MainMenuScreen::PressStart {
        let typeface = PxTypeface(
            asset_server.load(TYPEFACE_INVERTED_PATH),
            TYPEFACE_CHARACTERS,
            [(' ', 4)],
        );

        commands.spawn((
            MainMenuEntity,
            PressStartScreenEntity,
            PxAnchor::Center,
            PxCanvas::Camera,
            Layer::Hud,
            PxRectangle(IRect::new(0, 0, SCREEN_RESOLUTION.x as i32, 60)),
            PxText {
                value: "Press Start".to_string(),
                typeface: typeface.clone(),
            },
            Name::new("Text<PressStart>"),
        ));
    }
}

pub fn spawn_game_difficulty_screen(
    mut commands: Commands,
    asset_server: &Res<AssetServer>,
    screen: Res<MainMenuScreen>,
) {
    if screen.is_changed() && *screen.as_ref() == MainMenuScreen::DifficultySelect {
        let color = GBColor::White;
        let typeface = PxTypeface(
            asset_server.load(TYPEFACE_INVERTED_PATH),
            TYPEFACE_CHARACTERS,
            [(' ', 4)],
        );

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
                PxAnchor::Center,
                PxCanvas::Camera,
                Layer::Hud,
                PxRectangle(IRect::new(0, 0, SCREEN_RESOLUTION.x as i32, 60)),
                PxText {
                    value: name.to_string(),
                    typeface: typeface.clone(),
                },
                Name::new(format!("Text<{}>", name)),
            ));
        }
    }
}
