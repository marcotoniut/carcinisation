pub mod bundles;
pub mod components;
pub mod crosshair;
pub mod events;
pub mod resources;
mod systems;

use self::{
    crosshair::{Crosshair, CrosshairSettings},
    events::*,
    resources::AttackTimer,
    systems::{
        camera::{camera_shake, on_camera_shake},
        events::*,
        *,
    },
};
use super::resources::StageTime;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MovementSystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ConfinementSystemSet;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AttackTimer>()
            .init_state::<PlayerPluginUpdateState>()
            .configure_sets(Update, MovementSystemSet.before(ConfinementSystemSet))
            .add_event::<CameraShakeTrigger>()
            .observe(on_camera_shake)
            .add_event::<PlayerStartupTrigger>()
            .observe(on_player_startup)
            .add_event::<PlayerShutdownTrigger>()
            .observe(on_player_shutdown)
            .add_systems(
                Update,
                (
                    tick_attack_timer::<StageTime>,
                    check_attack_timer,
                    detect_player_attack,
                    camera_shake::<StageTime>,
                    player_movement::<StageTime>.in_set(MovementSystemSet),
                    confine_player_movement.in_set(ConfinementSystemSet),
                )
                    .run_if(in_state(PlayerPluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum PlayerPluginUpdateState {
    #[default]
    Inactive,
    Active,
}

pub struct CrosshairInfo {
    pub sprite: Handle<seldom_pixel::asset::PxAsset<seldom_pixel::sprite::PxSpriteData>>,
    pub crosshair: Crosshair,
}

impl CrosshairInfo {
    /** REVIEW if these are needed */
    pub fn get_sprite(
        crosshair: CrosshairInfo,
    ) -> Handle<seldom_pixel::asset::PxAsset<seldom_pixel::sprite::PxSpriteData>> {
        crosshair.sprite
    }

    /** REVIEW if these are needed */
    pub fn get_crosshair(crosshair: CrosshairInfo) -> Crosshair {
        crosshair.crosshair
    }

    pub fn crosshair_sprite(
        asset_server: &mut PxAssets<PxSprite>,
        crosshair_settings: &Res<CrosshairSettings>,
    ) -> CrosshairInfo {
        let sprite;
        let crosshair;

        match crosshair_settings.0 {
            2 => {
                sprite = asset_server.load(assert_assets_path!("sprites/crosshairs/squiggly.png"));
                crosshair = Crosshair {
                    name: "squiggly".to_string(),
                };
            }
            1 => {
                sprite = asset_server.load(assert_assets_path!("sprites/crosshairs/gun_sight.png"));
                crosshair = Crosshair {
                    name: "negative".to_string(),
                };
            }
            0 => {
                sprite = asset_server.load(assert_assets_path!(
                    "sprites/crosshairs/gun_sight_inverted.png"
                ));
                crosshair = Crosshair {
                    name: "default".to_string(),
                };
            }
            _ => {
                sprite = asset_server.load(assert_assets_path!(
                    "sprites/crosshairs/gun_sight_inverted.png"
                ));
                crosshair = Crosshair {
                    name: "default".to_string(),
                };
            }
        }

        return CrosshairInfo {
            sprite: sprite,
            crosshair: crosshair,
        };
    }
}
