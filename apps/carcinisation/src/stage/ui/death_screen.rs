pub mod components;
pub mod input;
pub mod messages;
mod systems;

use self::{
    components::{CurrentScoreText, DeathScreen, InfoText, UIBackground},
    input::{DeathScreenInput, init_input},
    messages::DeathScreenRestartMessage,
    systems::{check_press_continue_input, handle_death_screen_continue},
};
use super::StageUiPlugin;
use crate::{
    components::GBColor,
    game::{resources::Lives, score::components::Score},
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

pub fn render_death_screen(
    mut commands: Commands,
    assets_typeface: CxAssets<CxTypeface>,
    filters: CxAssets<CxFilter>,
    lives: Res<Lives>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::Death {
        let typeface = load_inverted_typeface(&assets_typeface);

        commands.spawn((
            DeathScreen,
            Name::new("Death Screen"),
            Visibility::Visible,
            children![
                (
                    WorldPos(*SCREEN_RESOLUTION_F32_H),
                    CxFilterRectBundle::<Layer> {
                        rect: CxFilterRect(UVec2::new(120, 90)),
                        position: CxPosition::from(*SCREEN_RESOLUTION_H),
                        anchor: CxAnchor::Center,
                        canvas: CxRenderSpace::Camera,
                        layers: CxFilterLayers::single_over(Layer::UIBackground),
                        filter: CxFilter(filters.load_color(GBColor::White)),
                        visibility: Visibility::Visible,
                    },
                    UIBackground,
                ),
                (
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 90).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        text: CxText {
                            // TODO use template
                            value: "Lives ".to_string() + &lives.0.to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Stage_Lives"),
                ),
                (
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 60).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        text: CxText {
                            value: "Score:".into(),
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
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 50).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        text: CxText {
                            value: score.value.to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    CurrentScoreText,
                    Name::new("FinalScoreText"),
                )
            ],
        ));
    }
}

pub fn despawn_death_screen(
    mut commands: Commands,
    stage_state: Res<State<StageProgressState>>,
    query: Query<Entity, With<DeathScreen>>,
) {
    if stage_state.is_changed() && *stage_state.get() != StageProgressState::Death {
        mark_for_despawn_by_query(&mut commands, &query);
    }
}

#[derive(Activable)]
pub struct DeathScreenPlugin;

impl Plugin for DeathScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DeathScreenRestartMessage>()
            .add_plugins(InputManagerPlugin::<DeathScreenInput>::default())
            .add_systems(Startup, init_input)
            .add_active_systems::<StageUiPlugin, _>((render_death_screen, despawn_death_screen))
            .add_active_systems_in::<StageUiPlugin, _>(
                PostUpdate,
                (check_press_continue_input, handle_death_screen_continue)
                    .chain()
                    .run_if(in_state(StageProgressState::Death)),
            );
    }
}
