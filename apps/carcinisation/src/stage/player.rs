//! Player stage systems: movement, attacks, camera shake, and crosshair assets.

pub mod attacks;
pub mod bundles;
pub mod components;
pub mod crosshair;
pub mod messages;
mod systems;

use self::{
    attacks::{AttackDefinitions, AttackInputState, AttackLoadout},
    crosshair::{Crosshair, CrosshairSettings},
    messages::*,
    systems::{
        camera::{camera_shake, on_camera_shake},
        messages::*,
        *,
    },
};
use crate::pixel::{PxAsset, PxAssets, PxSpriteData};
use activable::{Activable, ActivableAppExt};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::PxSprite;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Player movement systems run before confinement to ensure corrected positions.
pub struct MovementSystems;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Ensures players stay within the stage bounds after movement systems run.
pub struct ConfinementSystems;

/// Plugin that schedules player input, attack timers, and camera effects.
#[derive(Activable)]
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AttackDefinitions>()
            .init_resource::<AttackInputState>()
            .init_resource::<AttackLoadout>()
            .configure_sets(Update, MovementSystems.before(ConfinementSystems))
            .add_message::<CameraShakeEvent>()
            .add_observer(on_camera_shake)
            .add_message::<PlayerStartupEvent>()
            .add_observer(on_player_startup)
            .add_message::<PlayerShutdownEvent>()
            .add_observer(on_player_shutdown)
            .add_active_systems::<PlayerPlugin, _>(
                // Player logic only runs when the plugin is active.
                (
                    tick_attack_lifetimes,
                    despawn_expired_attacks,
                    detect_player_attack,
                    camera_shake,
                    player_movement.in_set(MovementSystems),
                    confine_player_movement.in_set(ConfinementSystems),
                ),
            );
    }
}

/// Convenience holder for the chosen crosshair sprite and metadata.
pub struct CrosshairInfo {
    pub sprite: Handle<PxAsset<PxSpriteData>>,
    pub crosshair: Crosshair,
}

impl CrosshairInfo {
    /** REVIEW if these are needed */
    /// Returns the underlying sprite handle (consumes the wrapper).
    pub fn get_sprite(crosshair: CrosshairInfo) -> Handle<PxAsset<PxSpriteData>> {
        crosshair.sprite
    }

    /** REVIEW if these are needed */
    /// Returns the crosshair metadata (consumes the wrapper).
    pub fn get_crosshair(crosshair: CrosshairInfo) -> Crosshair {
        crosshair.crosshair
    }

    /// Loads the sprite matching the configured index and returns the combined info.
    pub fn crosshair_sprite(
        asset_server: &mut PxAssets<PxSprite>,
        crosshair_settings: &Res<CrosshairSettings>,
    ) -> CrosshairInfo {
        let sprite;
        let crosshair;

        match crosshair_settings.0 {
            2 => {
                sprite = asset_server.load(assert_assets_path!(
                    "sprites/crosshairs/squiggly.px_sprite.png"
                ));
                crosshair = Crosshair {
                    name: "squiggly".to_string(),
                };
            }
            1 => {
                sprite = asset_server.load(assert_assets_path!(
                    "sprites/crosshairs/gun_sight.px_sprite.png"
                ));
                crosshair = Crosshair {
                    name: "negative".to_string(),
                };
            }
            0 => {
                sprite = asset_server.load(assert_assets_path!(
                    "sprites/crosshairs/gun_sight_inverted.px_sprite.png"
                ));
                crosshair = Crosshair {
                    name: "default".to_string(),
                };
            }
            _ => {
                sprite = asset_server.load(assert_assets_path!(
                    "sprites/crosshairs/gun_sight_inverted.px_sprite.png"
                ));
                crosshair = Crosshair {
                    name: "default".to_string(),
                };
            }
        }

        CrosshairInfo { sprite, crosshair }
    }
}
