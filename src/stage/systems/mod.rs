pub mod spawn;

use std::{ops::Sub, time::Duration};

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::{
    prelude::{PxAssets, PxCamera, PxSubPosition},
    sprite::PxSprite,
};

use crate::{systems::camera::CameraPos, GBInput};

use super::{
    bundles::*,
    components::Stage,
    data::*,
    events::StageActionTrigger,
    resources::*,
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
            println!("Game Paused.");
        } else {
            next_state.set(GameState::Running);
            println!("Game Running.");
        }
    }
}

pub fn setup_stage(mut commands: Commands, asset_server: Res<AssetServer>) {
    let stage_data_handle = StageDataHandle(asset_server.load("stages/asteroid.yaml"));
    commands.insert_resource(stage_data_handle);
}

pub fn spawn_current_stage_bundle(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    data_handle: Res<StageDataHandle>,
    data: Res<Assets<StageData>>,
    mut state: ResMut<NextState<GameState>>,
) {
    if let Some(stage) = data.get(&data_handle.0.clone()) {
        commands
            .spawn((Stage {}, Name::new("Stage")))
            .with_children(|parent| {
                let background_bundle =
                    make_background_bundle(&mut assets_sprite, stage.background.clone());
                parent.spawn(background_bundle);

                if let Some(skybox_path) = stage.skybox.clone() {
                    let skybox_bundle = make_skybox_bundle(&mut assets_sprite, skybox_path);
                    parent.spawn(skybox_bundle);
                }
            });

        state.set(GameState::Running);
    } else {
        // Error
    }
}

// TODO Probably can do without this now
pub fn increment_elapsed(mut progress: ResMut<StageProgress>, time: Res<Time>) {
    progress.elapsed += time.delta_seconds();
    progress.step_elapsed += time.delta_seconds();
}

pub fn tick_stage_stop_timer(mut timer: ResMut<StageActionTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

pub fn check_stage_stop_timer(
    timer: Res<StageActionTimer>,
    mut event_writer: EventWriter<StageActionTrigger>,
) {
    if timer.timer.finished() {
        event_writer.send(StageActionTrigger {});
    }
}

pub fn update_stage(
    mut commands: Commands,
    state: Res<State<StageState>>,
    stage_query: Query<(Entity, &Stage)>,
    mut next_state: ResMut<NextState<StageState>>,
    mut camera_pos_query: Query<&mut PxSubPosition, With<CameraPos>>,
    mut camera: ResMut<PxCamera>,
    mut event_writer: EventWriter<StageActionTrigger>,
    game_progress: Res<StageProgress>,
    time: Res<Time>,
    data: Res<Assets<StageData>>,
    data_handle: Res<StageDataHandle>,
) {
    match state.to_owned() {
        StageState::Initial => {
            next_state.set(StageState::Running);
        }
        StageState::Running => {
            if let Some(stage) = data.get(&data_handle.0.clone()) {
                if let Some(action) = stage.steps.get(game_progress.step) {
                    match action {
                        StageStep::Movement {
                            coordinates,
                            base_speed,
                            ..
                        } => {
                            let mut camera_pos = camera_pos_query.single_mut();
                            let direction = coordinates.sub(camera_pos.0).normalize();

                            **camera_pos += time.delta_seconds() * base_speed * direction;

                            if direction.x.signum() != (coordinates.x - camera_pos.0.x).signum() {
                                *camera_pos = PxSubPosition(coordinates.clone());
                                event_writer.send(StageActionTrigger {});
                            }

                            **camera = camera_pos.round().as_ivec2();
                        }
                        StageStep::Stop {
                            resume_conditions,
                            max_duration,
                            ..
                        } => {
                            // TODO
                            if let Some(duration) = max_duration {
                            } else {
                                // DEBUG
                                event_writer.send(StageActionTrigger {});
                            }
                        }
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
    game_progress: Res<StageProgress>,
    data: Res<Assets<StageData>>,
    data_handle: Res<StageDataHandle>,
) {
    if let Some(stage) = data.get(&data_handle.0.clone()) {
        if game_progress.step >= stage.steps.len() {
            next_state.set(StageState::Clear);
        }
    }
}

pub fn read_stage_action_trigger(
    mut event_reader: EventReader<StageActionTrigger>,
    mut game_progress: ResMut<StageProgress>,
    data: Res<Assets<StageData>>,
    data_handle: Res<StageDataHandle>,
    mut stage_action_timer: ResMut<StageActionTimer>,
    // mut stage_timer: ResMut<StageTimer>,
    time: Res<Time>,
) {
    for _ in event_reader.iter() {
        game_progress.step += 1;
        game_progress.step_elapsed = 0.;

        if let Some(stage) = data.get(&data_handle.0.clone()) {
            if let Some(action) = stage.steps.get(game_progress.step) {
                stage_action_timer.timer.pause();
                match action {
                    StageStep::Movement { .. } => {}
                    StageStep::Stop { max_duration, .. } => {
                        if let Some(duration) = max_duration {
                            stage_action_timer.timer.reset();
                            stage_action_timer
                                .timer
                                .set_duration(Duration::from_secs(duration.clone()));
                            stage_action_timer.timer.unpause();
                        }
                    }
                }
            }
        }
    }
}
