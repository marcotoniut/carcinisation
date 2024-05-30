pub mod components;
pub mod events;
pub mod input;
pub mod systems;

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
        mark_for_despawn_by_component_query, GBColor, PxSpriteColorLoader, FONT_SIZE,
        SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    stage::StageProgressState,
    Layer,
};
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use seldom_pixel::prelude::{
    PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxTextBundle, PxTypeface,
};

pub const HALF_SCREEN_SIZE: i32 = 70;

pub fn render_game_over_screen(
    mut commands: Commands,
    mut assets_typeface: PxAssets<PxTypeface>,
    mut assets_filter: PxAssets<PxFilter>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::GameOver {
        let typeface =
            assets_typeface.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
        let score_text = score.value.to_string();

        commands
            .spawn((GameOverScreen, Name::new("GameOver Screen")))
            .with_children(|parent| {
                for i in 25..(115 as i32) {
                    parent.spawn((
                        PxLineBundle::<Layer> {
                            canvas: PxCanvas::Camera,
                            line: [
                                ((SCREEN_RESOLUTION.x / 2) as i32 - HALF_SCREEN_SIZE, i).into(),
                                ((SCREEN_RESOLUTION.x / 2) as i32 + HALF_SCREEN_SIZE, i).into(),
                            ]
                            .into(),
                            layers: PxFilterLayers::single_over(Layer::UIBackground),
                            filter: assets_filter.load_color(GBColor::White),
                            ..Default::default()
                        },
                        UIBackground {},
                        Name::new("UIBackground"),
                    ));

                    parent.spawn((
                        PxTextBundle::<Layer> {
                            alignment: PxAnchor::BottomCenter,
                            canvas: PxCanvas::Camera,
                            layer: Layer::UI,
                            rect: IRect::new(
                                (SCREEN_RESOLUTION.x / 2) as i32 - HALF_SCREEN_SIZE,
                                90,
                                (SCREEN_RESOLUTION.x / 2) as i32 + HALF_SCREEN_SIZE,
                                90 + (FONT_SIZE + 2) as i32,
                            )
                            .into(),
                            text: "Game  Over".into(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        InfoText,
                        Name::new("InfoText_Stage_GameOver"),
                    ));

                    parent.spawn((
                        PxTextBundle::<Layer> {
                            alignment: PxAnchor::BottomCenter,
                            canvas: PxCanvas::Camera,
                            layer: Layer::UI,
                            rect: IRect::new(
                                (SCREEN_RESOLUTION.x / 2) as i32 - 40,
                                60,
                                (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                                60 + (FONT_SIZE + 2) as i32,
                            )
                            .into(),
                            text: "Score:".into(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        InfoText,
                        Name::new("InfoText_Score"),
                    ));

                    parent.spawn((
                        PxTextBundle::<Layer> {
                            alignment: PxAnchor::BottomCenter,
                            canvas: PxCanvas::Camera,
                            layer: Layer::UI,
                            rect: IRect::new(
                                (SCREEN_RESOLUTION.x / 2) as i32 - 40,
                                50,
                                (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                                50 + (FONT_SIZE + 2) as i32,
                            )
                            .into(),
                            text: score_text.clone().into(),
                            typeface: typeface.clone(),
                            ..Default::default()
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
        mark_for_despawn_by_component_query(&mut commands, &query);
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
