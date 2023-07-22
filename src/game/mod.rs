use bevy::prelude::*;

// TODO should come from this module?
use crate::events::GameOver;

pub mod enemy;
pub mod player;
pub mod score;
pub mod star;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GameOver>()
            .add_plugins(enemy::EnemyPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_plugins(score::ScorePlugin)
            .add_plugins(star::StarPlugin);
    }
}
