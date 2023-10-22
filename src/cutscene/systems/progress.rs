use bevy::prelude::*;

use crate::{
    components::StepStarted,
    cutscene::{
        components::Cinematic,
        data::{CinematicData, CinematicStageStep, CutsceneElapse},
        resources::{CutsceneProgress, CutsceneTime},
    },
};

pub fn read_step_trigger(
    mut commands: Commands,
    mut progress: ResMut<CutsceneProgress>,
    query: Query<Entity, (With<Cinematic>, Without<StepStarted>)>,
    data: Res<CinematicData>,
    time: Res<CutsceneTime>,
) {
    for entity in query.iter() {
        if let Some(action) = data.steps.get(progress.index) {
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(StepStarted(time.elapsed));
            match action.clone() {
                CinematicStageStep::CutsceneAnimationSpawn(x) => entity_commands.insert(x),
                CinematicStageStep::CutsceneAwaitInput(x) => entity_commands.insert(x),
                CinematicStageStep::CutsceneDespawn(x) => entity_commands.insert(x),
                CinematicStageStep::CutsceneMusicDespawn(x) => entity_commands.insert(x),
                CinematicStageStep::CutsceneMusicSpawn(x) => entity_commands.insert(x),
                CinematicStageStep::CutsceneElapse(x) => entity_commands.insert(x),
                CinematicStageStep::CutsceneSpawn(x) => entity_commands.insert(x),
            };

            progress.index += 1;
        }
    }
}

pub fn check_cutscene_elapsed_step(
    mut commands: Commands,
    query: Query<(Entity, &StepStarted, &CutsceneElapse), With<Cinematic>>,
    time: ResMut<CutsceneTime>,
) {
    for (entity, started, elapse) in query.iter() {
        if started.0 + elapse.0 < time.elapsed {
            commands
                .entity(entity)
                .remove::<StepStarted>()
                .remove::<CutsceneElapse>();
        }
    }
}
