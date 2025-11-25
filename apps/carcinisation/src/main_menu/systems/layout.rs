use super::super::components::*;
use crate::{
    components::GBColor,
    game::resources::Difficulty,
    globals::{
        mark_for_despawn_by_query, FONT_SIZE, HALF_SCREEN_RESOLUTION, SCREEN_RESOLUTION,
        SCREEN_RESOLUTION_F32, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    main_menu::{resources::DifficultySelection, MainMenuScreen},
    pixel::{
        bundle::{PxSpriteBundle, PxTextBundle},
        components::PxRectangle,
        PxAssets,
    },
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxPosition, PxSprite, PxSubPosition, PxText, PxTypeface,
};
use strum::IntoEnumIterator;

pub fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let entity = commands
        .spawn((MainMenu, Visibility::Visible, InheritedVisibility::VISIBLE))
        .id();

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

pub fn enter_press_start_screen(mut commands: Commands, assets_typeface: PxAssets<PxTypeface>) {
    let typeface = assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    commands.spawn((
        MainMenuEntity,
        PressStartScreenEntity,
        PxTextBundle::<Layer> {
            position: PxPosition::from(IVec2::new(HALF_SCREEN_RESOLUTION.x, 10)),
            anchor: PxAnchor::BottomCenter,
            canvas: PxCanvas::Camera,
            layer: Layer::UI,
            text: PxText {
                value: "Press Start".to_string(),
                typeface: typeface.clone(),
                ..Default::default()
            },
            ..default()
        },
        Name::new("Text<PressStart>"),
    ));
}

pub fn exit_press_start_screen(
    mut commands: Commands,
    press_start_query: Query<Entity, With<PressStartScreenEntity>>,
) {
    mark_for_despawn_by_query(&mut commands, &press_start_query);
}

/// Builds the difficulty selection UI when that screen activates.
pub fn enter_game_difficulty_screen(
    mut commands: Commands,
    assets_typeface: PxAssets<PxTypeface>,
    selection: Res<DifficultySelection>,
) {
    let color = GBColor::White;
    let typeface = assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

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
            layer: Layer::UIBackground,
        },
        Visibility::Visible,
    ));

    for (index, d) in Difficulty::iter().enumerate() {
        let name = match d {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        };
        let y = difficulty_option_y(index);
        commands.spawn((
            MainMenuEntity,
            DifficultySelectScreenEntity,
            PxTextBundle::<Layer> {
                position: PxPosition::from(IVec2::new(HALF_SCREEN_RESOLUTION.x, y)),
                anchor: PxAnchor::Center,
                canvas: PxCanvas::Camera,
                layer: Layer::UI,
                text: PxText {
                    value: name.to_string(),
                    typeface: typeface.clone(),
                    ..Default::default()
                },
                ..default()
            },
            Name::new(format!("Text<{}>", name)),
        ));
    }

    if let Some(selection_index) = difficulty_index(selection.0) {
        commands.spawn((
            MainMenuEntity,
            DifficultySelectScreenEntity,
            DifficultySelectionIndicator,
            PxTextBundle::<Layer> {
                position: PxPosition::from(difficulty_arrow_position(selection_index)),
                anchor: PxAnchor::CenterRight,
                canvas: PxCanvas::Camera,
                layer: Layer::UI,
                text: PxText {
                    value: ">".to_string(),
                    typeface,
                    ..Default::default()
                },
                ..default()
            },
            Name::new("DifficultySelectionIndicator"),
        ));
    }
}

pub fn exit_game_difficulty_screen(
    mut commands: Commands,
    difficulty_select_query: Query<Entity, With<DifficultySelectScreenEntity>>,
) {
    mark_for_despawn_by_query(&mut commands, &difficulty_select_query);
}

/// Moves the arrow indicator when the selected difficulty changes.
pub fn update_difficulty_selection_indicator(
    selection: Res<DifficultySelection>,
    screen: Res<State<MainMenuScreen>>,
    mut indicator_query: Query<&mut PxPosition, With<DifficultySelectionIndicator>>,
) {
    if **screen != MainMenuScreen::DifficultySelect || !selection.is_changed() {
        return;
    }

    if let (Ok(mut position), Some(selection_index)) =
        (indicator_query.single_mut(), difficulty_index(selection.0))
    {
        *position = PxPosition::from(difficulty_arrow_position(selection_index));
    }
}

fn difficulty_index(target: Difficulty) -> Option<usize> {
    Difficulty::iter().position(|d| d == target)
}

fn difficulty_option_y(index: usize) -> i32 {
    let total = Difficulty::iter().len() as f32;
    let spacing = FONT_SIZE as f32 + 8.0;
    let vertical_origin = SCREEN_RESOLUTION_F32.y / 2.;
    let offset = (total - 1.0) * 0.5 - index as f32;
    (vertical_origin + offset * spacing).round() as i32
}

fn difficulty_arrow_position(index: usize) -> IVec2 {
    let option_y = difficulty_option_y(index);
    let arrow_x = HALF_SCREEN_RESOLUTION.x - 30;
    IVec2::new(arrow_x, option_y)
}
