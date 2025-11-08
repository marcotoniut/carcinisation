pub mod camera;
pub mod movement;
pub mod setup;
pub mod spawn;

use crate::components::{AudioSystemType, VolumeSettings};
use crate::game::events::GameStartupTrigger;
use crate::{
    components::{DelayedDespawnOnPxAnimationFinished, DespawnAfterDelay, DespawnMark},
    core::time::ElapsedTime,
    main_menu::MainMenuPlugin,
};
use activable::activate;
use bevy::prelude::*;
use seldom_pixel::prelude::PxAnimationFinished;

/*
 * DEBUG
 */
// pub fn input_exit_game(
//     gb_input_query: Query<&ActionState<GBInput>>,
//     mut exit: ResMut<Events<AppExit>>,
// ) {
//     let gb_input = gb_input_query.single();
//     if gb_input.just_pressed(&GBInput::DExit) {
//         exit.write(AppExit);
//     }
// }

// /**
//  * DEBUG
//  */
// pub fn transition_to_game_state(
//     gb_input_query: Query<&ActionState<GBInput>>,
//     app_state: Res<State<AppState>>,
//     mut next_state: ResMut<NextState<AppState>>,
// ) {
//     let gb_input = gb_input_query.single();
//     if gb_input.just_pressed(&GBInput::DToGame) {
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
//     if gb_input.just_pressed(&GBInput::DToMainMenu) {
//         if app_state.get().to_owned() != AppState::MainMenu {
//             // commands.insert_resource(NextState(Some(AppState::MainMenu)));
//             next_state.set(AppState::MainMenu);
//             info!("Entered AppState::MainMenu");
//         }
//     }
// }

pub fn update_music_volume(
    mut source_settings: Query<(&mut PlaybackSettings, &AudioSystemType)>,
    volume_settings: Res<VolumeSettings>,
) {
    for (mut music_source_settings, audio_system_type) in source_settings.iter_mut() {
        if matches!(audio_system_type, AudioSystemType::SFX) {
            music_source_settings.volume = volume_settings.music;
        }
    }
}

pub fn update_sfx_volume(
    mut source_settings: Query<&mut PlaybackSettings>,
    volume_settings: Res<VolumeSettings>,
) {
    for mut sfx_source_settings in &mut source_settings {
        sfx_source_settings.volume = volume_settings.sfx;
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

pub fn debug_trigger_game_startup(mut commands: Commands) {
    commands.trigger(GameStartupTrigger);
}

pub fn on_post_startup(mut commands: Commands) {
    activate::<MainMenuPlugin>(&mut commands);
}
