use bevy::prelude::*;
use seldom_pixel::prelude::*;

use crate::{globals::*, Layer};

use super::{components::*, crosshair::CrosshairSettings, CrosshairInfo};

pub fn make_player_bundle(
    asset_server: &mut PxAssets<PxSprite>,
    crosshair_settings: Res<CrosshairSettings>,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Player, Name) {
    let crosshair_info = CrosshairInfo::crosshair_sprite(asset_server, crosshair_settings);
    let sprite = CrosshairInfo::get_sprite(crosshair_info);
    (
        PxSpriteBundle::<Layer> {
            canvas: PxCanvas::Camera,
            sprite,
            layer: Layer::Front,
            anchor: PxAnchor::Center,
            ..default()
        },
        PxSubPosition::from(Vec2::new(
            (SCREEN_RESOLUTION.x / 2) as f32,
            (HUD_HEIGHT as f32) + (SCREEN_RESOLUTION.y / 2) as f32,
        )),
        Player {},
        Name::new("Player"),
    )
}
