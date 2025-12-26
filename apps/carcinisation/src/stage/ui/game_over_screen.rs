pub mod components;
pub mod input;
pub mod messages;
mod systems;

use self::{
    components::*,
    input::{init_input, GameOverScreenInput},
    messages::GameOverScreenShutdownMessage,
    systems::{check_press_continue_input, handle_game_over_screen_continue},
};
use crate::{
    components::GBColor,
    game::score::components::Score,
    globals::{
        mark_for_despawn_by_query, SCREEN_RESOLUTION_F32_H, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    stage::StageProgressState,
};
use crate::{
    globals::SCREEN_RESOLUTION_H,
    pixel::{PxAssets, PxRectBundle, PxTextBundle},
};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxPosition, PxRect, PxSubPosition, PxText,
    PxTypeface,
};

pub const HALF_SCREEN_SIZE: i32 = 70;

pub fn render_game_over_screen(
    mut commands: Commands,
    assets_typeface: PxAssets<PxTypeface>,
    filters: PxAssets<PxFilter>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::GameOver {
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
        let score_text = score.value.to_string();

        commands.spawn((
            GameOverScreen,
            InheritedVisibility::VISIBLE,
            Name::new("GameOver Screen"),
            Visibility::Visible,
            children![
                (
                    PxSubPosition(*SCREEN_RESOLUTION_F32_H),
                    Name::new("UIBackground"),
                    UIBackground,
                    PxRectBundle::<Layer> {
                        rect: PxRect(UVec2::new(120, 90)),
                        position: PxPosition::from(*SCREEN_RESOLUTION_H),
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        layers: PxFilterLayers::single_over(Layer::UIBackground),
                        filter: PxFilter(filters.load_color(GBColor::White)),
                        visibility: Visibility::Visible,
                    },
                ),
                (
                    PxTextBundle::<Layer> {
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        position: PxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, 90)),
                        text: PxText {
                            value: "Game Over".to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Stage_GameOver"),
                ),
                (
                    PxTextBundle::<Layer> {
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        position: PxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, 60)),
                        text: PxText {
                            value: "Score:".to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Score"),
                ),
                (
                    PxTextBundle::<Layer> {
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        position: PxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, 50)),
                        text: PxText {
                            value: score_text.clone(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    FinalScoreText,
                    Name::new("FinalScoreText"),
                )
            ],
        ));
    }
}

pub fn despawn_game_over_screen(
    mut commands: Commands,
    stage_state: Res<State<StageProgressState>>,
    query: Query<Entity, With<GameOverScreen>>,
) {
    if stage_state.is_changed() && *stage_state.get() != StageProgressState::GameOver {
        mark_for_despawn_by_query(&mut commands, &query);
    }
}

#[derive(Activable)]
pub struct GameOverScreenPlugin;

impl Plugin for GameOverScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameOverScreenShutdownMessage>()
            .add_plugins(InputManagerPlugin::<GameOverScreenInput>::default())
            .add_systems(Startup, init_input)
            .add_active_systems::<GameOverScreenPlugin, _>((
                render_game_over_screen,
                despawn_game_over_screen,
            ))
            .add_active_systems_in::<GameOverScreenPlugin, _>(
                PostUpdate,
                (check_press_continue_input, handle_game_over_screen_continue).chain(),
            );
    }
}
