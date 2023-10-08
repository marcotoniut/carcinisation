pub mod audio;
pub mod camera;
pub mod spawn;
pub mod state;

use self::{audio::VolumeSettings, camera::CameraPos};
use crate::game::events::GameStartupEvent;
use crate::GBInput;
use crate::{
    audio::AudioSystemType,
    components::{DelayedDespawnOnPxAnimationFinished, DespawnAfterDelay, DespawnMark},
    core::time::ElapsedTime,
    game::events::GameOver,
};
use bevy::{audio::Volume, prelude::*};
use bevy_framepace::Limiter;
use leafwing_input_manager::{
    prelude::{ActionState, InputMap},
    InputManagerBundle,
};
use seldom_pixel::prelude::{PxAnimationFinished, PxSubPosition};

/**
 * DEBUG
 */
// pub fn input_exit_game(
//     gb_input_query: Query<&ActionState<GBInput>>,
//     mut exit: ResMut<Events<AppExit>>,
// ) {
//     let gb_input = gb_input_query.single();
//     if gb_input.just_pressed(GBInput::DExit) {
//         exit.send(AppExit);
//     }
// }

// pub fn input_snd_menu(
//     gb_input_query: Query<&ActionState<GBInput>>,
//     app_state: Res<State<AppState>>,
//     mut next_app_state: ResMut<NextState<AppState>>,
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,

//     mut next_game_state: ResMut<NextState<GameState>>,

//     state: Res<State<StageProgressState>>,
//     mut next_stage_state: ResMut<NextState<StageProgressState>>,
//     mut camera_pos_query: Query<&mut PxSubPosition, With<CameraPos>>,
//     mut camera: ResMut<PxCamera>,
//     time: Res<Time>,
// ) {
//     let gb_input = gb_input_query.single();
//     if gb_input.just_pressed(GBInput::Select) {
//         info!("open pause menu");

//         if app_state.get().to_owned() != AppState::MainMenu {
//             // commands.insert_resource(NextState(Some(AppState::MainMenu)));
//             info!("Entered AppState::MainMenu");

//             let stage_data_handle = StageDataHandle(asset_server.load("stages/settings.yaml"));
//             commands.insert_resource(stage_data_handle);

//             next_app_state.set(AppState::MainMenu);
//             next_stage_state.set(StageProgressState::Initial);
//             next_game_state.set(GameState::Loading);
//         } else if app_state.get().to_owned() != AppState::Game {
//             // commands.insert_resource(NextState(Some(AppState::MainMenu)));

//             info!("Entered AppState::Game");

//             info!("TODO Initialise game");

//             next_app_state.set(AppState::Game);
//             next_stage_state.set(StageProgressState::Running);
//             next_game_state.set(GameState::Loading);
//         }
//     }
// }

pub fn handle_game_over(mut game_over_event_reader: EventReader<GameOver>) {
    for game_over in game_over_event_reader.iter() {
        info!("Your final score: {}", game_over.score);
    }
}

pub fn set_framespace(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(59.727500569606);
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((PxSubPosition::default(), CameraPos));
}

pub fn spawn_gb_input(mut commands: Commands) {
    commands.spawn(InputManagerBundle::<GBInput> {
        action_state: ActionState::default(),
        input_map: InputMap::new([
            (KeyCode::Left, GBInput::Left),
            (KeyCode::Up, GBInput::Up),
            (KeyCode::Right, GBInput::Right),
            (KeyCode::Down, GBInput::Down),
            (KeyCode::Z, GBInput::B),
            (KeyCode::X, GBInput::A),
            (KeyCode::Return, GBInput::Start),
            (KeyCode::ShiftRight, GBInput::Select),
            // DEBUG
            (KeyCode::I, GBInput::DToGame),
            (KeyCode::Back, GBInput::DToMainMenu),
            (KeyCode::Escape, GBInput::DExit),
            (KeyCode::A, GBInput::DLeft),
            (KeyCode::W, GBInput::DUp),
            (KeyCode::D, GBInput::DRight),
            (KeyCode::S, GBInput::DDown),
        ]),
    });
}

// /**
//  * DEBUG
//  */
// pub fn transition_to_game_state(
//     gb_input_query: Query<&ActionState<GBInput>>,
//     app_state: Res<State<AppState>>,
//     mut next_state: ResMut<NextState<AppState>>,
// ) {
//     let gb_input = gb_input_query.single();
//     if gb_input.just_pressed(GBInput::DToGame) {
//         if app_state.get().to_owned() != AppState::Game {
//             next_state.set(AppState::Game);
//             info!("Entered AppState::Game");
//         }
//     }
// }

// /**
//  * DEBUG
//  */
// pub fn transition_to_main_menu_state(
//     gb_input_query: Query<&ActionState<GBInput>>,
//     app_state: Res<State<AppState>>,
//     mut next_state: ResMut<NextState<AppState>>,
// ) {
//     let gb_input = gb_input_query.single();
//     if gb_input.just_pressed(GBInput::DToMainMenu) {
//         if app_state.get().to_owned() != AppState::MainMenu {
//             // commands.insert_resource(NextState(Some(AppState::MainMenu)));
//             next_state.set(AppState::MainMenu);
//             info!("Entered AppState::MainMenu");
//         }
//     }
// }

pub fn update_master_volume(volume_settings: Res<VolumeSettings>) {
    let master_volume = volume_settings.0;
    GlobalVolume::new(master_volume);
}

pub fn update_music_volume(
    mut source_settings: Query<(&mut PlaybackSettings, &AudioSystemType)>,
    volume_settings: Res<VolumeSettings>,
) {
    let music_volume = volume_settings.1;
    for (mut music_source_settings, audio_system_type) in source_settings.iter_mut() {
        if matches!(audio_system_type, AudioSystemType::SFX) {
            music_source_settings.volume = Volume::new_relative(music_volume);
        }
    }
}

pub fn update_sfx_volume(
    mut source_settings: Query<&mut PlaybackSettings>,
    volume_settings: Res<VolumeSettings>,
) {
    let sfx_volume = volume_settings.2;
    for mut sfx_source_settings in &mut source_settings {
        sfx_source_settings.volume = Volume::new_relative(sfx_volume);
    }
}

pub fn delay_despawn<T: ElapsedTime + Resource>(
    mut commands: Commands,
    mut query: Query<
        (Entity, &DelayedDespawnOnPxAnimationFinished),
        (With<PxAnimationFinished>, Without<DespawnAfterDelay>),
    >,
    time: Res<T>,
) {
    for (entity, delayed) in &mut query.iter_mut() {
        let elapsed = time.elapsed();
        commands.entity(entity).insert(DespawnAfterDelay {
            elapsed,
            duration: delayed.0,
        });
    }
}

pub fn check_despawn_after_delay<T: ElapsedTime + Resource>(
    mut commands: Commands,
    mut query: Query<(Entity, &DespawnAfterDelay)>,
    time: Res<T>,
) {
    for (entity, despawn_after_delay) in &mut query.iter_mut() {
        if despawn_after_delay.elapsed + despawn_after_delay.duration <= time.elapsed() {
            commands.entity(entity).insert(DespawnMark);
        }
    }
}

pub fn trigger_game_startup(mut event_writer: EventWriter<GameStartupEvent>) {
    event_writer.send(GameStartupEvent);
}
