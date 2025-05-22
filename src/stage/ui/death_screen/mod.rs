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
use crate::{
    game::{resources::Lives, score::components::Score},
    globals::{
        mark_for_despawn_by_query, FONT_SIZE, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    pixel::components::PxRectangle,
    stage::StageProgressState,
};
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use seldom_pixel::prelude::{PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxTypeface};

pub const HALF_SCREEN_SIZE: i32 = 70;

pub fn render_death_screen(
    mut commands: Commands,
    asset_server: &Res<AssetServer>,
    lives: Res<Lives>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::Death {
        let typeface = PxTypeface(
            asset_server.load(TYPEFACE_INVERTED_PATH),
            TYPEFACE_CHARACTERS,
            [(' ', 4)],
        );
        let lives_text = "Lives ".to_string() + &lives.0.to_string();
        let score_text = score.value.to_string();

        commands
            .spawn((DeathScreen, Name::new("Death Screen")))
            .with_children(|p0| {
                for i in 25..(115 as i32) {
                    p0.spawn((
                        // TODO review
                        // canvas: PxCanvas::Camera,
                        PxLine::from([
                            ((SCREEN_RESOLUTION.x / 2) as i32 - HALF_SCREEN_SIZE, i).into(),
                            ((SCREEN_RESOLUTION.x / 2) as i32 + HALF_SCREEN_SIZE, i).into(),
                        ]),
                        PxFilterLayers::single_over(Layer::UIBackground),
                        PxFilter(asset_server.load(GBColor::White.get_filter_path())),
                        UIBackground,
                        Name::new("UIBackground"),
                    ));

                    p0.spawn((
                        PxAnchor::BottomCenter,
                        PxCanvas::Camera,
                        Layer::UI,
                        PxRectangle(IRect::new(
                            (SCREEN_RESOLUTION.x / 2) as i32 - HALF_SCREEN_SIZE,
                            90,
                            (SCREEN_RESOLUTION.x / 2) as i32 + HALF_SCREEN_SIZE,
                            90 + (FONT_SIZE + 2) as i32,
                        )),
                        PxText {
                            value: lives_text.to_string(),
                            typeface: typeface.clone(),
                        },
                        InfoText,
                        Name::new("InfoText_Stage_Lives"),
                    ));

                    p0.spawn((
                        PxAnchor::BottomCenter,
                        PxCanvas::Camera,
                        Layer::UI,
                        PxRectangle(IRect::new(
                            (SCREEN_RESOLUTION.x / 2) as i32 - 40,
                            60,
                            (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                            60 + (FONT_SIZE + 2) as i32,
                        )),
                        PxText {
                            value: score_text.to_string(),
                            typeface: typeface.clone(),
                        },
                        InfoText,
                        Name::new("InfoText_Score"),
                    ));

                    p0.spawn((
                        PxAnchor::BottomCenter,
                        PxCanvas::Camera,
                        Layer::UI,
                        PxRectangle(IRect::new(
                            (SCREEN_RESOLUTION.x / 2) as i32 - 40,
                            50,
                            (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                            50 + (FONT_SIZE + 2) as i32,
                        )),
                        PxText {
                            value: score_text.to_string(),
                            typeface: typeface.clone(),
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
    app.add_event::<DeathScreenRestartEvent>()
        .add_plugins(InputManagerPlugin::<DeathScreenInput>::default())
        .add_systems(Startup, init_input)
        .add_systems(
            PostUpdate,
            check_press_continue_input.run_if(in_state(StageUiPluginUpdateState::Active)),
        );
}
