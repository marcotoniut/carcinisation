use bevy::prelude::*;
use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxRect,
        PxSubPosition, PxTextBundle, PxTypeface,
    },
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{globals::*, Layer};

use super::super::{components::*, styles::*};

const LAYOUT_Y: i32 = 2;
const HUD_COUNT_W: i32 = 25;

const SCORE_COUNT_ML: i32 = 15;

pub fn spawn_hud(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
) {
    let main_menu_entity = spawn_hud_bundle(
        &mut commands,
        &mut typefaces,
        &mut assets_sprite,
        &mut filters,
    );
}

pub fn despawn_hud(mut commands: Commands, query: Query<Entity, With<Hud>>) {
    if let Ok(main_menu_entity) = query.get_single() {
        commands.entity(main_menu_entity).despawn_recursive();
    }
}

pub fn spawn_hud_bundle(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let main_menu_entity = commands
        .spawn((Hud {}, Name::new("Hud")))
        .with_children(|parent| {
            for i in 0..(HUD_HEIGHT as i32) {
                parent.spawn((
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [(0, i).into(), (SCREEN_RESOLUTION.x as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::UIBackground),
                        filter: filters.load("filter/color3.png"),
                        ..default()
                    },
                    UIBackground {},
                    Name::new("UIBackground"),
                ));
            }

            parent.spawn((Name::new("Score"),)).with_children(|parent| {
                parent.spawn((
                    PxSpriteBundle::<Layer> {
                        anchor: PxAnchor::BottomLeft,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        sprite: assets_sprite.load("sprites/score-icon.png"),
                        ..default()
                    },
                    PxSubPosition::from(Vec2::new(6.0, LAYOUT_Y as f32)),
                    Name::new("ScoreIcon"),
                ));
                parent.spawn((
                    PxTextBundle::<Layer> {
                        alignment: PxAnchor::BottomRight,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        rect: IRect::new(
                            IVec2::new(SCORE_COUNT_ML, LAYOUT_Y),
                            IVec2::new(
                                SCORE_COUNT_ML + HUD_COUNT_W,
                                LAYOUT_Y + (FONT_SIZE + 2) as i32,
                            ),
                        )
                        .into(),
                        text: "0".into(),
                        typeface: typeface.clone(),
                        ..default()
                    },
                    ScoreText,
                    Name::new("ScoreText"),
                ));
            });

            parent
                .spawn((Name::new("Enemies"),))
                .with_children(|parent| {
                    parent.spawn((
                        PxSpriteBundle::<Layer> {
                            anchor: PxAnchor::BottomRight,
                            canvas: PxCanvas::Camera,
                            layer: Layer::UI,
                            sprite: assets_sprite.load("sprites/enemy-count-icon.png"),
                            ..default()
                        },
                        PxSubPosition::from(Vec2::new(
                            SCREEN_RESOLUTION.x as f32 - 6.0,
                            LAYOUT_Y as f32,
                        )),
                        Name::new("EnemyCountIcon"),
                    ));

                    parent.spawn((
                        PxTextBundle::<Layer> {
                            alignment: PxAnchor::BottomRight,
                            canvas: PxCanvas::Camera,
                            layer: Layer::UI,
                            rect: IRect::new(
                                IVec2::new(
                                    SCREEN_RESOLUTION.x as i32 - SCORE_COUNT_ML - HUD_COUNT_W,
                                    LAYOUT_Y,
                                ),
                                IVec2::new(
                                    SCREEN_RESOLUTION.x as i32 - HUD_COUNT_W,
                                    LAYOUT_Y + (FONT_SIZE + 2) as i32,
                                ),
                            )
                            .into(),
                            text: "0".into(),
                            typeface: typeface.clone(),
                            ..default()
                        },
                        EnemyCountText,
                        Name::new("EnemyCountText"),
                    ));
                });
        })
        .id();

    return main_menu_entity;
}
