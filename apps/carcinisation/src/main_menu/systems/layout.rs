use super::super::components::{
    DifficultySelectScreenEntity, DifficultySelectionIndicator, MainMenu, MainMenuEntity,
    PressStartScreenEntity,
};
use crate::{
    game::resources::Difficulty,
    globals::{
        FONT_SIZE, SCREEN_RESOLUTION, SCREEN_RESOLUTION_F32, SCREEN_RESOLUTION_F32_H,
        SCREEN_RESOLUTION_H, load_inverted_typeface, mark_for_despawn_by_query,
    },
    layer::Layer,
    main_menu::{MainMenuScreen, resources::DifficultySelection},
    pixel::{
        CxAssets,
        bundle::{CxSpriteBundle, CxTextBundle},
    },
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxPosition, CxRenderSpace, CxSprite, CxText, CxTypeface, WorldPos,
};
use carapace::primitive::{CxPrimitive, CxPrimitiveFill, CxPrimitiveShape};
use strum::IntoEnumIterator;

/// @system Spawns the main menu background entity.
pub fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let entity = commands
        .spawn((MainMenu, Visibility::Visible, InheritedVisibility::VISIBLE))
        .id();

    let mut entity_commands = commands.entity(entity);
    entity_commands.with_children(|p0| {
        let background_sprite =
            asset_server.load(assert_assets_path!("ui/main_menu/background.px_sprite.png"));
        p0.spawn((
            CxSpriteBundle::<Layer> {
                sprite: CxSprite(background_sprite),
                anchor: CxAnchor::BottomLeft,
                layer: Layer::Hud,
                ..default()
            },
            Name::new("MainMenuBackground"),
        ));
    });
}

/// @system Spawns the "Press Start" text when entering that screen state.
pub fn enter_press_start_screen(mut commands: Commands, assets_typeface: CxAssets<CxTypeface>) {
    let typeface = load_inverted_typeface(&assets_typeface);

    commands.spawn((
        MainMenuEntity,
        PressStartScreenEntity,
        CxTextBundle::<Layer> {
            position: CxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, 10)),
            anchor: CxAnchor::BottomCenter,
            canvas: CxRenderSpace::Camera,
            layer: Layer::UI,
            text: CxText {
                value: "Press Start".to_string(),
                typeface: typeface.clone(),
                ..Default::default()
            },
            ..default()
        },
        Name::new("Text<PressStart>"),
    ));
}

/// @system Despawns press-start screen entities.
pub fn exit_press_start_screen(
    mut commands: Commands,
    press_start_query: Query<Entity, With<PressStartScreenEntity>>,
) {
    mark_for_despawn_by_query(&mut commands, &press_start_query);
}

/// @system Builds the difficulty selection UI when that screen activates.
pub fn enter_game_difficulty_screen(
    mut commands: Commands,
    assets_typeface: CxAssets<CxTypeface>,
    selection: Res<DifficultySelection>,
) {
    let typeface = load_inverted_typeface(&assets_typeface);

    commands.spawn((
        MainMenuEntity,
        DifficultySelectScreenEntity,
        CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                // TODO use more memory by making this rect into a global?
                size: UVec2::new(SCREEN_RESOLUTION.x - 50, SCREEN_RESOLUTION.y - 50),
            },
            fill: CxPrimitiveFill::Solid(4),
        },
        CxAnchor::Center,
        CxRenderSpace::Camera,
        CxPosition::from(*SCREEN_RESOLUTION_H),
        Layer::UIBackground,
        // TODO should not be using WorldPos here
        WorldPos(*SCREEN_RESOLUTION_F32_H),
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
            CxTextBundle::<Layer> {
                position: CxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, y)),
                anchor: CxAnchor::Center,
                canvas: CxRenderSpace::Camera,
                layer: Layer::UI,
                text: CxText {
                    value: name.to_string(),
                    typeface: typeface.clone(),
                    ..Default::default()
                },
                ..default()
            },
            Name::new(format!("Text<{name}>")),
        ));
    }

    if let Some(selection_index) = difficulty_index(selection.0) {
        commands.spawn((
            MainMenuEntity,
            DifficultySelectScreenEntity,
            DifficultySelectionIndicator,
            CxTextBundle::<Layer> {
                position: CxPosition::from(difficulty_arrow_position(selection_index)),
                anchor: CxAnchor::CenterRight,
                canvas: CxRenderSpace::Camera,
                layer: Layer::UI,
                text: CxText {
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

/// @system Despawns difficulty-select screen entities.
pub fn exit_game_difficulty_screen(
    mut commands: Commands,
    difficulty_select_query: Query<Entity, With<DifficultySelectScreenEntity>>,
) {
    mark_for_despawn_by_query(&mut commands, &difficulty_select_query);
}

/// @system Moves the arrow indicator when the selected difficulty changes.
pub fn update_difficulty_selection_indicator(
    selection: Res<DifficultySelection>,
    screen: Res<State<MainMenuScreen>>,
    mut indicator_query: Query<&mut CxPosition, With<DifficultySelectionIndicator>>,
) {
    if **screen != MainMenuScreen::DifficultySelect || !selection.is_changed() {
        return;
    }

    if let (Ok(mut position), Some(selection_index)) =
        (indicator_query.single_mut(), difficulty_index(selection.0))
    {
        *position = CxPosition::from(difficulty_arrow_position(selection_index));
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
    let arrow_x = SCREEN_RESOLUTION_H.x - 30;
    IVec2::new(arrow_x, option_y)
}
