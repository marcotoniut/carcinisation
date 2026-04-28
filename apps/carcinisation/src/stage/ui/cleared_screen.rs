pub mod components;
pub mod input;
pub mod messages;
mod systems;

use self::{
    components::{ClearedScreen, InfoText, UIBackground},
    input::{ClearScreenInput, init_input},
    messages::ClearScreenShutdownMessage,
    systems::check_press_continue_input,
};
use super::{StageUiPlugin, components::ScoreText};
use crate::{assets::CxAssets, globals::SCREEN_RESOLUTION_H};
use crate::{
    game::score::components::Score,
    globals::{SCREEN_RESOLUTION_F32_H, load_inverted_typeface, mark_for_despawn_by_query},
    layer::{Layer, MenuLayer},
    stage::StageProgressState,
};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxPosition, CxRenderSpace, CxText, CxTextBundle, CxTypeface, WorldPos,
};
use carapace::primitive::{CxPrimitive, CxPrimitiveFill, CxPrimitiveShape};
use leafwing_input_manager::plugin::InputManagerPlugin;

pub fn render_cleared_screen(
    mut commands: Commands,
    assets_typeface: CxAssets<CxTypeface>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::Cleared {
        let typeface = load_inverted_typeface(&assets_typeface);
        let score_text = score.value.to_string();

        commands.spawn((
            ClearedScreen,
            Name::new("Screen Cleared"),
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
                    Layer::Menu(MenuLayer::Background),
                    WorldPos(*SCREEN_RESOLUTION_F32_H),
                    UIBackground,
                ),
                (
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 90).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::Menu(MenuLayer::Foreground),
                        text: CxText {
                            value: "Stage  Cleared".to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Stage_Cleared"),
                ),
                (
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 60).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::Menu(MenuLayer::Foreground),
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
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 50).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::Menu(MenuLayer::Foreground),
                        text: CxText {
                            value: score_text.clone(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    ScoreText,
                    Name::new("ScoreText"),
                )
            ],
        ));
    }
}

pub fn despawn_cleared_screen(
    mut commands: Commands,
    stage_state: Res<State<StageProgressState>>,
    query: Query<Entity, With<ClearedScreen>>,
) {
    if stage_state.is_changed() && *stage_state.get() != StageProgressState::Cleared {
        mark_for_despawn_by_query(&mut commands, &query);
    }
}

#[derive(Activable)]
pub struct ClearedScreenPlugin;

impl Plugin for ClearedScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ClearScreenShutdownMessage>()
            .add_plugins(InputManagerPlugin::<ClearScreenInput>::default())
            .add_systems(Startup, init_input)
            .add_active_systems::<StageUiPlugin, _>((render_cleared_screen, despawn_cleared_screen))
            .add_active_systems_in::<StageUiPlugin, _>(
                PostUpdate,
                check_press_continue_input.run_if(in_state(StageProgressState::Cleared)),
            );
    }
}
