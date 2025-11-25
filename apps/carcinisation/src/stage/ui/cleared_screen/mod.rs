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
use crate::pixel::{PxAssets, PxLineBundle, PxTextBundle};
use crate::{
    components::GBColor,
    game::score::components::Score,
    globals::{
        mark_for_despawn_by_query, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    stage::StageProgressState,
};
use activable::ActiveState;
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxPosition, PxText, PxTypeface,
};

pub fn render_cleared_screen(
    mut commands: Commands,
    assets_typeface: PxAssets<PxTypeface>,
    assets_filter: PxAssets<PxFilter>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::Cleared {
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
        let score_text = score.value.to_string();

        commands
            .spawn((ClearedScreen, Name::new("Screen Cleared")))
            .with_children(|p0| {
                for i in 25..115 {
                    p0.spawn((
                        PxLineBundle::<Layer> {
                            canvas: PxCanvas::Camera,
                            line: [
                                ((SCREEN_RESOLUTION.x / 2) as i32 - HALF_SCREEN_SIZE, i).into(),
                                ((SCREEN_RESOLUTION.x / 2) as i32 + HALF_SCREEN_SIZE, i).into(),
                            ]
                            .into(),
                            layers: PxFilterLayers::single_over(Layer::UIBackground),
                            filter: PxFilter(assets_filter.load_color(GBColor::White)),
                            ..default()
                        },
                        UIBackground {},
                        Name::new("UIBackground"),
                    ));
                }

                let center_x = (SCREEN_RESOLUTION.x / 2) as i32;

                p0.spawn((
                    PxTextBundle::<Layer> {
                        position: PxPosition::from(IVec2::new(center_x, 90)),
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
                ));

                p0.spawn((
                    PxTextBundle::<Layer> {
                        position: PxPosition::from(IVec2::new(center_x, 60)),
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
                ));

                p0.spawn((
                    PxTextBundle::<Layer> {
                        position: PxPosition::from(IVec2::new(center_x, 50)),
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
                ));
            });
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

pub const HALF_SCREEN_SIZE: i32 = 70;

pub fn cleared_screen_plugin(app: &mut App) {
    app.add_message::<ClearScreenShutdownEvent>()
        .add_plugins(InputManagerPlugin::<ClearScreenInput>::default())
        .add_systems(Startup, init_input)
        .add_systems(
            Update,
            (render_cleared_screen, despawn_cleared_screen)
                .run_if(in_state(ActiveState::<StageUiPlugin>::active())),
        )
        .add_systems(
            PostUpdate,
            check_press_continue_input.run_if(in_state(ActiveState::<StageUiPlugin>::active())),
        );
}
