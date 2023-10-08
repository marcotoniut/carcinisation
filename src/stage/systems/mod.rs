pub mod camera;
pub mod damage;
pub mod movement;
pub mod spawn;
pub mod state;

use self::spawn::{spawn_destructible, spawn_enemy, spawn_object, spawn_pickup};
use super::{
    bundles::*,
    components::{
        interactive::{Dead, Object},
        CinematicStageStep, CurrentStageStep, MovementStageStep, Stage, StopStageStep,
    },
    data::*,
    destructible::components::Destructible,
    enemy::components::Enemy,
    events::{NextStepEvent, StageClearedEvent, StageGameOverEvent},
    player::{components::Player, events::PlayerStartupEvent},
    resources::{StageActionTimer, StageProgress, StageStepSpawner, StageTime},
    GameState, StageProgressState,
};

use crate::{
    cinemachine::cinemachine::CinemachineScene,
    components::{DespawnMark, Music},
    globals::{mark_for_despawn_by_component_query, DEBUG_STAGESTEP},
    plugins::movement::linear::components::{
        LinearMovementBundle, LinearTargetReached, TargetingPositionX, TargetingPositionY,
    },
    resource::{asteroid::STAGE_ASTEROID_DATA, debug::STAGE_DEBUG_DATA, park::STAGE_PARK_DATA},
    systems::{audio::VolumeSettings, camera::CameraPos, spawn::spawn_music},
    GBInput,
};
use bevy::{audio::PlaybackMode, prelude::*};
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::{
    prelude::{PxAssets, PxSubPosition},
    sprite::PxSprite,
};

pub fn tick_stage_time(mut stage_time: ResMut<StageTime>, time: Res<Time>) {
    let delta = time.delta();
    stage_time.delta = delta;
    stage_time.elapsed += delta;
}

pub fn toggle_game(
    gb_input: Res<ActionState<GBInput>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if gb_input.just_pressed(GBInput::Start) {
        if state.get().to_owned() == GameState::Running {
            next_state.set(GameState::Paused);
            info!("Game Paused.");
        } else {
            next_state.set(GameState::Running);
            info!("Game Running.");
        }
    }
}

#[derive(Resource)]
pub struct StageRawData {
    stage_data: StageData,
}

pub fn setup_stage(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,

    mut event_writer: EventWriter<PlayerStartupEvent>,
    asset_server: Res<AssetServer>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    volume_settings: Res<VolumeSettings>,
) {
    event_writer.send(PlayerStartupEvent);

    let camera_pos = camera_query.get_single().unwrap();

    let stage_data = STAGE_DEBUG_DATA.clone();
    // let stage_data = STAGE_PARK_DATA.clone();

    for spawn in &stage_data.spawns {
        match spawn {
            StageSpawn::Destructible(spawn) => {
                spawn_destructible(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Enemy(spawn) => {
                spawn_enemy(&mut commands, camera_pos.0, spawn);
            }
            StageSpawn::Object(spawn) => {
                spawn_object(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Pickup(spawn) => {
                spawn_pickup(&mut commands, &mut assets_sprite, Vec2::ZERO, spawn);
            }
        }
    }

    spawn_music(
        &mut commands,
        &asset_server,
        &volume_settings,
        stage_data.music_path.clone(),
        PlaybackMode::Loop,
    );

    commands.insert_resource(StageRawData { stage_data });
}

pub fn spawn_current_stage_bundle(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut state: ResMut<NextState<GameState>>,
    stage_data_raw: Res<StageRawData>,
) {
    let stage = &stage_data_raw.stage_data;
    commands
        .spawn((Stage, Name::new("Stage")))
        .with_children(|parent| {
            let background_bundle =
                make_background_bundle(&mut assets_sprite, stage.background_path.clone());
            parent.spawn(background_bundle);

            let skybox_bundle = make_skybox_bundle(&mut assets_sprite, stage.skybox.clone());
            parent.spawn(skybox_bundle);
        });

    state.set(GameState::Running);
}

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
    stage_data_raw: Res<StageRawData>,
) {
    match state.to_owned() {
        StageProgressState::Initial => {
            next_state.set(StageProgressState::Running);
        }
        StageProgressState::Running => {
            let stage = &stage_data_raw.stage_data;
            if let Some(action) = stage.steps.get(stage_progress.step) {
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
    stage_data_raw: Res<StageRawData>,
) {
    let stage = &stage_data_raw.stage_data;
    if stage_progress.step >= stage.steps.len() {
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

        spawn_music(
            &mut commands,
            &asset_server,
            &volume_settings,
            "audio/music/intro.ogg".to_string(),
            PlaybackMode::Despawn,
        );

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

        spawn_music(
            &mut commands,
            &asset_server,
            &volume_settings,
            "audio/music/game_over.ogg".to_string(),
            PlaybackMode::Despawn,
        );

        next_state.set(StageProgressState::GameOver);
    }
}

pub fn read_stage_step_trigger(
    mut commands: Commands,
    mut stage_progress: ResMut<StageProgress>,
    query: Query<Entity, (With<Stage>, Without<CurrentStageStep>)>,
    stage_data_raw: Res<StageRawData>,
    stage_time: Res<StageTime>,
) {
    if let Ok(entity) = query.get_single() {
        stage_progress.step += 1;

        let stage = &stage_data_raw.stage_data;
        if let Some(action) = stage.steps.get(stage_progress.step) {
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
        }
    }
}

pub fn initialise_cinematic_step(
    mut game_state_next_state: ResMut<NextState<GameState>>,
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
        }
    }
}

pub fn initialise_stop_step(
    mut commands: Commands,
    query: Query<(Entity, &StopStageStep), (With<Stage>, Added<StopStageStep>)>,
) {
    if let Ok((entity, step)) = query.get_single() {
        commands
            .entity(entity)
            .insert(StageStepSpawner::new(step.spawns.clone()));
    }
}

pub fn update_cinematic_step(
    mut commands: Commands,
    mut event_writer: EventWriter<NextStepEvent>,
    query: Query<(Entity, &CinematicStageStep), With<Stage>>,
) {
    for (entity, _) in query.iter() {}
}

pub fn check_movement_step_reached(
    mut event_writer: EventWriter<NextStepEvent>,
    query: Query<
        (Entity, &MovementStageStep),
        Added<LinearTargetReached<StageTime, TargetingPositionX>>,
    >,
) {
    for (entity, _) in query.iter() {
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
