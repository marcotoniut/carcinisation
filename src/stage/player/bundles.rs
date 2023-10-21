use bevy::prelude::*;
use seldom_pixel::prelude::*;

use crate::{
    globals::*,
    stage::components::{interactive::Health, StageEntity},
    Layer,
};

use super::{components::*, crosshair::CrosshairSettings, CrosshairInfo};

pub fn make_player_bundle(
    asset_server: &mut PxAssets<PxSprite>,
    crosshair_settings: &Res<CrosshairSettings>,
) -> (
    Name,
    Player,
    Health,
    PxSpriteBundle<Layer>,
    PxSubPosition,
    StageEntity,
) {
    let crosshair_info = CrosshairInfo::crosshair_sprite(asset_server, crosshair_settings);
    let sprite = CrosshairInfo::get_sprite(crosshair_info);
    (
        Name::new("Player"),
        Player,
        Health(PLAYER_MAX_HEALTH),
        PxSpriteBundle::<Layer> {
            canvas: PxCanvas::Camera,
            sprite,
            layer: Layer::Front,
            anchor: PxAnchor::Center,
            ..Default::default()
        },
        PxSubPosition::from(Vec2::new(
            (SCREEN_RESOLUTION.x / 2) as f32,
            (HUD_HEIGHT as f32) + (SCREEN_RESOLUTION.y / 2) as f32,
        )),
        StageEntity,
    )
}
