use super::super::components::*;
use crate::{globals::*, stage::ui::components::ScoreText, Layer};
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxSubPosition,
        PxTextBundle, PxTypeface,
    },
    sprite::{PxSprite, PxSpriteBundle},
};

const LAYOUT_Y: i32 = 2;
const HUD_HEALTH_W: i32 = 37;
const HUD_HEALTH_ML: i32 = 15;

const HUD_SCORE_W: i32 = 95;
const HUD_SCORE_MR: i32 = 15;

pub fn spawn_hud(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
) {
    spawn_hud_bundle(
        &mut commands,
        &mut typefaces,
        &mut assets_sprite,
        &mut filters,
    );
}

pub fn spawn_hud_bundle(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let entity = commands
        .spawn((Hud {}, Name::new("Hud")))
        .with_children(|parent| {
            for i in 0..(HUD_HEIGHT as i32) {
                parent.spawn((
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [(0, i).into(), (SCREEN_RESOLUTION.x as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::HudBackground),
                        filter: filters.load("filter/color3.png"),
                        ..default()
                    },
                    UIBackground {},
                    Name::new("UIBackground"),
                ));
            }

            parent
                .spawn((Name::new("Health"),))
                .with_children(|parent| {
                    parent.spawn((
                        PxSpriteBundle::<Layer> {
                            anchor: PxAnchor::BottomLeft,
                            canvas: PxCanvas::Camera,
                            layer: Layer::Hud,
                            sprite: assets_sprite.load("sprites/score-icon.png"),
                            ..default()
                        },
                        PxSubPosition::from(Vec2::new(6.0, LAYOUT_Y as f32)),
                        Name::new("HealthIcon"),
                    ));
                    parent.spawn((
                        PxTextBundle::<Layer> {
                            alignment: PxAnchor::BottomRight,
                            canvas: PxCanvas::Camera,
                            layer: Layer::Hud,
                            rect: IRect::new(
                                IVec2::new(HUD_HEALTH_ML, LAYOUT_Y),
                                IVec2::new(
                                    HUD_HEALTH_ML + HUD_HEALTH_W,
                                    LAYOUT_Y + (FONT_SIZE + 2) as i32,
                                ),
                            )
                            .into(),
                            text: "0".into(),
                            typeface: typeface.clone(),
                            ..default()
                        },
                        HealthText,
                        Name::new("HealthText"),
                    ));
                });

            parent.spawn((Name::new("Score"),)).with_children(|parent| {
                parent.spawn((
                    PxTextBundle::<Layer> {
                        alignment: PxAnchor::BottomRight,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Hud,
                        rect: IRect::new(
                            IVec2::new(
                                SCREEN_RESOLUTION.x as i32 - HUD_SCORE_MR - HUD_SCORE_W,
                                LAYOUT_Y,
                            ),
                            IVec2::new(
                                SCREEN_RESOLUTION.x as i32 - HUD_SCORE_MR,
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
        })
        .id();

    return entity;
}
