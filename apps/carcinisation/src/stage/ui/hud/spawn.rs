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

pub(super) const LAYOUT_Y: i32 = 2;
pub(super) const HUD_BOTTOM_Y: i32 = SCREEN_RESOLUTION.y as i32 - HUD_HEIGHT as i32;
pub(super) const HUD_HEALTH_ICON_WIDTH: i32 = 9;
pub(super) const HUD_HEALTH_ICON_X: f32 = 6.0;
pub(super) const HUD_HEALTH_TEXT_PADDING: i32 = 4;
pub(super) const HUD_HEALTH_TEXT_CHAR_WIDTH: i32 = 10;
pub(super) const HUD_HEALTH_LAYOUT_Y: i32 = HUD_BOTTOM_Y + LAYOUT_Y;

const HUD_SCORE_MR: i32 = 15;

pub fn spawn_hud(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let entity = commands
        .spawn((
            Hud,
            Name::new("Hud"),
            StageEntity,
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
        ))
        .with_children(|p0| {
            let hud_bottom_y = HUD_BOTTOM_Y;
            let hud_layout_y = HUD_HEALTH_LAYOUT_Y;

            for row in 0..(HUD_HEIGHT as i32) {
                let y = hud_bottom_y + row;
                p0.spawn((
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [(0, y).into(), (SCREEN_RESOLUTION.x as i32, y).into()].into(),
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
                    PxSubPosition::from(Vec2::new(HUD_HEALTH_ICON_X, hud_layout_y as f32)),
                    Name::new("HealthIcon"),
                ));
                parent.spawn((
                    PxTextBundle::<Layer> {
                        position: PxPosition::from(IVec2::new(
                            HUD_HEALTH_ICON_X as i32
                                + HUD_HEALTH_ICON_WIDTH
                                + HUD_HEALTH_TEXT_PADDING,
                            hud_layout_y,
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
                            hud_layout_y,
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

    entity
}
