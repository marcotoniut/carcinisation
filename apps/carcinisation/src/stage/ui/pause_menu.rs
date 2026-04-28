pub mod components;

use self::components::{InfoText, PauseMenu, ScoreText, UIBackground};
use crate::assets::CxAssets;
use crate::globals::{SCREEN_RESOLUTION_F32_H, SCREEN_RESOLUTION_H};
use crate::{
    game::{GameProgressState, score::components::Score},
    globals::{load_inverted_typeface, mark_for_despawn_by_query},
    layer::{Layer, MenuLayer},
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxPosition, CxRenderSpace, CxSprite, CxText, CxTextBundle, CxTypeface, WorldPos,
};
use carapace::primitive::{CxPrimitive, CxPrimitiveFill, CxPrimitiveShape};

// TODO if state is changed (split unpause from pause?)
pub fn pause_menu_renderer(
    mut commands: Commands,
    mut typefaces: CxAssets<CxTypeface>,
    mut assets_sprite: CxAssets<CxSprite>,
    score: Res<Score>,
    query: Query<Entity, With<PauseMenu>>,
    state: Res<State<GameProgressState>>,
) {
    if *state.get() == GameProgressState::Paused {
        if let Ok(_entity) = query.single() {
            //do nothing
        } else {
            spawn_pause_menu_bundle(&mut commands, &mut typefaces, &mut assets_sprite, score);
        }
    } else {
        mark_for_despawn_by_query(&mut commands, &query);
    }
}

pub fn spawn_pause_menu_bundle(
    commands: &mut Commands,
    typefaces: &mut CxAssets<CxTypeface>,
    _assets_sprite: &mut CxAssets<CxSprite>,
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
                    CxPrimitive {
                        shape: CxPrimitiveShape::Rect {
                            size: UVec2::new(80, 60),
                        },
                        fill: CxPrimitiveFill::Solid(4),
                    },
                    CxAnchor::Center,
                    CxRenderSpace::Camera,
                    CxPosition::from(*SCREEN_RESOLUTION_H),
                    Layer::Menu(MenuLayer::Background),
                    WorldPos(*SCREEN_RESOLUTION_F32_H),
                    UIBackground,
                ),
                (
                    CxTextBundle::<Layer> {
                        position: IVec2::new(SCREEN_RESOLUTION_H.x, 90).into(),
                        anchor: CxAnchor::BottomCenter,
                        canvas: CxRenderSpace::Camera,
                        layer: Layer::Menu(MenuLayer::Foreground),
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
                        layer: Layer::Menu(MenuLayer::Foreground),
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
                        layer: Layer::Menu(MenuLayer::Foreground),
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
