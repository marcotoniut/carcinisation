pub mod pause_menu;

use self::pause_menu::{InfoText, PauseMenu, ScoreText, UIBackground};
use crate::pixel::{PxAssets, PxLineBundle, PxTextBundle};
use crate::{
    game::{score::components::Score, GameProgressState},
    globals::{
        mark_for_despawn_by_query, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxPosition, PxSprite, PxText, PxTypeface,
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
        if let Ok(entity) = query.single() {
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
                        filter: PxFilter(filters.load("filter/color3.px_filter.png")),
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
                            value: "Paused".to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Pause"),
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
            }
        })
        .id();
    return entity;
}
