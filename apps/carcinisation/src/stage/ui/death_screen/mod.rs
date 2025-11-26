pub mod components;
pub mod events;
pub mod input;
mod systems;

use self::{
    components::*,
    events::DeathScreenRestartEvent,
    input::{init_input, DeathScreenInput},
    systems::check_press_continue_input,
};
use super::StageUiPlugin;
use crate::{
    components::GBColor,
    game::{resources::Lives, score::components::Score},
    globals::{
        mark_for_despawn_by_query, SCREEN_RESOLUTION_F32_H, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    pixel::components::PxRectangle,
    stage::StageProgressState,
};
use crate::{
    globals::SCREEN_RESOLUTION_H,
    pixel::{PxAssets, PxTextBundle},
};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use seldom_pixel::prelude::{PxAnchor, PxCanvas, PxSubPosition, PxText, PxTypeface};

pub fn render_death_screen(
    mut commands: Commands,
    assets_typeface: PxAssets<PxTypeface>,
    lives: Res<Lives>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::Death {
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

        commands.spawn((
            DeathScreen,
            Name::new("Death Screen"),
            children![
                (
                    PxSubPosition(*SCREEN_RESOLUTION_F32_H),
                    PxRectangle {
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        color: GBColor::White,
                        height: 90,
                        layer: Layer::UIBackground,
                        width: 120,
                    },
                    UIBackground,
                ),
                (
                    PxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 90).into(),
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        text: PxText {
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
                    PxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 60).into(),
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        text: PxText {
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
                    PxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 50).into(),
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        text: PxText {
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
    if stage_state.is_changed() && *stage_state.get() != StageProgressState::GameOver {
        mark_for_despawn_by_query(&mut commands, &query);
    }
}

#[derive(Activable)]
pub struct DeathScreenPlugin;

impl Plugin for DeathScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DeathScreenRestartEvent>()
            .add_plugins(InputManagerPlugin::<DeathScreenInput>::default())
            .add_systems(Startup, init_input)
            .add_active_systems::<StageUiPlugin, _>((render_death_screen, despawn_death_screen))
            .add_active_systems_in::<StageUiPlugin, _>(PostUpdate, check_press_continue_input);
    }
}
