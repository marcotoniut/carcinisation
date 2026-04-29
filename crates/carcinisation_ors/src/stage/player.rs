//! Player stage systems: movement, attacks, camera shake, and crosshair assets.

pub mod attacks;
pub mod bundles;
pub mod components;
pub mod config;
pub mod crosshair;
pub mod flamethrower;
pub mod intent;
pub mod messages;
pub(crate) mod systems;

use self::{
    attacks::{AttackDefinitions, AttackInputState, AttackLoadout},
    config::PlayerConfig,
    crosshair::{Crosshair, CrosshairSettings},
    flamethrower::{
        FlamethrowerConfig, flamethrower_damage, manage_flamethrower, update_flamethrower,
    },
    intent::{PlayerIntent, SelectChordState, resolve_player_intent},
    messages::{CameraShakeEvent, PlayerShutdownEvent, PlayerStartupEvent},
    systems::{
        camera::on_camera_shake,
        confine_player_movement, despawn_expired_attacks, detect_player_attack,
        messages::{on_player_shutdown, on_player_startup},
        player_movement, tick_attack_lifetimes, tick_webbed_status,
    },
};
use crate::assets::{CxAsset, CxAssets, CxSpriteData};
use activable::{Activable, ActivableAppExt};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::CxSprite;

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
        app.insert_resource(PlayerConfig::load())
            .insert_resource(FlamethrowerConfig::load())
            .init_resource::<AttackDefinitions>()
            .init_resource::<AttackInputState>()
            .init_resource::<AttackLoadout>()
            .init_resource::<PlayerIntent>()
            .init_resource::<SelectChordState>()
            .configure_sets(Update, MovementSystems.before(ConfinementSystems))
            .add_message::<CameraShakeEvent>()
            .add_observer(on_camera_shake)
            .add_message::<PlayerStartupEvent>()
            .add_observer(on_player_startup)
            .add_message::<PlayerShutdownEvent>()
            .add_observer(on_player_shutdown)
            .add_active_systems::<PlayerPlugin, _>((
                resolve_player_intent,
                tick_attack_lifetimes,
                tick_webbed_status,
                despawn_expired_attacks,
                detect_player_attack.after(resolve_player_intent),
                manage_flamethrower.after(resolve_player_intent),
                update_flamethrower.after(manage_flamethrower),
                flamethrower_damage.after(update_flamethrower),
                player_movement
                    .in_set(MovementSystems)
                    .after(resolve_player_intent)
                    .after(tick_webbed_status),
                confine_player_movement.in_set(ConfinementSystems),
            ));
    }
}

/// Convenience holder for the chosen crosshair sprite and metadata.
pub struct CrosshairInfo {
    pub sprite: Handle<CxAsset<CxSpriteData>>,
    pub crosshair: Crosshair,
}

impl CrosshairInfo {
    /** REVIEW if these are needed */
    /// Returns the underlying sprite handle (consumes the wrapper).
    #[must_use]
    pub fn get_sprite(crosshair: CrosshairInfo) -> Handle<CxAsset<CxSpriteData>> {
        crosshair.sprite
    }

    /** REVIEW if these are needed */
    /// Returns the crosshair metadata (consumes the wrapper).
    #[must_use]
    pub fn get_crosshair(crosshair: CrosshairInfo) -> Crosshair {
        crosshair.crosshair
    }

    /// Loads the sprite matching the configured index and returns the combined info.
    pub fn crosshair_sprite(
        asset_server: &mut CxAssets<CxSprite>,
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
