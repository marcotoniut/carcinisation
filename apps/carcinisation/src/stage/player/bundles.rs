use super::{
    CrosshairInfo,
    components::{PLAYER_MAX_HEALTH, Player},
    crosshair::CrosshairSettings,
};
use crate::{
    assets::CxAssets,
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION, SCREEN_RESOLUTION_H},
    layer::{Layer, OrsLayer},
    stage::components::{StageEntity, interactive::Health},
};
use bevy::prelude::*;
use carapace::prelude::{CxAnchor, CxRenderSpace, CxSprite, CxSpriteBundle, WorldPos};

pub fn make_player_bundle(
    asset_server: &mut CxAssets<CxSprite>,
    crosshair_settings: &Res<CrosshairSettings>,
) -> (
    Name,
    Player,
    Health,
    CxSpriteBundle<Layer>,
    WorldPos,
    StageEntity,
) {
    let crosshair_info = CrosshairInfo::crosshair_sprite(asset_server, crosshair_settings);
    let sprite = CrosshairInfo::get_sprite(crosshair_info);
    (
        Name::new("Player"),
        Player,
        Health(PLAYER_MAX_HEALTH),
        CxSpriteBundle::<Layer> {
            canvas: CxRenderSpace::Camera,
            sprite: sprite.into(),
            layer: Layer::Ors(OrsLayer::Front),
            anchor: CxAnchor::Center,
            ..default()
        },
        WorldPos::from(Vec2::new(
            SCREEN_RESOLUTION_H.x as f32,
            (HUD_HEIGHT as f32) + (SCREEN_RESOLUTION.y / 2) as f32,
        )),
        StageEntity,
    )
}
