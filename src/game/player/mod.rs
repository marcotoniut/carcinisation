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

// #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
// pub enum PlayerSystemSet {
//     Movement,
//     Confinement
// }

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    // fn build(&self, app: &mut App) {
    //     app.configure_set(Update, MovementSystemSet.before(ConfinementSystemSet))
    //         .add_systems(Startup, spawn_player)
    //         .add_systems(
    //             Update,
    //             (
    //                 player_movement.in_set(MovementSystemSet),
    //                 confine_player_movement.in_set(ConfinementSystemSet),
    //             ),
    //         )
    //         .add_systems(Update, player_hit_star)
    //         .add_systems(Update, enemy_hit_player);
    // }

    fn build(&self, app: &mut App) {
        app.configure_set(Update, MovementSystemSet.before(ConfinementSystemSet))
            .add_systems(OnEnter(AppState::Game), spawn_player)
            .add_systems(
                Update,
                (
                    player_movement.in_set(MovementSystemSet),
                    confine_player_movement.in_set(ConfinementSystemSet),
                    player_hit_star,
                    enemy_hit_player,
                )
                    .run_if(in_state(AppState::Game))
                    .run_if(in_state(GameState::Running)),
            )
            .add_systems(OnExit(AppState::Game), despawn_player);
    }
}
