use bevy::{
    prelude::{Res, ResMut},
    time::Time,
};

use super::resources::TransitionUpdateTimer;

pub mod layout;

pub fn tick_timer(mut timer: ResMut<TransitionUpdateTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}
