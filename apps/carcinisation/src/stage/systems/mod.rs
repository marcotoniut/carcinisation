//! Stage system orchestration: camera, damage, movement, lifecycle, and spawn logic.

pub mod camera;
pub mod damage;
pub mod movement;
pub mod setup;
pub mod spawn;

use super::{
    attack::components::EnemyAttack,
    bundles::*,
    components::{
        interactive::{Dead, Object},
        placement::spawn_floor_depths,
        CinematicStageStep, CurrentStageStep, MovementStageStep, Stage, StageElapsedStarted,
        StageEntity, StopStageStep,
    },
    data::*,
    destructible::components::Destructible,
    enemy::components::Enemy,
    events::{NextStepEvent, StageClearedTrigger, StageDeathEvent},
    player::components::Player,
    resources::{StageActionTimer, StageProgress, StageStepSpawner, StageTimeDomain},
    StageProgressState,
};
use crate::components::VolumeSettings;
use crate::core::time::TimeMultiplier;
use crate::pixel::PxAssets;
use crate::{
    components::{DespawnMark, Music},
    game::{
        data::DEATH_SCORE_PENALTY, events::GameOverTrigger, resources::Lives,
        score::components::Score, GameProgressState,
    },
    globals::{mark_for_despawn_by_query, DEBUG_STAGESTEP},
    input::GBInput,
    plugins::movement::linear::components::{
        extra::LinearMovement2DReachCheck, LinearMovementBundle, LinearPositionRemovalBundle,
        TargetingPositionX, TargetingPositionY,
    },
    systems::{camera::CameraPos, spawn::make_music_bundle},
    transitions::trigger_transition,
};
use assert_assets_path::assert_assets_path;
use bevy::{audio::PlaybackMode, prelude::*};
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::{PxSprite, PxSubPosition};

/// @system Toggles the game state between running and paused on `Start`.
pub fn toggle_game(
    gb_input: Res<ActionState<GBInput>>,
    state: Res<State<GameProgressState>>,
    stage_state: Res<State<StageProgressState>>,
    mut next_state: ResMut<NextState<GameProgressState>>,
) {
    if *stage_state.get() == StageProgressState::GameOver {
        return;
    }

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

/// @system Advances the stage-local time domain while gameplay is running.
pub fn tick_stage_time(
    real_time: Res<Time>,
    state: Res<State<StageProgressState>>,
    mult: Option<Res<TimeMultiplier<StageTimeDomain>>>,
    mut stage_time: ResMut<Time<StageTimeDomain>>,
) {
    if state.get() != &StageProgressState::Running {
        return;
    }

    let base_dt = real_time.delta();
    let scaled_dt = mult.map(|m| base_dt.mul_f32(m.value)).unwrap_or(base_dt);

    stage_time.advance_by(scaled_dt);
}

// REVIEW
/// @system Spawns the core stage bundle and kicks off gameplay.
pub fn spawn_current_stage_bundle(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut state: ResMut<NextState<GameProgressState>>,
    stage_data: Res<StageData>,
) {
    commands
        .spawn((
            Stage,
            Name::new("Stage"),
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
        ))
        .with_children(|p0| {
            p0.spawn(BackgroundBundle::new(
                assets_sprite.load(stage_data.background_path.clone()),
            ));
            p0.spawn(SkyboxBundle::new(
                &mut assets_sprite,
                stage_data.skybox.clone(),
            ));
        });

    state.set(GameProgressState::Running);
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

/// @system Evaluates the current stage progress and transitions between states.
pub fn update_stage(
    mut commands: Commands,
    state: Res<State<StageProgressState>>,
    stage_query: Query<Entity, With<Stage>>,
    mut next_state: ResMut<NextState<StageProgressState>>,
    stage_progress: ResMut<StageProgress>,
    stage_data: Res<StageData>,
) {
    match state.to_owned() {
        StageProgressState::Initial => {
            next_state.set(StageProgressState::Running);
        }
        StageProgressState::Running => {
            if let Some(action) = stage_data.steps.get(stage_progress.index) {
                if DEBUG_STAGESTEP {
                    let curr_action = match action {
                        StageStep::Movement { .. } => "movement".to_string(),
                        StageStep::Stop { .. } => "stop".to_string(),
                        StageStep::Cinematic { .. } => "cinematic".to_string(),
                    };

                    info!("curr action: {}", curr_action);
                }
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

/// @system Triggers `StageClearedTrigger` once all steps are complete.
pub fn check_staged_cleared(
    mut commands: Commands,
    stage_progress: Res<StageProgress>,
    stage_data: Res<StageData>,
) {
    if stage_progress.index >= stage_data.steps.len() {
        commands.trigger(StageClearedTrigger);
    }
}

/// @trigger Handles cleanup and celebration when the stage is cleared.
#[allow(clippy::too_many_arguments)]
pub fn on_stage_cleared(
    _trigger: On<StageClearedTrigger>,
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

/// @trigger Responds to `StageDeathEvent`, transitioning to Game Over or restarting.
#[allow(clippy::too_many_arguments)]
pub fn on_death(
    _trigger: On<StageDeathEvent>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<StageProgressState>>,
    mut game_over_event_writer: MessageWriter<GameOverTrigger>,
    lives: Res<Lives>,
    score: Res<Score>,
    attack_query: Query<Entity, With<EnemyAttack>>,
    destructible_query: Query<Entity, With<Destructible>>,
    enemy_query: Query<Entity, With<Enemy>>,
    music_query: Query<Entity, With<Music>>,
    object_query: Query<Entity, With<Object>>,
    player_query: Query<Entity, With<Player>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
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

    if 0 == lives.0 {
        game_over_event_writer.write(GameOverTrigger { score: score.value });
        next_state.set(StageProgressState::GameOver);
    } else {
        next_state.set(StageProgressState::Death);
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
    if let Ok(entity) = query.single() {
        if let Some(action) = data.steps.get(progress.index) {
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
                StageStep::Movement(step) => {
                    entity_commands.insert(step.clone());
                }
                StageStep::Stop(step) => {
                    entity_commands.insert(step.clone());
                }
            }
        }
    }
}

/// @system Prepares cinematic steps (placeholder for future cutscene integration).
pub fn initialise_cinematic_step(
    _next_state: ResMut<NextState<GameProgressState>>,
    query: Query<(Entity, &CinematicStageStep), (With<Stage>, Added<CinematicStageStep>)>,
) {
    if query.single().is_ok() {
        // next_state.set(GameState::Cutscene);
    }
}

/// @system Sets up camera movement and spawns tied to a movement step.
pub fn initialise_movement_step(
    mut commands: Commands,
    query: Query<(Entity, &MovementStageStep), (With<Stage>, Added<MovementStageStep>)>,
    camera_query: Query<(Entity, &PxSubPosition), With<CameraPos>>,
) {
    if let Ok((
        _,
        MovementStageStep {
            coordinates,
            base_speed,
            spawns,
            floor_depths,
        },
    )) = query.single()
    {
        if let Ok((camera_entity, position)) = camera_query.single() {
            let direction = *coordinates - position.0;
            let speed = direction.normalize_or_zero() * *base_speed * GAME_BASE_SPEED;

            commands
                .entity(camera_entity)
                .insert(
                    LinearMovementBundle::<StageTimeDomain, TargetingPositionX>::new(
                        position.x,
                        coordinates.x,
                        speed.x,
                    ),
                )
                .insert(
                    LinearMovementBundle::<StageTimeDomain, TargetingPositionY>::new(
                        position.y,
                        coordinates.y,
                        speed.y,
                    ),
                )
                .insert(LinearMovement2DReachCheck::<
                    StageTimeDomain,
                    TargetingPositionX,
                    TargetingPositionY,
                >::new())
                .insert(StageStepSpawner::new(spawns.clone()));

            if let Some(floor_depths) = floor_depths {
                spawn_floor_depths(&mut commands, floor_depths);
            }
        }
    }
}

/// @system Seeds stop-step spawners and optional floor markers.
pub fn initialise_stop_step(
    mut commands: Commands,
    query: Query<(Entity, &StopStageStep), (With<Stage>, Added<StopStageStep>)>,
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
            spawn_floor_depths(&mut commands, floor_depths);
        }
    }
}

/// @system Advances once the camera finishes its scripted movement.
pub fn check_movement_step_reached(
    mut commands: Commands,
    step_query: Query<Entity, With<MovementStageStep>>,
    camera_query: Query<
        (
            Entity,
            &LinearMovement2DReachCheck<StageTimeDomain, TargetingPositionX, TargetingPositionY>,
        ),
        With<CameraPos>,
    >,
) {
    if let Ok((camera_entity, reach_check)) = camera_query.single() {
        if reach_check.reached() {
            for _ in step_query.iter() {
                let mut entity_commands = commands.entity(camera_entity);
                entity_commands.remove::<LinearMovement2DReachCheck<
                    StageTimeDomain,
                    TargetingPositionX,
                    TargetingPositionY,
                >>();
                entity_commands
                    .remove::<LinearPositionRemovalBundle<StageTimeDomain, TargetingPositionX>>();
                entity_commands
                    .remove::<LinearPositionRemovalBundle<StageTimeDomain, TargetingPositionY>>();
                commands.trigger(NextStepEvent);
            }
        }
    }
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
            .map(|max_duration| current_step.started + max_duration <= stage_time.elapsed())
            .unwrap_or(false)
        {
            commands.trigger(NextStepEvent);
        }
    }
}

/// @system Placeholder hook for updating cinematic steps each frame.
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
    query: Query<(Entity, &MovementStageStep), With<Stage>>,
) {
    for (entity, _) in query.iter() {
        commands
            .entity(entity)
            .remove::<MovementStageStep>()
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
