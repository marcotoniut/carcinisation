use std::sync::Arc;

use activable::activate;
use bevy::prelude::*;

use crate::{
    components::Music,
    globals::mark_for_despawn_by_query,
    stage::{
        components::interactive::Object,
        components::{Stage, StageEntity},
        data::StageData,
        destructible::components::Destructible,
        enemy::components::Enemy,
        messages::{StageRestart, StageStartupEvent},
        player::components::Player,
        resources::{StageActionTimer, StageProgress, StageTimeDomain},
        StagePlugin, StageProgressState,
    },
};

#[allow(clippy::too_many_arguments)]
pub fn handle_stage_restart(
    mut commands: Commands,
    stage_data: Res<StageData>,
    stage_query: Query<Entity, With<Stage>>,
    stage_entity_query: Query<Entity, With<StageEntity>>,
    destructible_query: Query<Entity, With<Destructible>>,
    enemy_query: Query<Entity, With<Enemy>>,
    music_query: Query<Entity, With<Music>>,
    object_query: Query<Entity, With<Object>>,
    player_query: Query<Entity, With<Player>>,
    mut stage_progress: ResMut<StageProgress>,
    mut stage_state: ResMut<NextState<StageProgressState>>,
    mut stage_time: ResMut<Time<StageTimeDomain>>,
    mut stage_action_timer: ResMut<StageActionTimer>,
    mut restart_reader: MessageReader<StageRestart>,
    mut startup_writer: MessageWriter<StageStartupEvent>,
) {
    for _ in restart_reader.read() {
        // Reset progression/resources before rebuilding.
        stage_progress.index = 0;
        stage_state.set(StageProgressState::Initial);
        *stage_time = Time::default();
        stage_action_timer.timer.reset();
        stage_action_timer.stop();

        // Clean up all stage-scoped entities so the upcoming startup runs from a clean slate.
        mark_for_despawn_by_query(&mut commands, &stage_query);
        mark_for_despawn_by_query(&mut commands, &stage_entity_query);
        mark_for_despawn_by_query(&mut commands, &destructible_query);
        mark_for_despawn_by_query(&mut commands, &enemy_query);
        mark_for_despawn_by_query(&mut commands, &music_query);
        mark_for_despawn_by_query(&mut commands, &object_query);
        mark_for_despawn_by_query(&mut commands, &player_query);

        activate::<StagePlugin>(&mut commands);

        let data = stage_data.clone();
        startup_writer.write(StageStartupEvent {
            data: Arc::new(data),
        });
    }
}

pub struct StageRestartPlugin;

impl Plugin for StageRestartPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StageRestart>().add_systems(
            Update,
            handle_stage_restart.run_if(resource_exists::<StageData>),
        );
    }
}
