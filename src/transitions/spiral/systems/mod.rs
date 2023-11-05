pub mod layout;
pub mod setup;

use super::resources::TransitionUpdateTimer;
use bevy::{
    prelude::{Res, ResMut},
    time::Time,
};

pub fn tick_timer(mut timer: ResMut<TransitionUpdateTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}
