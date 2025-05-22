pub mod pause_menu;

use self::pause_menu::{InfoText, PauseMenu, ScoreText, UIBackground};
use crate::{
    game::{score::components::Score, GameProgressState},
    globals::{
        mark_for_despawn_by_query, FONT_SIZE, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS,
        TYPEFACE_INVERTED_PATH,
    },
    layer::Layer,
    pixel::components::PxRectangle,
};
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxTypeface},
    sprite::PxSprite,
};

pub fn pause_menu_renderer(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<Entity, With<PauseMenu>>,
    score: Res<Score>,
    state: Res<State<GameProgressState>>,
) {
    if state.get().to_owned() == GameProgressState::Paused {
        if let Ok(entity) = query.get_single() {
            //do nothing
        } else {
            spawn_pause_menu_bundle(&mut commands, &asset_server, score);
        }
    } else {
        mark_for_despawn_by_query(&mut commands, &query);
    }
}

pub fn spawn_pause_menu_bundle(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    score: Res<Score>,
) -> Entity {
    let typeface_asset = asset_server.load(TYPEFACE_INVERTED_PATH);

    let typeface = PxTypeface(typeface_asset, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let score_text = score.value.to_string();
    let entity = commands
        .spawn((PauseMenu, Name::new("PauseMenu")))
        .with_children(|p0| {
            for i in 40..(100 as i32) {
                p0.spawn((
                    // TODO review
                    // canvas: PxCanvas::Camera,
                    PxLine::from([
                        ((SCREEN_RESOLUTION.x / 2) as i32 - 40, i).into(),
                        ((SCREEN_RESOLUTION.x / 2) as i32 + 40 as i32, i).into(),
                    ]),
                    PxFilterLayers::single_over(Layer::UIBackground),
                    // TODO Color
                    PxFilter(asset_server.load("filter/color3.png")),
                    UIBackground,
                    Name::new("UIBackground"),
                ));

                p0.spawn((
                    PxAnchor::BottomCenter,
                    PxCanvas::Camera,
                    Layer::UI,
                    PxRectangle(IRect::new(
                        (SCREEN_RESOLUTION.x / 2) as i32 - 40,
                        90,
                        (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                        90 + (FONT_SIZE + 2) as i32,
                    )),
                    PxText {
                        value: "Paused".to_string(),
                        typeface: typeface.clone(),
                    },
                    InfoText,
                    Name::new("InfoText_Pause"),
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
                    ScoreText,
                    Name::new("ScoreText"),
                ));
            }
        })
        .id();
    return entity;
}
