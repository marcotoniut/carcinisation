pub mod spawn;

use std::{ops::Sub, time::Duration};

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::{
    prelude::{PxAssets, PxCamera, PxSubPosition},
    sprite::PxSprite,
};

use crate::{
    globals::DEBUG_STAGESTEP, resource::park::STAGE_PARK_DATA, systems::camera::CameraPos, GBInput,
};

use self::spawn::{spawn_enemy, spawn_object};

use super::{
    bundles::*,
    components::Stage,
    data::*,
    events::{StageSpawnTrigger, StageStepTrigger},
    resources::{StageActionTimer, StageProgress},
    GameState, StageState,
};

pub fn pause_game(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Paused);
}

pub fn resume_game(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Running);
}

pub fn toggle_game(
    gb_input_query: Query<&ActionState<GBInput>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let gb_input = gb_input_query.single();
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
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    let camera_pos = camera_query.get_single().unwrap();

    let stage_data = STAGE_PARK_DATA.clone();

    for spawn in &stage_data.spawns {
        match spawn {
            StageSpawn::Destructible(_) => {}
            StageSpawn::Enemy(spawn) => {
                spawn_enemy(&mut commands, &camera_pos, spawn);
            }
            StageSpawn::Object(spawn) => {
                spawn_object(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Pickup(_) => {}
        }
    }

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
        .spawn((Stage {}, Name::new("Stage")))
        .with_children(|parent| {
            let background_bundle =
                make_background_bundle(&mut assets_sprite, stage.background.clone());
            parent.spawn(background_bundle);

            let skybox_bundle = make_skybox_bundle(&mut assets_sprite, stage.skybox.clone());
            parent.spawn(skybox_bundle);
        });

    state.set(GameState::Running);
}

// TODO Probably can do without this now
pub fn increment_elapsed(mut progress: ResMut<StageProgress>, time: Res<Time>) {
    let delta = time.delta_seconds();
    progress.elapsed += delta;
    progress.step_elapsed += delta;
    progress.spawn_step_elapsed += delta;
}

pub fn tick_stage_step_timer(mut timer: ResMut<StageActionTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

pub fn check_stage_step_timer(
    timer: Res<StageActionTimer>,
    mut event_writer: EventWriter<StageStepTrigger>,
) {
    if timer.timer.finished() {
        event_writer.send(StageStepTrigger {});
    }
}

pub fn update_stage(
    mut commands: Commands,
    state: Res<State<StageState>>,
    stage_query: Query<(Entity, &Stage)>,
    mut next_state: ResMut<NextState<StageState>>,
    mut camera_pos_query: Query<&mut PxSubPosition, With<CameraPos>>,
    mut camera: ResMut<PxCamera>,
    mut spawn_event_writer: EventWriter<StageSpawnTrigger>,
    mut step_event_writer: EventWriter<StageStepTrigger>,
    mut stage_progress: ResMut<StageProgress>,
    time: Res<Time>,
    mut stage_data_raw: Res<StageRawData>,
) {
    match state.to_owned() {
        StageState::Initial => {
            next_state.set(StageState::Running);
        }
        StageState::Running => {
            if let stage = &stage_data_raw.stage_data {
                if let Some(action) = stage.steps.get(stage_progress.step) {
                    if DEBUG_STAGESTEP {
                        let curr_action = match action {
                            StageStep::Movement { .. } => "movement".to_string(),
                            StageStep::Stop { .. } => "stop".to_string(),
                            StageStep::Cinematic { .. } => "cinematic".to_string(),
                        };

                        info!("curr action: {}", curr_action);
                    }
                    
                    let mut spawnsVal = None;
                    match action {
                        StageStep::Movement {
                            coordinates,
                            base_speed,
                            spawns,
                            ..
                        } => {
                            let mut camera_pos = camera_pos_query.single_mut();
                            let direction = coordinates.sub(camera_pos.0).normalize();

                            **camera_pos += time.delta_seconds() * base_speed * direction;

                            if direction.x.signum() != (coordinates.x - camera_pos.0.x).signum() {
                                if DEBUG_STAGESTEP {
                                    warn!(
                                        "================>>>> movement complete? {}",
                                        direction.x.to_string()
                                    );
                                }
                                *camera_pos = PxSubPosition(coordinates.clone());
                                step_event_writer.send(StageStepTrigger {});
                            }

                            **camera = camera_pos.round().as_ivec2();

                            spawnsVal = Some(spawns);
                        },
                        StageStep::Stop {
                            resume_conditions,
                            max_duration,
                            spawns,
                            ..
                        } => {
                            // TODO
                            if let Some(duration) = max_duration {
                            } else {
                                // DEBUG

                                if DEBUG_STAGESTEP {
                                    let mut duration = 0.;
                                    if max_duration.is_some() {
                                        duration = max_duration.unwrap();
                                    }
                                    warn!(
                                        "================>>>> stop complete? {}",
                                        duration.to_string()
                                    );
                                }
                                step_event_writer.send(StageStepTrigger {});
                            }
                            spawnsVal = Some(spawns);
                        },
                        StageStep::Cinematic { .. } => {},
                    }

                    if let Some(spawns) = spawnsVal {
                        let mut cloned_spawns = spawns.clone();

                        let mut i = 0;
                        while let Some(spawn) = cloned_spawns.first() {
                            if (stage_progress.spawn_step <= i) {
                                let elapsed = stage_progress.spawn_step_elapsed - spawn.get_elapsed();
                                if 0. <= elapsed {
                                    stage_progress.spawn_step_elapsed -= spawn.get_elapsed();

                                    spawn_event_writer.send(StageSpawnTrigger {
                                        spawn: spawn.clone(),
                                    });
                                } else {
                                    break;
                                }
                            }
                            cloned_spawns.remove(0);
                            i += 1;
                        }
                        stage_progress.spawn_step = i;
                    }
                }
            }
        }
        StageState::Clear => {
            if let Ok((entity, _)) = stage_query.get_single() {
                commands.entity(entity).despawn_descendants();

                // TODO
                // commands.spawn(make_stage_cleared_bundle());
            }

            next_state.set(StageState::Cleared);
        }
        StageState::Cleared => {}
    }
}

pub fn check_staged_cleared(
    mut next_state: ResMut<NextState<StageState>>,
    stage_progress: Res<StageProgress>,
    mut stage_data_raw: Res<StageRawData>,
) {
    if let stage = &stage_data_raw.stage_data {
        if stage_progress.step >= stage.steps.len() {
            next_state.set(StageState::Clear);
        }
    }
}

pub fn read_stage_step_trigger(
    mut event_reader: EventReader<StageStepTrigger>,
    mut stage_progress: ResMut<StageProgress>,
    mut stage_action_timer: ResMut<StageActionTimer>,
    mut stage_data_raw: Res<StageRawData>,
) {
    for _ in event_reader.iter() {
        stage_progress.step += 1;
        stage_progress.step_elapsed = 0.;
        stage_progress.spawn_step = 0;
        stage_progress.spawn_step_elapsed = 0.;

        if let stage = &stage_data_raw.stage_data {
            if let Some(action) = stage.steps.get(stage_progress.step) {
                stage_action_timer.timer.pause();
                match action {
                    StageStep::Movement { .. } => {
                        stage_action_timer.timer.reset();
                    }
                    StageStep::Stop { max_duration, .. } => {
                        if let Some(duration) = max_duration {
                            stage_action_timer.timer.reset();
                            stage_action_timer
                                .timer
                                .set_duration(Duration::from_secs_f32(duration.clone()));
                            stage_action_timer.timer.unpause();
                        }
                    }
                    StageStep::Cinematic { max_duration, .. }  => {
                        if let Some(duration) = max_duration {
                            stage_action_timer.timer.reset();
                            stage_action_timer
                                .timer
                                .set_duration(Duration::from_secs_f32(duration.clone()));
                            stage_action_timer.timer.unpause();
                        }
                    }
                    StageStep::Cinematic { max_duration, .. }  => {
                        if let Some(duration) = max_duration {
                            stage_action_timer.timer.reset();
                            stage_action_timer
                                .timer
                                .set_duration(Duration::from_secs_f32(duration.clone()));
                            stage_action_timer.timer.unpause();
                        }
                    }
                }
            }
        }
    }
}
