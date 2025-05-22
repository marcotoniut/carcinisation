use super::components::{HealthText, Hud, UIBackground};
use crate::{
    globals::*,
    layer::Layer,
    pixel::components::PxRectangle,
    stage::{components::StageEntity, ui::components::ScoreText},
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxSprite, PxSubPosition, PxTypeface,
};

const LAYOUT_Y: i32 = 2;
const HUD_HEALTH_W: i32 = 37;
const HUD_HEALTH_ML: i32 = 15;

const HUD_SCORE_W: i32 = 95;
const HUD_SCORE_MR: i32 = 15;

pub fn spawn_hud(commands: &mut Commands, asset_server: &Res<AssetServer>) -> Entity {
    let typeface_asset = asset_server.load(TYPEFACE_INVERTED_PATH);
    let typeface = PxTypeface(typeface_asset, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let entity = commands
        .spawn((Hud, Name::new("Hud"), StageEntity))
        .with_children(|p0| {
            for i in 0..(HUD_HEIGHT as i32) {
                p0.spawn((
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [(0, i).into(), (SCREEN_RESOLUTION.x as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::HudBackground),
                        filter: filters.load("filter/color3.png"),
                        ..default()
                    },
                    UIBackground,
                    Name::new("UIBackground"),
                ));
            }

            p0.spawn((Name::new("Health"),)).with_children(|parent| {
                parent.spawn((
                    PxAnchor::BottomLeft,
                    PxCanvas::Camera,
                    Layer::Hud,
                    PxSprite(
                        assets_sprite.load(assert_assets_path!("sprites/pickups/health_6.png")),
                    ),
                    PxSubPosition::from(Vec2::new(6.0, LAYOUT_Y as f32)),
                    Name::new("HealthIcon"),
                ));
                parent.spawn((
                    PxAnchor::BottomRight,
                    PxCanvas::Camera,
                    Layer::UI,
                    PxRectangle(IRect::new(
                        HUD_HEALTH_ML,
                        LAYOUT_Y,
                        HUD_HEALTH_ML + HUD_HEALTH_W,
                        LAYOUT_Y + (FONT_SIZE + 2) as i32,
                    )),
                    PxText {
                        value: "0".to_string(),
                        typeface: typeface.clone(),
                    },
                    HealthText,
                    Name::new("HealthText"),
                ));
            });

            p0.spawn((Name::new("Score"),)).with_children(|parent| {
                parent.spawn((
                    PxAnchor::BottomRight,
                    PxCanvas::Camera,
                    Layer::UI,
                    PxRectangle(IRect::new(
                        SCREEN_RESOLUTION.x as i32 - HUD_SCORE_MR - HUD_SCORE_W,
                        LAYOUT_Y,
                        SCREEN_RESOLUTION.x as i32 - HUD_SCORE_MR,
                        LAYOUT_Y + (FONT_SIZE + 2) as i32,
                    )),
                    PxText {
                        value: "0".to_string(),
                        typeface: typeface.clone(),
                    },
                    ScoreText,
                    Name::new("ScoreText"),
                ));
            });
        })
        .id();

    return entity;
}
