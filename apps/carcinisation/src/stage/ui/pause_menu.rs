pub mod components;

use self::components::{InfoText, PauseMenu, ScoreText, UIBackground};
use crate::globals::{SCREEN_RESOLUTION_F32_H, SCREEN_RESOLUTION_H};
use crate::pixel::{PxAssets, PxRectBundle, PxTextBundle};
use crate::{
    game::{GameProgressState, score::components::Score},
    globals::{TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH, mark_for_despawn_by_query},
    layer::Layer,
};
use bevy::prelude::*;
use carcinisation_core::components::GBColor;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxPosition, PxRect, PxSprite, PxSubPosition,
    PxText, PxTypeface,
};

// TODO if state is changed (split unpause from pause?)
pub fn pause_menu_renderer(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    filters: PxAssets<PxFilter>,
    score: Res<Score>,
    query: Query<Entity, With<PauseMenu>>,
    state: Res<State<GameProgressState>>,
) {
    if *state.get() == GameProgressState::Paused {
        if let Ok(_entity) = query.single() {
            //do nothing
        } else {
            spawn_pause_menu_bundle(
                &mut commands,
                &mut typefaces,
                &mut assets_sprite,
                &filters,
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
    _assets_sprite: &mut PxAssets<PxSprite>,
    filters: &PxAssets<PxFilter>,
    score: Res<Score>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
    let score_text = score.value.to_string();
    commands
        .spawn((
            PauseMenu,
            Name::new("PauseMenu"),
            children![
                (
                    PxSubPosition(*SCREEN_RESOLUTION_F32_H),
                    PxRectBundle::<Layer> {
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        filter: PxFilter(filters.load_color(GBColor::White)),
                        layers: PxFilterLayers::single_over(Layer::UIBackground),
                        position: PxPosition::from(*SCREEN_RESOLUTION_H),
                        rect: PxRect(UVec2::new(80, 60)),
                        visibility: Visibility::Visible,
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
                            value: "Paused".to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Pause"),
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
        ))
        .id()
}
