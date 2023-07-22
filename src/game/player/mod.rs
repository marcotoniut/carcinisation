use bevy::prelude::*;

use self::systems::*;

pub mod components;
pub mod resources;
pub mod systems;

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
    fn build(&self, app: &mut App) {
        app.configure_set(Update, MovementSystemSet.before(ConfinementSystemSet))
            .add_systems(Startup, spawn_player)
            .add_systems(
                Update,
                (
                    player_movement.in_set(MovementSystemSet),
                    confine_player_movement.in_set(ConfinementSystemSet),
                ),
            )
            .add_systems(Update, player_hit_star)
            .add_systems(Update, enemy_hit_player);
    }
}
