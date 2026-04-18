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
        systems::{CameraStepTween, camera},
    },
    systems::camera::CameraPos,
};

/// Despawns all entities that belong to the current stage run.
///
/// Covers the `Stage` controller entity and every entity tagged with
/// `StageEntity` (HUD, background, skybox, music, attack effects, etc.).
/// Used by both the checkpoint-restart and game-over-exit paths to ensure
/// a clean slate.
pub fn despawn_stage_entities(
    commands: &mut Commands,
    stage_query: &Query<Entity, With<Stage>>,
    stage_entity_query: &Query<Entity, With<StageEntity>>,
) {
    mark_for_despawn_by_query(commands, stage_query);
    mark_for_despawn_by_query(commands, stage_entity_query);
}

/// # Panics
///
/// Panics if `StageRestart::from_checkpoint` is `true` but the stage data has
/// no checkpoint defined.
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
    mut camera_query: Query<(Entity, Option<&CameraShake>, &mut PxSubPosition), With<CameraPos>>,
    camera_tween_query: Query<Entity, With<CameraStepTween>>,
    mut stage_progress: ResMut<StageProgress>,
    mut stage_state: ResMut<NextState<StageProgressState>>,
    mut stage_time: ResMut<Time<StageTimeDomain>>,
    mut stage_action_timer: ResMut<StageActionTimer>,
    mut restart_reader: MessageReader<StageRestart>,
) {
    for restart in restart_reader.read() {
        camera::cleanup_camera_stage_state(&mut commands, &mut camera_query, &camera_tween_query);

        let start_index = if restart.from_checkpoint {
            stage_data
                .checkpoint
                .as_ref()
                .expect("StageRestart with from_checkpoint requires a defined checkpoint")
                .step_index
        } else {
            0
        };

        // Reset progression/resources before rebuilding.
        reset_stage_progression(
            &mut stage_progress,
            &mut stage_state,
            &mut stage_time,
            &mut stage_action_timer,
            start_index,
        );

        // Clean up all stage-scoped entities so the upcoming startup runs from a clean slate.
        despawn_stage_entities(&mut commands, &stage_query, &stage_entity_query);
        // Gameplay entities that may still be alive mid-restart (on_death
        // already handles these for the game-over path).
        mark_for_despawn_by_query(&mut commands, &destructible_query);
        mark_for_despawn_by_query(&mut commands, &enemy_query);
        mark_for_despawn_by_query(&mut commands, &music_query);
        mark_for_despawn_by_query(&mut commands, &object_query);
        mark_for_despawn_by_query(&mut commands, &player_query);

        activate::<StagePlugin>(&mut commands);

        let data = stage_data.clone();
        commands.trigger(StageStartupEvent {
            data: Arc::new(data),
            from_checkpoint: restart.from_checkpoint,
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
