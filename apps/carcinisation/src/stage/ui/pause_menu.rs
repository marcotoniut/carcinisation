pub mod components;

use self::components::{InfoText, PauseMenu, ScoreText, UIBackground};
use crate::globals::{SCREEN_RESOLUTION_F32_H, SCREEN_RESOLUTION_H};
use crate::pixel::{CxAssets, CxFilterRectBundle, CxTextBundle};
use crate::{
    game::{GameProgressState, score::components::Score},
    globals::{load_inverted_typeface, mark_for_despawn_by_query},
    layer::Layer,
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxFilter, CxFilterLayers, CxFilterRect, CxPosition, CxRenderSpace, CxSprite, CxText,
    CxTypeface, WorldPos,
};
use carcinisation_core::components::GBColor;

// TODO if state is changed (split unpause from pause?)
pub fn pause_menu_renderer(
    mut commands: Commands,
    mut typefaces: CxAssets<CxTypeface>,
    mut assets_sprite: CxAssets<CxSprite>,
    filters: CxAssets<CxFilter>,
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
    typefaces: &mut CxAssets<CxTypeface>,
    _assets_sprite: &mut CxAssets<CxSprite>,
    filters: &CxAssets<CxFilter>,
    score: Res<Score>,
) -> Entity {
    let typeface = load_inverted_typeface(typefaces);
    let score_text = score.value.to_string();
    commands
        .spawn((
            PauseMenu,
            Name::new("PauseMenu"),
            children![
                (
                    WorldPos(*SCREEN_RESOLUTION_F32_H),
                    CxFilterRectBundle::<Layer> {
                        anchor: CxAnchor::Center,
                        canvas: CxRenderSpace::Camera,
                        filter: CxFilter(filters.load_color(GBColor::White)),
                        layers: CxFilterLayers::single_over(Layer::UIBackground),
                        position: CxPosition::from(*SCREEN_RESOLUTION_H),
                        rect: CxFilterRect(UVec2::new(80, 60)),
                        visibility: Visibility::Visible,
                    },
                    UIBackground,
                ),
                (
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 90).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        text: CxText {
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
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 60).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        text: CxText {
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
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 50).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::UI,
                        text: CxText {
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
