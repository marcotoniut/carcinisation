pub mod components;

use bevy::prelude::*;

use crate::{
    globals::{
        mark_for_despawn_by_component_query, FONT_SIZE, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    stage::{score::components::Score, StageProgressState},
    Layer,
};
use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxTextBundle,
        PxTypeface,
    },
    sprite::PxSprite,
};

use self::components::*;

pub fn render_game_over_screen(
    mut commands: Commands,
    mut assets_typeface: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut assets_filter: PxAssets<PxFilter>,
    score: Res<Score>,
    stage_state: Res<State<StageProgressState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageProgressState::GameOver {
        spawn_screen(
            &mut commands,
            &mut assets_typeface,
            &mut assets_sprite,
            &mut assets_filter,
            score,
        );
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

pub const HALF_SCREEN_SIZE: i32 = 70;

pub fn spawn_screen(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
    score: Res<Score>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
    let score_text = score.value.to_string();

    commands
        .spawn((GameOverScreen {}, Name::new("GameOver Screen")))
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
                        filter: filters.load("filter/color3.png"),
                        ..default()
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
                        text: "Game  Over".into(),
                        typeface: typeface.clone(),
                        ..default()
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
                            IVec2::new((SCREEN_RESOLUTION.x / 2) as i32 - 40, 60),
                            IVec2::new(
                                (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                                60 + (FONT_SIZE + 2) as i32,
                            ),
                        )
                        .into(),
                        text: "Score:".into(),
                        typeface: typeface.clone(),
                        ..default()
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
                        ..default()
                    },
                    FinalScoreText,
                    Name::new("FinalScoreText"),
                ));
            }
        })
        .id()
}
