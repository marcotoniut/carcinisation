use super::components::{HealthText, Hud, UIBackground};
use crate::pixel::{PxAssets, PxLineBundle, PxSpriteBundle, PxTextBundle};
use crate::{
    globals::*,
    layer::Layer,
    stage::{components::StageEntity, ui::components::ScoreText},
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxPosition, PxSprite, PxSubPosition, PxText,
    PxTypeface,
};

const LAYOUT_Y: i32 = 2;
const HUD_HEALTH_W: i32 = 37;
const HUD_HEALTH_ML: i32 = 15;

const HUD_SCORE_W: i32 = 95;
const HUD_SCORE_MR: i32 = 15;

pub fn spawn_hud(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let entity = commands
        .spawn((Hud, Name::new("Hud"), StageEntity))
        .with_children(|p0| {
            for i in 0..(HUD_HEIGHT as i32) {
                p0.spawn((
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [(0, i).into(), (SCREEN_RESOLUTION.x as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::HudBackground),
                        filter: PxFilter(filters.load("filter/color3.px_filter.png")),
                        ..default()
                    },
                    UIBackground,
                    Name::new("UIBackground"),
                ));
            }

            p0.spawn((Name::new("Health"),)).with_children(|parent| {
                parent.spawn((
                    PxSpriteBundle::<Layer> {
                        anchor: PxAnchor::BottomLeft,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Hud,
                        sprite: PxSprite(assets_sprite.load(assert_assets_path!(
                            "sprites/pickups/health_6.px_sprite.png"
                        ))),
                        ..default()
                    },
                    PxSubPosition::from(Vec2::new(6.0, LAYOUT_Y as f32)),
                    Name::new("HealthIcon"),
                ));
                parent.spawn((
                    PxTextBundle::<Layer> {
                        position: PxPosition::from(IVec2::new(
                            HUD_HEALTH_ML + HUD_HEALTH_W,
                            LAYOUT_Y + (FONT_SIZE + 2) as i32,
                        )),
                        anchor: PxAnchor::BottomRight,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Hud,
                        text: PxText {
                            value: "0".to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
                        ..default()
                    },
                    HealthText,
                    Name::new("HealthText"),
                ));
            });

            p0.spawn((Name::new("Score"),)).with_children(|parent| {
                parent.spawn((
                    PxTextBundle::<Layer> {
                        position: PxPosition::from(IVec2::new(
                            SCREEN_RESOLUTION.x as i32 - HUD_SCORE_MR,
                            LAYOUT_Y + (FONT_SIZE + 2) as i32,
                        )),
                        anchor: PxAnchor::BottomRight,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Hud,
                        text: PxText {
                            value: "0".to_string(),
                            typeface: typeface.clone(),
                            ..Default::default()
                        },
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
