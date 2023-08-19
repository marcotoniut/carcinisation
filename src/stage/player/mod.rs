pub mod components;
pub mod systems;

use bevy::prelude::*;

use self::systems::*;
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
                    enemy_hit_player,
                    player_hit_star,
                )
                    .run_if(in_state(AppState::Game))
                    .run_if(in_state(GameState::Running)),
            )
            .add_systems(OnExit(AppState::Game), despawn_player);
    }
}
