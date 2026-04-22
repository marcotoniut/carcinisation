pub mod components;
pub mod input;
pub mod messages;
mod systems;

use self::{
    components::{FinalScoreText, GameOverScreen, InfoText, UIBackground},
    input::{GameOverScreenInput, init_input},
    messages::GameOverScreenShutdownMessage,
    systems::{check_press_continue_input, handle_game_over_screen_continue},
};
use super::StageUiPlugin;
use crate::{
    components::GBColor,
    game::score::components::Score,
    globals::{SCREEN_RESOLUTION_F32_H, load_inverted_typeface, mark_for_despawn_by_query},
    layer::Layer,
    stage::StageProgressState,
};
use crate::{
    globals::SCREEN_RESOLUTION_H,
    pixel::{CxAssets, CxFilterRectBundle, CxTextBundle},
};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxFilter, CxFilterLayers, CxFilterRect, CxPosition, CxRenderSpace, CxText,
    CxTypeface, WorldPos,
};
use leafwing_input_manager::plugin::InputManagerPlugin;

pub fn render_game_over_screen(
    mut commands: Commands,
    assets_typeface: CxAssets<CxTypeface>,
    filters: CxAssets<CxFilter>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::GameOver {
        let typeface = load_inverted_typeface(&assets_typeface);
        let score_text = score.value.to_string();

        commands.spawn((
            GameOverScreen,
            Name::new("GameOver Screen"),
            Visibility::Visible,
            children![
                (
                    WorldPos(*SCREEN_RESOLUTION_F32_H),
                    Name::new("UIBackground"),
                    UIBackground,
                    CxFilterRectBundle::<Layer> {
                        rect: CxFilterRect(UVec2::new(120, 90)),
                        position: CxPosition::from(*SCREEN_RESOLUTION_H),
                        anchor: CxAnchor::Center,
                        canvas: CxRenderSpace::Camera,
                        layers: CxFilterLayers::single_over(Layer::UIBackground),
                        filter: CxFilter(filters.load_color(GBColor::White)),
                        visibility: Visibility::Visible,
                    },
                ),
                (
                    CxTextBundle::<Layer> {
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        position: CxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, 90)),
                        text: CxText {
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
                    CxTextBundle::<Layer> {
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        position: CxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, 60)),
                        text: CxText {
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
                    CxTextBundle::<Layer> {
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        position: CxPosition::from(IVec2::new(SCREEN_RESOLUTION_H.x, 50)),
                        text: CxText {
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
            .add_active_systems::<StageUiPlugin, _>((
                render_game_over_screen,
                despawn_game_over_screen,
            ))
            .add_active_systems_in::<StageUiPlugin, _>(
                PostUpdate,
                (check_press_continue_input, handle_game_over_screen_continue)
                    .chain()
                    .run_if(in_state(StageProgressState::GameOver)),
            );
    }
}
