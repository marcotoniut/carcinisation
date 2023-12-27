pub mod components;
pub mod events;
pub mod input;
pub mod systems;

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
        mark_for_despawn_by_component_query, GBColor, PxSpriteColorLoader, FONT_SIZE,
        SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    stage::StageProgressState,
    Layer,
};
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use seldom_pixel::prelude::{
    IRect, PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxTextBundle,
    PxTypeface,
};

pub const HALF_SCREEN_SIZE: i32 = 70;

pub fn render_death_screen(
    mut commands: Commands,
    mut assets_typeface: PxAssets<PxTypeface>,
    mut assets_filter: PxAssets<PxFilter>,
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
                                IVec2::new((SCREEN_RESOLUTION.x / 2) as i32 - HALF_SCREEN_SIZE, 90),
                                IVec2::new(
                                    (SCREEN_RESOLUTION.x / 2) as i32 + HALF_SCREEN_SIZE,
                                    90 + (FONT_SIZE + 2) as i32,
                                ),
                            )
                            .into(),
                            text: lives_text.clone().into(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        InfoText,
                        Name::new("InfoText_Stage_Lives"),
                    ));

                    parent.spawn((
                        PxTextBundle::<Layer> {
                            alignment: PxAnchor::BottomCenter,
                            canvas: PxCanvas::Camera,
                            layer: Layer::UI,
                            rect: IRect::new(
                                IVec2::new((SCREEN_RESOLUTION.x / 2) as i32 - 40, 60),
                                IVec2::new(
                                    (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                                    60 + (FONT_SIZE + 2) as i32,
                                ),
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
                                IVec2::new((SCREEN_RESOLUTION.x / 2) as i32 - 40, 50),
                                IVec2::new(
                                    (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                                    50 + (FONT_SIZE + 2) as i32,
                                ),
                            )
                            .into(),
                            text: score_text.clone().into(),
                            typeface: typeface.clone(),
                            ..Default::default()
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
        mark_for_despawn_by_component_query(&mut commands, &query);
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
