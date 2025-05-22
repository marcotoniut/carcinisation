pub mod components;
pub mod events;
pub mod input;
mod systems;

use self::{
    components::*,
    events::GameOverScreenShutdownEvent,
    input::{init_input, GameOverScreenInput},
    systems::check_press_continue_input,
};
use super::StageUiPluginUpdateState;
use crate::{
    game::score::components::Score,
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

pub fn render_game_over_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::GameOver {
        let typeface = PxTypeface(
            asset_server.load(TYPEFACE_INVERTED_PATH),
            TYPEFACE_CHARACTERS,
            [(' ', 4)],
        );
        let score_text = score.value.to_string();

        commands
            .spawn((GameOverScreen, Name::new("GameOver Screen")))
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
                        UIBackground {},
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
                            value: "Game Over".to_string(),
                            typeface: typeface.clone(),
                        },
                        InfoText,
                        Name::new("InfoText_Stage_GameOver"),
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
                            value: "Score:".to_string(),
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
                        FinalScoreText,
                        Name::new("FinalScoreText"),
                    ));
                }
            });
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

pub fn game_over_screen_plugin(app: &mut App) {
    app.add_event::<GameOverScreenShutdownEvent>()
        .add_plugins(InputManagerPlugin::<GameOverScreenInput>::default())
        .add_systems(Startup, init_input)
        .add_systems(
            PostUpdate,
            check_press_continue_input.run_if(in_state(StageUiPluginUpdateState::Active)),
        );
}
