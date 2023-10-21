use bevy::prelude::*;

use crate::{
    cutscene::{
        components::{Cinematic, CurrentCutsceneStep},
        data::CinematicData,
        resources::{CutsceneProgress, CutsceneTime},
    },
    stage::components::CurrentStageStep,
};

pub fn read_step_trigger(
    mut commands: Commands,
    mut progress: ResMut<CutsceneProgress>,
    query: Query<Entity, (With<Cinematic>, Without<CurrentCutsceneStep>)>,
    data: Res<CinematicData>,
    time: Res<CutsceneTime>,
) {
    if let Ok(entity) = query.get_single() {
        progress.index += 1;

        if let Some(action) = data.steps.get(progress.index) {
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(CurrentCutsceneStep {
                started: time.elapsed,
            });

            entity_commands.insert(action.clone());
        }
    }
}
