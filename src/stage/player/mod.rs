pub mod bundles;
pub mod components;
pub mod crosshair;
pub mod systems;

use bevy::prelude::*;
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

use self::{
    crosshair::{Crosshair, CrosshairSettings},
    systems::*,
};
use super::GameState;
use crate::AppState;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MovementSystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ConfinementSystemSet;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(Update, MovementSystemSet.before(ConfinementSystemSet))
            .add_systems(OnEnter(AppState::Game), spawn_player)
            .add_systems(
                Update,
                (
                    player_movement.in_set(MovementSystemSet),
                    confine_player_movement.in_set(ConfinementSystemSet),
                )
                    .run_if(in_state(AppState::Game))
                    .run_if(in_state(GameState::Running)),
            )
            .add_systems(OnExit(AppState::Game), despawn_player);
    }
}

pub struct CrosshairInfo {
    pub sprite: Handle<seldom_pixel::asset::PxAsset<seldom_pixel::sprite::PxSpriteData>>,
    pub crosshair: Crosshair,
}

impl CrosshairInfo {
    pub fn get_sprite(
        crosshair: CrosshairInfo,
    ) -> Handle<seldom_pixel::asset::PxAsset<seldom_pixel::sprite::PxSpriteData>> {
        return crosshair.sprite;
    }

    pub fn get_crosshair(crosshair: CrosshairInfo) -> Crosshair {
        return crosshair.crosshair;
    }

    pub fn crosshair_sprite(
        asset_server: &mut PxAssets<PxSprite>,
        crosshair_settings: Res<CrosshairSettings>,
    ) -> CrosshairInfo {
        let sprite;
        let crosshair;

        match crosshair_settings.0 {
            2 => {
                sprite = asset_server.load("sprites/crosshairs/squiggly.png");
                crosshair = Crosshair {
                    name: "squiggly".to_string(),
                };
            }
            1 => {
                sprite = asset_server.load("sprites/crosshairs/gun_sight.png");
                crosshair = Crosshair {
                    name: "negative".to_string(),
                };
            }
            0 => {
                sprite = asset_server.load("sprites/crosshairs/gun_sight_inverted.png");
                crosshair = Crosshair {
                    name: "default".to_string(),
                };
            }
            _ => {
                sprite = asset_server.load("sprites/crosshairs/gun_sight_inverted.png");
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
