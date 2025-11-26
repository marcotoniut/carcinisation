pub mod components;
pub mod events;
pub mod input;
mod systems;

use self::{
    components::*,
    events::ClearScreenShutdownEvent,
    input::{init_input, ClearScreenInput},
    systems::check_press_continue_input,
};
use super::{components::ScoreText, StageUiPlugin};
use crate::{
    components::GBColor,
    game::score::components::Score,
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

pub fn render_cleared_screen(
    mut commands: Commands,
    assets_typeface: PxAssets<PxTypeface>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::Cleared {
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
        let score_text = score.value.to_string();

        commands.spawn((
            ClearedScreen,
            Name::new("Screen Cleared"),
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
                    PxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 60).into(),
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
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
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 50).into(),
                        anchor: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        text: PxText {
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
        app.add_message::<ClearScreenShutdownEvent>()
            .add_plugins(InputManagerPlugin::<ClearScreenInput>::default())
            .add_systems(Startup, init_input)
            .add_active_systems::<StageUiPlugin, _>((render_cleared_screen, despawn_cleared_screen))
            .add_active_systems_in::<StageUiPlugin, _>(PostUpdate, check_press_continue_input);
    }
}
