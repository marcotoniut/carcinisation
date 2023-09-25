pub mod components;

use bevy::prelude::*;

use crate::{
    globals::{FONT_SIZE, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH},
    stage::{score::components::Score, StageState},
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

pub fn render_cleared_screen(
    mut commands: Commands,
    mut assets_typeface: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut assets_filter: PxAssets<PxFilter>,
    score: Res<Score>,
    stage_state: Res<State<StageState>>,
) {
    if stage_state.is_changed() && *stage_state.get() == StageState::Cleared {
        spawn_screen(
            &mut commands,
            &mut assets_typeface,
            &mut assets_sprite,
            &mut assets_filter,
            score,
        );
    }
}

pub fn despawn_cleared_screen(
    mut commands: Commands,
    stage_state: Res<State<StageState>>,
    query: Query<Entity, With<ClearedScreen>>,
) {
    if stage_state.is_changed() && *stage_state.get() != StageState::Cleared {
        if let Ok(entity) = query.get_single() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

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
        .spawn((ClearedScreen {}, Name::new("Screen Cleared")))
        .with_children(|parent| {
            for i in 40..(100 as i32) {
                parent.spawn((
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

                parent.spawn((
                    PxTextBundle::<Layer> {
                        alignment: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        rect: IRect::new(
                            IVec2::new((SCREEN_RESOLUTION.x / 2) as i32 - 40, 90),
                            IVec2::new(
                                (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                                90 + (FONT_SIZE + 2) as i32,
                            ),
                        )
                        .into(),
                        text: "Stage Cleared".into(),
                        typeface: typeface.clone(),
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText_Stage_Cleared"),
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
                    ScoreText,
                    Name::new("ScoreText"),
                ));
            }
        })
        .id()
}