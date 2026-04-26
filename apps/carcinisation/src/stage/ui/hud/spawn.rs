use super::components::{HealthIcon, HealthText, Hud, UIBackground};
use crate::pixel::{CxAssets, CxTextBundle};
use crate::{
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION, load_inverted_typeface},
    layer::Layer,
    stage::{
        components::StageEntity, data::PickupType, pickup::visual::load_pickup_visual,
        ui::components::ScoreText,
    },
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxPosition, CxPresentationTransform, CxRenderSpace, CxText, CxTypeface, WorldPos,
};
use carapace::primitive::{CxPrimitive, CxPrimitiveFill, CxPrimitiveShape};

pub(super) const LAYOUT_Y: i32 = 3;
pub(super) const HUD_BOTTOM_Y: i32 = SCREEN_RESOLUTION.y as i32 - HUD_HEIGHT as i32;
pub(super) const HUD_HEALTH_ICON_X: f32 = 8.0;
pub(super) const HUD_HEALTH_TEXT_X: i32 = 48;

// pub(super) const HUD_HEALTH_LAYOUT_Y: i32 = HUD_BOTTOM_Y + LAYOUT_Y;

const HUD_SCORE_MR: i32 = 15;

pub fn spawn_hud(
    commands: &mut Commands,
    typefaces: &mut CxAssets<CxTypeface>,
    asset_server: &AssetServer,
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
                    CxPrimitive {
                        shape: CxPrimitiveShape::Rect {
                            size: UVec2::new(SCREEN_RESOLUTION.x, HUD_HEIGHT),
                        },
                        fill: CxPrimitiveFill::Solid(4),
                    },
                    CxAnchor::BottomLeft,
                    CxRenderSpace::Camera,
                    CxPosition::from(IVec2::ZERO),
                    Layer::HudBackground,
                    // TODO Technically not a UiBackground
                    UIBackground,
                    Name::new("HudBackground"),
                ),
                (
                    Name::new("Health"),
                    Visibility::Visible,
                    InheritedVisibility::VISIBLE,
                    children![
                        {
                            let mut visual = load_pickup_visual(
                                asset_server,
                                PickupType::BigHealth.visible_parts(),
                            );
                            visual.render_space = Some(CxRenderSpace::Camera);
                            (
                                visual,
                                WorldPos::from(Vec2::new(
                                    HUD_HEALTH_ICON_X,
                                    HUD_HEIGHT as f32 / 2.0,
                                )),
                                CxPresentationTransform::scaled(0.5),
                                Layer::Hud,
                                HealthIcon,
                                Name::new("HealthIcon"),
                            )
                        },
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
