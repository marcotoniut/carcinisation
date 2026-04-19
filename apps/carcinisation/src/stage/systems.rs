//! Stage system orchestration: camera, damage, movement, lifecycle, and spawn logic.

pub mod camera;
pub mod damage;
pub mod movement;
pub mod setup;
pub mod spawn;

use super::{
    StageProgressState,
    attack::components::EnemyAttack,
    components::{
        CinematicStageStep, CurrentStageStep, Stage, StageElapsedStarted, StageEntity,
        StopStageStep, TweenStageStep,
        interactive::{Dead, Object},
        placement::{Floor, despawn_floor_entities, spawn_floor_depths},
    },
    data::{GAME_BASE_SPEED, StageData, StageStep},
    destructible::components::Destructible,
    enemy::components::Enemy,
    messages::{NextStepEvent, StageClearedEvent, StageDeathEvent},
    player::components::{CameraShake, Player},
    projection::effective_projection,
    resources::{
        ActiveProjection, StageActionTimer, StageProgress, StageStepSpawner, StageTimeDomain,
    },
};
use crate::components::VolumeSettings;
use crate::{
    components::{DespawnMark, Music},
    core::time::TimeShouldRun,
    game::{
        GameProgressState, data::DEATH_SCORE_PENALTY, messages::GameOverEvent, resources::Lives,
        score::components::Score,
    },
    globals::{DEBUG_STAGESTEP, mark_for_despawn_by_query},
    input::GBInput,
    systems::{camera::CameraPos, spawn::make_music_bundle},
    transitions::trigger_transition,
};
use assert_assets_path::assert_assets_path;
use bevy::{audio::PlaybackMode, ecs::hierarchy::ChildOf, prelude::*};
use carapace::prelude::PxSubPosition;
use cween::linear::components::{
    TargetingValueX, TargetingValueY, TweenChildBundle, extra::LinearTween2DReachCheck,
};
use leafwing_input_manager::prelude::ActionState;

/// @system Toggles the game state between running and paused on `Start`.
pub fn toggle_game(
    gb_input: Res<ActionState<GBInput>>,
    state: Res<State<GameProgressState>>,
    mut next_state: ResMut<NextState<GameProgressState>>,
) {
    if gb_input.just_pressed(&GBInput::Start) {
        if *state.get() == GameProgressState::Running {
            #[cfg(debug_assertions)]
            info!("Game Paused.");

            next_state.set(GameProgressState::Paused);
        } else {
            #[cfg(debug_assertions)]
            info!("Game Running.");

            next_state.set(GameProgressState::Running);
        }
    }
}

/// @system Updates whether the stage-local time domain should advance.
pub fn update_stage_time_should_run(
    stage_state: Res<State<StageProgressState>>,
    game_state: Res<State<GameProgressState>>,
    mut should_run: ResMut<TimeShouldRun<StageTimeDomain>>,
) {
    should_run.value = *stage_state.get() == StageProgressState::Running
        && matches!(
            *game_state.get(),
            GameProgressState::Running | GameProgressState::Cutscene
        );
}

// TODO combine the two and use just_finished
/// @system Advances the stage action timer every frame.
pub fn tick_stage_step_timer(
    mut timer: ResMut<StageActionTimer>,
    time: Res<Time<StageTimeDomain>>,
) {
    if timer.timer.is_paused() {
        return;
    }
    timer.timer.tick(time.delta());
}

/// @system Emits `NextStepEvent` when the action timer elapses.
pub fn check_stage_step_timer(timer: Res<StageActionTimer>, mut commands: Commands) {
    if timer.timer.is_paused() || !timer.timer.is_finished() {
        return;
    }
    commands.trigger(NextStepEvent);
}

/// @system Keeps [`ActiveProjection`] in sync with the current step.
///
/// `StageData` is removed when the stage is torn down, so the resource is
/// optional — the system simply no-ops after cleanup.
pub fn update_active_projection(
    stage_data: Option<Res<StageData>>,
    stage_progress: Res<StageProgress>,
    mut active: ResMut<ActiveProjection>,
) {
    let Some(stage_data) = stage_data else {
        return;
    };
    active.0 = effective_projection(&stage_data, stage_progress.index);
}

/// @system Evaluates the current stage progress and transitions between states.
///
/// `StageData` is optional because it is removed during teardown (game-over
/// continue path).  The system no-ops when absent.
pub fn update_stage(
    mut commands: Commands,
    state: Res<State<StageProgressState>>,
    stage_query: Query<Entity, With<Stage>>,
    mut next_state: ResMut<NextState<StageProgressState>>,
    stage_progress: ResMut<StageProgress>,
    stage_data: Option<Res<StageData>>,
) {
    match state.to_owned() {
        StageProgressState::Initial => {
            next_state.set(StageProgressState::Running);
        }
        StageProgressState::Running => {
            if let Some(ref stage_data) = stage_data
                && let Some(action) = stage_data.steps.get(stage_progress.index)
                && DEBUG_STAGESTEP
            {
                let curr_action = match action {
                    StageStep::Tween { .. } => "tween".to_string(),
                    StageStep::Stop { .. } => "stop".to_string(),
                    StageStep::Cinematic { .. } => "cinematic".to_string(),
                };

                info!("curr action: {}", curr_action);
            }
        }
        StageProgressState::Clear => {
            if let Ok(entity) = stage_query.single() {
                commands.entity(entity).insert(DespawnMark);

                // TODO
                // commands.spawn(make_stage_cleared_bundle());
            }

            next_state.set(StageProgressState::Cleared);
        }
        _ => {}
    }
}

/// @system Triggers `StageClearedEvent` once all steps are complete.
pub fn check_staged_cleared(
    mut commands: Commands,
    stage_progress: Res<StageProgress>,
    stage_data: Res<StageData>,
) {
    if stage_progress.index >= stage_data.steps.len() {
        commands.trigger(StageClearedEvent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    fn run_gate(stage: StageProgressState, game: GameProgressState) -> bool {
        let mut world = World::new();
        world.insert_resource(State::new(stage));
        world.insert_resource(State::new(game));
        world.insert_resource(TimeShouldRun::<StageTimeDomain>::default());

        let mut system_state: SystemState<(
            Res<State<StageProgressState>>,
            Res<State<GameProgressState>>,
            ResMut<TimeShouldRun<StageTimeDomain>>,
        )> = SystemState::new(&mut world);

        let (stage_state, game_state, should_run) = system_state.get_mut(&mut world);
        update_stage_time_should_run(stage_state, game_state, should_run);
        system_state.apply(&mut world);

        world.resource::<TimeShouldRun<StageTimeDomain>>().value
    }

    #[test]
    fn stage_time_runs_while_running() {
        assert!(run_gate(
            StageProgressState::Running,
            GameProgressState::Running
        ));
    }

    #[test]
    fn stage_time_stops_while_paused() {
        assert!(!run_gate(
            StageProgressState::Running,
            GameProgressState::Paused
        ));
    }

    #[test]
    fn stage_time_runs_during_cutscene() {
        assert!(run_gate(
            StageProgressState::Running,
            GameProgressState::Cutscene
        ));
    }

    #[test]
    fn stage_time_stops_outside_running_stage() {
        assert!(!run_gate(
            StageProgressState::Death,
            GameProgressState::Running
        ));
    }

    /// Helper encoding the same routing logic as `on_death`.
    fn death_routes_to_continue(lives: u8, has_checkpoint: bool) -> bool {
        lives > 0 && has_checkpoint
    }

    #[test]
    fn death_with_lives_and_checkpoint_routes_to_continue() {
        assert!(death_routes_to_continue(2, true));
    }

    #[test]
    fn death_with_lives_but_no_checkpoint_routes_to_game_over() {
        assert!(!death_routes_to_continue(2, false));
    }

    #[test]
    fn death_with_no_lives_and_checkpoint_routes_to_game_over() {
        assert!(!death_routes_to_continue(0, true));
    }

    #[test]
    fn death_with_no_lives_and_no_checkpoint_routes_to_game_over() {
        assert!(!death_routes_to_continue(0, false));
    }

    /// Systems that read `StageData` as `Option<Res<>>` must not panic when
    /// the resource is removed during teardown.
    #[test]
    fn update_active_projection_survives_stage_data_removal() {
        use crate::stage::resources::ActiveProjection;

        let mut world = World::new();
        let stage_data = StageData {
            name: "test".into(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: crate::stage::data::SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: vec![],
            steps: vec![],
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
            checkpoint: None,
        };

        world.insert_resource(stage_data);
        world.insert_resource(StageProgress { index: 0 });
        world.insert_resource(ActiveProjection::default());

        // Normal: runs fine with StageData present.
        let mut system_state: SystemState<(
            Option<Res<StageData>>,
            Res<StageProgress>,
            ResMut<ActiveProjection>,
        )> = SystemState::new(&mut world);
        let (data, progress, active) = system_state.get_mut(&mut world);
        update_active_projection(data, progress, active);
        system_state.apply(&mut world);

        // Teardown: StageData removed — should no-op, not panic.
        world.remove_resource::<StageData>();
        let (data, progress, active) = system_state.get_mut(&mut world);
        assert!(data.is_none());
        update_active_projection(data, progress, active);
        system_state.apply(&mut world);
    }
}

/// @trigger Handles cleanup and celebration when the stage is cleared.
#[allow(clippy::too_many_arguments)]
pub fn on_stage_cleared(
    _trigger: On<StageClearedEvent>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<StageProgressState>>,
    destructible_query: Query<Entity, With<Destructible>>,
    enemy_query: Query<Entity, With<Enemy>>,
    music_query: Query<Entity, With<Music>>,
    object_query: Query<Entity, With<Object>>,
    player_query: Query<Entity, With<Player>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
    stage_data: Res<StageData>,
) {
    if let Some(request) = &stage_data.on_end_transition_o {
        trigger_transition(&mut commands, request);
    }

    mark_for_despawn_by_query(&mut commands, &destructible_query);
    mark_for_despawn_by_query(&mut commands, &enemy_query);
    mark_for_despawn_by_query(&mut commands, &music_query);
    mark_for_despawn_by_query(&mut commands, &object_query);
    mark_for_despawn_by_query(&mut commands, &player_query);

    let (player, settings, system_bundle, music_tag) = make_music_bundle(
        &asset_server,
        &volume_settings,
        assert_assets_path!("audio/music/intro.ogg").to_string(),
        PlaybackMode::Despawn,
    );

    commands.spawn((player, settings, system_bundle, music_tag, StageEntity));

    next_state.set(StageProgressState::Cleared);
}

/// @system Applies death penalties and fires `StageDeathEvent` when the player dies.
pub fn check_stage_death(
    mut commands: Commands,
    mut lives: ResMut<Lives>,
    mut score: ResMut<Score>,
    player_query: Query<&Player, Added<Dead>>,
) {
    if player_query.single().is_ok() {
        score.add(-DEATH_SCORE_PENALTY);
        lives.0 = lives.0.saturating_sub(1);
        commands.trigger(StageDeathEvent);
    }
}

/// @trigger Responds to `StageDeathEvent`, transitioning to Death (continue) or Game Over.
///
/// Routes to `StageProgressState::Death` only when the player still has lives
/// **and** the stage defines an authored checkpoint.  Otherwise routes to
/// `StageProgressState::GameOver`.
#[allow(clippy::too_many_arguments)]
pub fn on_death(
    _trigger: On<StageDeathEvent>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<StageProgressState>>,
    mut game_over_event_writer: MessageWriter<GameOverEvent>,
    lives: Res<Lives>,
    score: Res<Score>,
    stage_data: Res<StageData>,
    attack_query: Query<Entity, With<EnemyAttack>>,
    destructible_query: Query<Entity, With<Destructible>>,
    enemy_query: Query<Entity, With<Enemy>>,
    music_query: Query<Entity, With<Music>>,
    object_query: Query<Entity, With<Object>>,
    player_query: Query<Entity, With<Player>>,
    mut camera_query: Query<(Entity, Option<&CameraShake>, &mut PxSubPosition), With<CameraPos>>,
    camera_tween_query: Query<Entity, With<CameraStepTween>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    camera::cleanup_camera_stage_state(&mut commands, &mut camera_query, &camera_tween_query);

    mark_for_despawn_by_query(&mut commands, &attack_query);
    mark_for_despawn_by_query(&mut commands, &destructible_query);
    mark_for_despawn_by_query(&mut commands, &enemy_query);
    mark_for_despawn_by_query(&mut commands, &music_query);
    mark_for_despawn_by_query(&mut commands, &object_query);
    mark_for_despawn_by_query(&mut commands, &player_query);

    let (player, settings, system_bundle, music_tag) = make_music_bundle(
        &asset_server,
        &volume_settings,
        assert_assets_path!("audio/music/game_over.ogg").to_string(),
        PlaybackMode::Despawn,
    );
    commands.spawn((player, settings, system_bundle, music_tag, StageEntity));

    if lives.0 > 0 && stage_data.checkpoint.is_some() {
        next_state.set(StageProgressState::Death);
    } else {
        game_over_event_writer.write(GameOverEvent { score: score.value });
        next_state.set(StageProgressState::GameOver);
    }
}

/// @system Applies the next stage step to the stage entity when none is active.
pub fn read_step_trigger(
    mut commands: Commands,
    mut progress: ResMut<StageProgress>,
    query: Query<Entity, (With<Stage>, Without<CurrentStageStep>)>,
    data: Res<StageData>,
    time: Res<Time<StageTimeDomain>>,
) {
    if let Ok(entity) = query.single()
        && let Some(action) = data.steps.get(progress.index)
    {
        progress.index += 1;

        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            CurrentStageStep {
                started: time.elapsed(),
            },
            // StageElapse::new(action.elapse),
            StageElapsedStarted(time.elapsed()),
        ));

        match action {
            StageStep::Cinematic(step) => {
                entity_commands.insert(step.clone());
            }
            StageStep::Tween(step) => {
                entity_commands.insert(step.clone());
            }
            StageStep::Stop(step) => {
                entity_commands.insert(step.clone());
            }
        }
    }
}

/// @system Prepares cinematic steps (stub — cutscene integration pending).
pub fn initialise_cinematic_step(
    _next_state: ResMut<NextState<GameProgressState>>,
    query: Query<(Entity, &CinematicStageStep), (With<Stage>, Added<CinematicStageStep>)>,
) {
    if query.single().is_ok() {
        // next_state.set(GameState::Cutscene);
    }
}

/// Marker component for tween children spawned for camera tween steps.
#[derive(Component, Clone, Debug)]
pub struct CameraStepTween;

/// @system Sets up camera movement and spawns tied to a movement step.
/// Spawns movement children to drive the camera, and attaches a reach check component.
pub fn initialise_movement_step(
    mut commands: Commands,
    query: Query<(Entity, &TweenStageStep), (With<Stage>, Added<TweenStageStep>)>,
    camera_query: Query<(Entity, &PxSubPosition), With<CameraPos>>,
    floor_query: Query<Entity, With<Floor>>,
) {
    if let Ok((
        _,
        TweenStageStep {
            coordinates,
            base_speed,
            spawns,
            floor_depths,
            ..
        },
    )) = query.single()
        && let Ok((camera_entity, position)) = camera_query.single()
    {
        let direction = *coordinates - position.0;
        let speed = direction.normalize_or_zero() * *base_speed * GAME_BASE_SPEED;

        commands.entity(camera_entity).insert((
            TargetingValueX::new(position.0.x),
            TargetingValueY::new(position.0.y),
        ));

        // Spawn movement children for the camera
        commands.spawn((
            TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
                camera_entity,
                position.x,
                coordinates.x,
                speed.x,
            ),
            CameraStepTween,
            Name::new("Camera Movement X"),
        ));

        commands.spawn((
            TweenChildBundle::<StageTimeDomain, TargetingValueY>::new(
                camera_entity,
                position.y,
                coordinates.y,
                speed.y,
            ),
            CameraStepTween,
            Name::new("Camera Movement Y"),
        ));

        // Add reach check to camera
        commands
            .entity(camera_entity)
            .insert(LinearTween2DReachCheck::<
                StageTimeDomain,
                TargetingValueX,
                TargetingValueY,
            >::new())
            .insert(StageStepSpawner::new(spawns.clone()));

        if let Some(floor_depths) = floor_depths {
            despawn_floor_entities(&mut commands, &floor_query);
            spawn_floor_depths(&mut commands, floor_depths);
        }
    }
}

/// @system Seeds stop-step spawners and optional floor markers.
pub fn initialise_stop_step(
    mut commands: Commands,
    query: Query<(Entity, &StopStageStep), (With<Stage>, Added<StopStageStep>)>,
    floor_query: Query<Entity, With<Floor>>,
) {
    if let Ok((
        entity,
        StopStageStep {
            spawns,
            floor_depths,
            ..
        },
    )) = query.single()
    {
        commands
            .entity(entity)
            .insert(StageStepSpawner::new(spawns.clone()));

        if let Some(floor_depths) = floor_depths {
            despawn_floor_entities(&mut commands, &floor_query);
            spawn_floor_depths(&mut commands, floor_depths);
        }
    }
}

/// @system Advances once the camera finishes its scripted movement.
pub fn check_movement_step_reached(
    mut commands: Commands,
    step_query: Query<Entity, With<TweenStageStep>>,
    camera_query: Query<
        (
            Entity,
            &LinearTween2DReachCheck<StageTimeDomain, TargetingValueX, TargetingValueY>,
        ),
        With<CameraPos>,
    >,
) {
    let Ok((camera_entity, reach_check)) = camera_query.single() else {
        return;
    };
    if !reach_check.reached() || step_query.is_empty() {
        return;
    }

    let mut entity_commands = commands.entity(camera_entity);
    entity_commands
        .remove::<LinearTween2DReachCheck<StageTimeDomain, TargetingValueX, TargetingValueY>>();
    commands.trigger(NextStepEvent);
}

/// @system Advances stop steps once their optional duration expires.
pub fn check_stop_step_finished_by_duration(
    mut commands: Commands,
    query: Query<(&StopStageStep, &CurrentStageStep), With<Stage>>,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for (step, current_step) in query.iter() {
        if step
            .max_duration
            .is_some_and(|max_duration| current_step.started + max_duration <= stage_time.elapsed())
        {
            commands.trigger(NextStepEvent);
        }
    }
}

/// @system Updates cinematic steps each frame (stub — not yet wired).
pub fn update_cinematic_step(
    _commands: Commands,
    query: Query<(Entity, &CinematicStageStep), With<Stage>>,
) {
    for (_entity, _) in query.iter() {}
}

/// @trigger Removes cinematic step markers after `NextStepEvent` fires.
pub fn on_next_step_cleanup_cinematic_step(
    _trigger: On<NextStepEvent>,
    mut commands: Commands,
    query: Query<(Entity, &CinematicStageStep), With<Stage>>,
) {
    for (entity, _) in query.iter() {
        commands
            .entity(entity)
            .remove::<CinematicStageStep>()
            .remove::<CurrentStageStep>();
    }
}

/// @trigger Cleans up movement step components once the stage advances.
pub fn on_next_step_cleanup_movement_step(
    _trigger: On<NextStepEvent>,
    mut commands: Commands,
    query: Query<(Entity, &TweenStageStep), With<Stage>>,
) {
    for (entity, _) in query.iter() {
        commands
            .entity(entity)
            .remove::<TweenStageStep>()
            .remove::<StageStepSpawner>()
            .remove::<CurrentStageStep>();
    }
}

/// @trigger Cleans up stop step components once the stage advances.
pub fn on_next_step_cleanup_stop_step(
    _trigger: On<NextStepEvent>,
    mut commands: Commands,
    query: Query<(Entity, &StopStageStep), With<Stage>>,
) {
    for (entity, _) in query.iter() {
        commands
            .entity(entity)
            .remove::<StopStageStep>()
            .remove::<StageStepSpawner>()
            .remove::<CurrentStageStep>();
    }
}
#[cfg(debug_assertions)]
/// @system Logs parent chains where `InheritedVisibility` is missing to guard against `[B0004]`.
pub fn debug_visibility_hierarchy(
    children: Query<(Entity, &ChildOf, Option<&Name>), With<InheritedVisibility>>,
    parents: Query<(Option<&InheritedVisibility>, Option<&Name>)>,
) {
    for (entity, parent, child_name) in &children {
        if let Ok((opt_vis, parent_name)) = parents.get(parent.0)
            && opt_vis.is_none()
        {
            info!(
                "B0004 candidate: child {:?} ({:?}), parent {:?} ({:?})",
                entity,
                child_name.map(bevy::prelude::Name::as_str),
                parent.0,
                parent_name.map(bevy::prelude::Name::as_str)
            );
        }
    }
}
