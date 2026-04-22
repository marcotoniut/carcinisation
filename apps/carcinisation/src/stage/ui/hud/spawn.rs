use super::components::{HealthIcon, HealthText, Hud, UIBackground};
use crate::pixel::{CxAssets, CxFilterRectBundle, CxSpriteBundle, CxTextBundle};
use crate::{
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION, load_inverted_typeface},
    layer::Layer,
    stage::{components::StageEntity, ui::components::ScoreText},
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxFilter, CxFilterLayers, CxFilterRect, CxPosition, CxRenderSpace, CxSprite, CxText,
    CxTypeface, WorldPos,
};
use carcinisation_core::components::GBColor;

pub(super) const LAYOUT_Y: i32 = 3;
pub(super) const HUD_BOTTOM_Y: i32 = SCREEN_RESOLUTION.y as i32 - HUD_HEIGHT as i32;
pub(super) const HUD_HEALTH_ICON_X: f32 = 6.0;
pub(super) const HUD_HEALTH_TEXT_X: i32 = 48;

// pub(super) const HUD_HEALTH_LAYOUT_Y: i32 = HUD_BOTTOM_Y + LAYOUT_Y;

const HUD_SCORE_MR: i32 = 15;

pub fn spawn_hud(
    commands: &mut Commands,
    typefaces: &mut CxAssets<CxTypeface>,
    assets_sprite: &mut CxAssets<CxSprite>,
    filters: &mut CxAssets<CxFilter>,
) -> Entity {
    let typeface = load_inverted_typeface(typefaces);
    commands
        .spawn((
            Hud,
            Name::new("Hud"),
            StageEntity,
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
            children![
                (
                    CxFilterRectBundle::<Layer> {
                        anchor: CxAnchor::BottomLeft,
                        canvas: CxRenderSpace::Camera,
                        filter: CxFilter(filters.load_color(GBColor::White)),
                        layers: CxFilterLayers::single_over(Layer::HudBackground),
                        position: IVec2::ZERO.into(),
                        rect: CxFilterRect(UVec2::new(SCREEN_RESOLUTION.x, HUD_HEIGHT)),
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
                            CxSpriteBundle::<Layer> {
                                anchor: CxAnchor::BottomLeft,
                                canvas: CxRenderSpace::Camera,
                                layer: Layer::Hud,
                                // TODO could add a macro at the level of assets_sprite that extends assert_assets_path!
                                sprite: CxSprite(assets_sprite.load(assert_assets_path!(
                                    "sprites/pickups/health_6.px_sprite.png"
                                ))),
                                ..default()
                            },
                            WorldPos::from(Vec2::new(HUD_HEALTH_ICON_X, LAYOUT_Y as f32)),
                            HealthIcon,
                            Name::new("HealthIcon")
                        ),
                        (
                            CxTextBundle::<Layer> {
                                position: CxPosition::from(
                                    IVec2::new(HUD_HEALTH_TEXT_X, LAYOUT_Y,)
                                ),
                                anchor: CxAnchor::BottomRight,
                                canvas: CxRenderSpace::Camera,
                                layer: Layer::Hud,
                                text: CxText {
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
                        CxTextBundle::<Layer> {
                            position: CxPosition::from(IVec2::new(
                                SCREEN_RESOLUTION.x as i32 - HUD_SCORE_MR,
                                LAYOUT_Y,
                            )),
                            anchor: CxAnchor::BottomRight,
                            canvas: CxRenderSpace::Camera,
                            layer: Layer::Hud,
                            text: CxText {
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
