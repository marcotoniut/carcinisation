use bevy::prelude::*;

use super::super::resources::TransitionCounter;

pub fn insert_transition_counter(mut commands: Commands) {
    commands.insert_resource(TransitionCounter::default());
}

pub fn remove_transition_counter(mut commands: Commands) {
    commands.remove_resource::<TransitionCounter>();
}
