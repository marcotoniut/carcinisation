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
    game::score::components::Score,
    globals::{SCREEN_RESOLUTION_F32_H, load_inverted_typeface, mark_for_despawn_by_query},
    layer::Layer,
    stage::StageProgressState,
};
use crate::{
    globals::SCREEN_RESOLUTION_H,
    pixel::{CxAssets, CxTextBundle},
};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use carapace::prelude::{CxAnchor, CxPosition, CxRenderSpace, CxText, CxTypeface, WorldPos};
use carapace::primitive::{CxPrimitive, CxPrimitiveFill, CxPrimitiveShape};
use leafwing_input_manager::plugin::InputManagerPlugin;

pub fn render_game_over_screen(
    mut commands: Commands,
    assets_typeface: CxAssets<CxTypeface>,
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
                    CxPrimitive {
                        shape: CxPrimitiveShape::Rect {
                            size: UVec2::new(120, 90),
                        },
                        fill: CxPrimitiveFill::Solid(4),
                    },
                    CxAnchor::Center,
                    CxRenderSpace::Camera,
                    CxPosition::from(*SCREEN_RESOLUTION_H),
                    Layer::UIBackground,
                    WorldPos(*SCREEN_RESOLUTION_F32_H),
                    Name::new("UIBackground"),
                    UIBackground,
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
