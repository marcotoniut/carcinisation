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
use super::StageUiPluginUpdateState;
use crate::pixel::{PxAssets, PxLineBundle, PxTextBundle};
use crate::{
    components::GBColor,
    game::{resources::Lives, score::components::Score},
    globals::{
        mark_for_despawn_by_query, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    stage::StageProgressState,
};
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxPosition, PxText, PxTypeface,
};

pub const HALF_SCREEN_SIZE: i32 = 70;

pub fn render_death_screen(
    mut commands: Commands,
    assets_typeface: PxAssets<PxTypeface>,
    assets_filter: PxAssets<PxFilter>,
    lives: Res<Lives>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::Death {
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
        let lives_text = "Lives ".to_string() + &lives.0.to_string();
        let score_text = score.value.to_string();

        commands
            .spawn((DeathScreen, Name::new("Death Screen")))
            .with_children(|p0| {
                for i in 25..(115 as i32) {
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

                    let center_x = (SCREEN_RESOLUTION.x / 2) as i32;

                    p0.spawn((
                        PxTextBundle::<Layer> {
                            position: PxPosition::from(IVec2::new(center_x, 90)),
                            anchor: PxAnchor::BottomCenter,
                            canvas: PxCanvas::Camera,
                            layer: Layer::UI,
                            text: PxText {
                                value: lives_text.clone(),
                                typeface: typeface.clone(),
                                ..Default::default()
                            },
                            ..default()
                        },
                        InfoText,
                        Name::new("InfoText_Stage_Lives"),
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
                        CurrentScoreText,
                        Name::new("FinalScoreText"),
                    ));
                }
            });
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

pub fn death_screen_plugin(app: &mut App) {
    app.add_message::<DeathScreenRestartEvent>()
        .add_plugins(InputManagerPlugin::<DeathScreenInput>::default())
        .add_systems(Startup, init_input)
        .add_systems(
            Update,
            (render_death_screen, despawn_death_screen)
                .run_if(in_state(StageUiPluginUpdateState::Active)),
        )
        .add_systems(
            PostUpdate,
            check_press_continue_input.run_if(in_state(StageUiPluginUpdateState::Active)),
        );
}
