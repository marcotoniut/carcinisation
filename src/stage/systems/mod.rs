pub mod camera;
pub mod damage;
pub mod game_over;
pub mod movement;
pub mod setup;
pub mod spawn;
pub mod state;

use super::{
    bundles::*,
    components::{
        interactive::{Dead, Object},
        placement::spawn_floor_depths,
        CinematicStageStep, CurrentStageStep, MovementStageStep, Stage, StageEntity, StopStageStep,
    },
    data::*,
    destructible::components::Destructible,
    enemy::components::Enemy,
    events::{NextStepEvent, StageClearedEvent, StageGameOverEvent},
    player::components::Player,
    resources::{StageActionTimer, StageProgress, StageStepSpawner, StageTime},
    StageProgressState,
};
use crate::{
    components::{DespawnMark, Music},
    game::GameProgressState,
    globals::{mark_for_despawn_by_component_query, DEBUG_STAGESTEP},
    plugins::movement::linear::components::{
        LinearMovementBundle, LinearTargetReached, TargetingPositionX, TargetingPositionY,
    },
    systems::{audio::VolumeSettings, camera::CameraPos, spawn::make_music_bundle},
    GBInput,
};
use bevy::{audio::PlaybackMode, prelude::*};
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::{
    prelude::{PxAssets, PxSubPosition},
    sprite::PxSprite,
};

pub fn toggle_game(
    gb_input: Res<ActionState<GBInput>>,
    state: Res<State<GameProgressState>>,
    mut next_state: ResMut<NextState<GameProgressState>>,
) {
    if gb_input.just_pressed(GBInput::Start) {
        if state.get().to_owned() == GameProgressState::Running {
            next_state.set(GameProgressState::Paused);
            info!("Game Paused.");
        } else {
            next_state.set(GameProgressState::Running);
            info!("Game Running.");
        }
    }
}

pub fn spawn_current_stage_bundle(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut state: ResMut<NextState<GameProgressState>>,
    stage_data: Res<StageData>,
) {
    commands
        .spawn((Stage, Name::new("Stage")))
        .with_children(|parent| {
            let background_bundle =
                make_background_bundle(&mut assets_sprite, stage_data.background_path.clone());
            parent.spawn(background_bundle);

            let skybox_bundle = make_skybox_bundle(&mut assets_sprite, stage_data.skybox.clone());
            parent.spawn(skybox_bundle);
        });

    state.set(GameProgressState::Running);
}

// TODO combine the two and use just_finished and StageTime
// TODO should be using StageTime instead of Time
pub fn tick_stage_step_timer(mut timer: ResMut<StageActionTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

pub fn check_stage_step_timer(
    timer: Res<StageActionTimer>,
    mut event_writer: EventWriter<NextStepEvent>,
) {
    if timer.timer.finished() {
        event_writer.send(NextStepEvent {});
    }
}

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
            if let Ok(entity) = stage_query.get_single() {
                commands.entity(entity).insert(DespawnMark);

                // TODO
                // commands.spawn(make_stage_cleared_bundle());
            }

            next_state.set(StageProgressState::Cleared);
        }
        _ => {}
    }
}

pub fn check_staged_cleared(
    mut event_writer: EventWriter<StageClearedEvent>,
    stage_progress: Res<StageProgress>,
    stage_data: Res<StageData>,
) {
    if stage_progress.index >= stage_data.steps.len() {
        event_writer.send(StageClearedEvent {});
    }
}

pub fn read_stage_cleared_trigger(
    mut commands: Commands,
    mut next_state: ResMut<NextState<StageProgressState>>,
    mut event_reader: EventReader<StageClearedEvent>,
    destructible_query: Query<Entity, With<Destructible>>,
    enemy_query: Query<Entity, With<Enemy>>,
    music_query: Query<Entity, With<Music>>,
    object_query: Query<Entity, With<Object>>,
    player_query: Query<Entity, With<Player>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    for _ in event_reader.iter() {
        mark_for_despawn_by_component_query(&mut commands, &destructible_query);
        mark_for_despawn_by_component_query(&mut commands, &enemy_query);
        mark_for_despawn_by_component_query(&mut commands, &music_query);
        mark_for_despawn_by_component_query(&mut commands, &object_query);
        mark_for_despawn_by_component_query(&mut commands, &player_query);

        let music_bundle = make_music_bundle(
            &asset_server,
            &volume_settings,
            "audio/music/intro.ogg".to_string(),
            PlaybackMode::Despawn,
        );

        commands.spawn((music_bundle, StageEntity));

        next_state.set(StageProgressState::Cleared);
    }
}

pub fn check_stage_game_over(
    mut event_writer: EventWriter<StageGameOverEvent>,
    player_query: Query<&Player, Added<Dead>>,
) {
    if let Ok(_) = player_query.get_single() {
        event_writer.send(StageGameOverEvent {});
    }
}

pub fn read_stage_game_over_trigger(
    mut commands: Commands,
    mut next_state: ResMut<NextState<StageProgressState>>,
    mut event_reader: EventReader<StageGameOverEvent>,
    destructible_query: Query<Entity, With<Destructible>>,
    enemy_query: Query<Entity, With<Enemy>>,
    music_query: Query<Entity, With<Music>>,
    object_query: Query<Entity, With<Object>>,
    player_query: Query<Entity, With<Player>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    for _ in event_reader.iter() {
        mark_for_despawn_by_component_query(&mut commands, &destructible_query);
        mark_for_despawn_by_component_query(&mut commands, &enemy_query);
        mark_for_despawn_by_component_query(&mut commands, &music_query);
        mark_for_despawn_by_component_query(&mut commands, &object_query);
        mark_for_despawn_by_component_query(&mut commands, &player_query);

        let music_bundle = make_music_bundle(
            &asset_server,
            &volume_settings,
            "audio/music/game_over.ogg".to_string(),
            PlaybackMode::Despawn,
        );
        commands.spawn((music_bundle, StageEntity));

        next_state.set(StageProgressState::GameOver);
    }
}

pub fn read_step_trigger(
    mut commands: Commands,
    mut progress: ResMut<StageProgress>,
    query: Query<Entity, (With<Stage>, Without<CurrentStageStep>)>,
    stage_data: Res<StageData>,
    stage_time: Res<StageTime>,
) {
    if let Ok(entity) = query.get_single() {
        if let Some(action) = stage_data.steps.get(progress.index) {
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(CurrentStageStep {
                started: stage_time.elapsed,
            });

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
            // TODO review progress
            progress.index += 1;
        }
    }
}

pub fn initialise_cinematic_step(
    mut game_state_next_state: ResMut<NextState<GameProgressState>>,
    query: Query<(Entity, &CinematicStageStep), (With<Stage>, Added<CinematicStageStep>)>,
) {
    if let Ok((_, _)) = query.get_single() {
        // game_state_next_state.set(GameState::Cutscene);
    }
}

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
    )) = query.get_single()
    {
        for (entity, position) in camera_query.iter() {
            let direction = coordinates.clone() - position.0;
            let speed = direction.normalize() * base_speed.clone() * GAME_BASE_SPEED;

            commands
                .entity(entity)
                .insert(LinearMovementBundle::<StageTime, TargetingPositionX>::new(
                    position.x.clone(),
                    coordinates.x.clone(),
                    speed.x,
                ))
                .insert(LinearMovementBundle::<StageTime, TargetingPositionY>::new(
                    position.y.clone(),
                    coordinates.y.clone(),
                    speed.y,
                ))
                .insert(StageStepSpawner::new(spawns.clone()));

            if let Some(floor_depths) = floor_depths {
                spawn_floor_depths(&mut commands, floor_depths);
            }
        }
    }
}

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
    )) = query.get_single()
    {
        commands
            .entity(entity)
            .insert(StageStepSpawner::new(spawns.clone()));

        if let Some(floor_depths) = floor_depths {
            spawn_floor_depths(&mut commands, &floor_depths);
        }
    }
}
pub fn check_movement_step_reached(
    mut event_writer: EventWriter<NextStepEvent>,
    query: Query<(
        With<MovementStageStep>,
        Added<LinearTargetReached<StageTime, TargetingPositionX>>,
    )>,
) {
    for _ in query.iter() {
        // TODO review cleanup of PositionY
        event_writer.send(NextStepEvent {})
    }
}

pub fn check_stop_step_finished_by_duration(
    mut commands: Commands,
    mut event_writer: EventWriter<NextStepEvent>,
    query: Query<(Entity, &StopStageStep, &CurrentStageStep), With<Stage>>,
    stage_time: Res<StageTime>,
) {
    for (entity, step, current_step) in query.iter() {
        if step
            .max_duration
            .map(|max_duration| current_step.started + max_duration <= stage_time.elapsed)
            .unwrap_or(false)
        {
            event_writer.send(NextStepEvent {});
        }
    }
}

pub fn update_cinematic_step(
    mut commands: Commands,
    mut event_writer: EventWriter<NextStepEvent>,
    query: Query<(Entity, &CinematicStageStep), With<Stage>>,
) {
    for (entity, _) in query.iter() {}
}

pub fn cleanup_cinematic_step(
    mut commands: Commands,
    mut event_reader: EventReader<NextStepEvent>,
    query: Query<(Entity, &CinematicStageStep), With<Stage>>,
) {
    for _ in event_reader.iter() {
        for (entity, _) in query.iter() {
            commands
                .entity(entity)
                .remove::<CinematicStageStep>()
                .remove::<CurrentStageStep>();
        }
    }
}

pub fn cleanup_movement_step(
    mut commands: Commands,
    mut event_reader: EventReader<NextStepEvent>,
    query: Query<(Entity, &MovementStageStep), With<Stage>>,
) {
    for _ in event_reader.iter() {
        // Cleanup logic
        for (entity, _) in query.iter() {
            commands
                .entity(entity)
                .remove::<MovementStageStep>()
                .remove::<StageStepSpawner>()
                .remove::<CurrentStageStep>();
        }
    }
}

pub fn cleanup_stop_step(
    mut commands: Commands,
    mut event_reader: EventReader<NextStepEvent>,

    query: Query<(Entity, &StopStageStep), With<Stage>>,
) {
    for _ in event_reader.iter() {
        // Cleanup logic
        for (entity, _) in query.iter() {
            commands
                .entity(entity)
                .remove::<StopStageStep>()
                .remove::<StageStepSpawner>()
                .remove::<CurrentStageStep>();
        }
    }
}
