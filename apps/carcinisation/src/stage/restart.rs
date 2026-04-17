use std::sync::Arc;

use activable::activate;
use bevy::prelude::*;

use carapace::prelude::PxSubPosition;

use crate::{
    components::Music,
    globals::mark_for_despawn_by_query,
    stage::{
        StagePlugin, StageProgressState,
        components::interactive::Object,
        components::{Stage, StageEntity},
        data::StageData,
        destructible::components::Destructible,
        enemy::components::Enemy,
        messages::{StageRestart, StageStartupEvent},
        player::components::{CameraShake, Player},
        resources::{StageActionTimer, StageProgress, StageTimeDomain, reset_stage_progression},
    },
    systems::camera::CameraPos,
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
    mut camera_query: Query<(Entity, &CameraShake, &mut PxSubPosition), With<CameraPos>>,
    mut stage_progress: ResMut<StageProgress>,
    mut stage_state: ResMut<NextState<StageProgressState>>,
    mut stage_time: ResMut<Time<StageTimeDomain>>,
    mut stage_action_timer: ResMut<StageActionTimer>,
    mut restart_reader: MessageReader<StageRestart>,
) {
    for _ in restart_reader.read() {
        // Kill any in-progress camera shake so it doesn't bleed into the new attempt.
        if let Ok((cam, shake, mut pos)) = camera_query.single_mut() {
            pos.0 -= shake.current_offset;
            commands.entity(cam).remove::<CameraShake>();
        }

        // Reset progression/resources before rebuilding.
        reset_stage_progression(
            &mut stage_progress,
            &mut stage_state,
            &mut stage_time,
            &mut stage_action_timer,
        );

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
        commands.trigger(StageStartupEvent {
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
