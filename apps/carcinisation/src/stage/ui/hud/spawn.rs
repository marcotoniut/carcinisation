use super::components::{HealthIcon, HealthText, Hud, UIBackground};
use crate::pixel::{PxAssets, PxRectBundle, PxSpriteBundle, PxTextBundle};
use crate::{
    globals::*,
    layer::Layer,
    stage::{components::StageEntity, ui::components::ScoreText},
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carcinisation_core::components::GBColor;
use seldom_pixel::prelude::{
    PxAnchor, PxCanvas, PxFilter, PxFilterLayers, PxPosition, PxRect, PxSprite, PxSubPosition,
    PxText, PxTypeface,
};

pub(super) const LAYOUT_Y: i32 = 3;
pub(super) const HUD_BOTTOM_Y: i32 = SCREEN_RESOLUTION.y as i32 - HUD_HEIGHT as i32;
pub(super) const HUD_HEALTH_ICON_X: f32 = 6.0;
pub(super) const HUD_HEALTH_TEXT_X: i32 = 48;

// pub(super) const HUD_HEALTH_LAYOUT_Y: i32 = HUD_BOTTOM_Y + LAYOUT_Y;

const HUD_SCORE_MR: i32 = 15;

pub fn spawn_hud(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);
    commands
        .spawn((
            Hud,
            Name::new("Hud"),
            StageEntity,
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
            children![
                (
                    PxRectBundle::<Layer> {
                        anchor: PxAnchor::BottomLeft,
                        canvas: PxCanvas::Camera,
                        filter: PxFilter(filters.load_color(GBColor::White)),
                        layers: PxFilterLayers::single_over(Layer::HudBackground),
                        position: IVec2::ZERO.into(),
                        rect: PxRect(UVec2::new(SCREEN_RESOLUTION.x, HUD_HEIGHT)),
                        visibility: Visibility::Visible,
                    },
                    // TODO Technically not a UiBackground
                    UIBackground,
                    Name::new("HudBackground"),
                ),
                (
                    Name::new("Health"),
                    Visibility::Visible,
                    InheritedVisibility::VISIBLE,
                    children![
                        (
                            PxSpriteBundle::<Layer> {
                                anchor: PxAnchor::BottomLeft,
                                canvas: PxCanvas::Camera,
                                layer: Layer::Hud,
                                // TODO could add a macro at the level of assets_sprite that extends assert_assets_path!
                                sprite: PxSprite(assets_sprite.load(assert_assets_path!(
                                    "sprites/pickups/health_6.px_sprite.png"
                                ))),
                                ..default()
                            },
                            PxSubPosition::from(Vec2::new(HUD_HEALTH_ICON_X, LAYOUT_Y as f32)),
                            HealthIcon,
                            Name::new("HealthIcon")
                        ),
                        (
                            PxTextBundle::<Layer> {
                                position: PxPosition::from(
                                    IVec2::new(HUD_HEALTH_TEXT_X, LAYOUT_Y,)
                                ),
                                anchor: PxAnchor::BottomRight,
                                canvas: PxCanvas::Camera,
                                layer: Layer::Hud,
                                text: PxText {
                                    typeface: typeface.clone(),
                                    ..Default::default()
                                },
                                ..default()
                            },
                            HealthText,
                            Name::new("HealthText")
                        ),
                    ],
                ),
                (
                    Name::new("Score"),
                    Visibility::Visible,
                    InheritedVisibility::VISIBLE,
                    children![(
                        PxTextBundle::<Layer> {
                            position: PxPosition::from(IVec2::new(
                                SCREEN_RESOLUTION.x as i32 - HUD_SCORE_MR,
                                LAYOUT_Y,
                            )),
                            anchor: PxAnchor::BottomRight,
                            canvas: PxCanvas::Camera,
                            layer: Layer::Hud,
                            text: PxText {
                                typeface: typeface.clone(),
                                ..Default::default()
                            },
                            ..default()
                        },
                        ScoreText,
                        Name::new("ScoreText")
                    ),],
                )
            ],
        ))
        .id()
}
