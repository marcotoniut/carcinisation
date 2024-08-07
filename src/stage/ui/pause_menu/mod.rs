pub mod pause_menu;

use self::pause_menu::{InfoText, PauseMenu, ScoreText, UIBackground};
use crate::{
    game::{score::components::Score, GameProgressState},
    globals::{
        mark_for_despawn_by_query, FONT_SIZE, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
};
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{
        PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxTextBundle,
        PxTypeface,
    },
    sprite::PxSprite,
};

pub fn pause_menu_renderer(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
    score: Res<Score>,
    query: Query<Entity, With<PauseMenu>>,
    state: Res<State<GameProgressState>>,
) {
    if state.get().to_owned() == GameProgressState::Paused {
        if let Ok(entity) = query.get_single() {
            //do nothing
        } else {
            spawn_pause_menu_bundle(
                &mut commands,
                &mut typefaces,
                &mut assets_sprite,
                &mut filters,
                score,
            );
        }
    } else {
        mark_for_despawn_by_query(&mut commands, &query);
    }
}

pub fn spawn_pause_menu_bundle(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
    score: Res<Score>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
    let score_text = score.value.to_string();
    let entity = commands
        .spawn((PauseMenu {}, Name::new("PauseMenu")))
        .with_children(|p0| {
            for i in 40..(100 as i32) {
                p0.spawn((
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [
                            ((SCREEN_RESOLUTION.x / 2) as i32 - 40, i).into(),
                            ((SCREEN_RESOLUTION.x / 2) as i32 + 40 as i32, i).into(),
                        ]
                        .into(),
                        layers: PxFilterLayers::single_over(Layer::UIBackground),
                        filter: filters.load("filter/color3.png"),
                        ..default()
                    },
                    UIBackground {},
                    Name::new("UIBackground"),
                ));

                p0.spawn((
                    PxTextBundle::<Layer> {
                        alignment: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        rect: IRect::new(
                            (SCREEN_RESOLUTION.x / 2) as i32 - 40,
                            90,
                            (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                            90 + (FONT_SIZE + 2) as i32,
                        )
                        .into(),
                        text: "Paused".into(),
                        typeface: typeface.clone(),
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Pause"),
                ));

                p0.spawn((
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
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Score"),
                ));

                p0.spawn((
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
                        ..default()
                    },
                    ScoreText,
                    Name::new("ScoreText"),
                ));
            }
        })
        .id();
    return entity;
}
