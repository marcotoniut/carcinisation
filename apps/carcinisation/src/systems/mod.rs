pub mod camera;
pub mod movement;
pub mod setup;
pub mod spawn;

use crate::{
    components::{
        AudioSystemType, DelayedDespawnOnCxAnimationFinished, DespawnAfterDelay, DespawnMark,
        VolumeSettings,
    },
    game::messages::GameStartupEvent,
    splash::messages::SplashStartupEvent,
};
use bevy::prelude::*;
use carapace::prelude::CxAnimationFinished;

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

/// @system Syncs music source volumes with the global volume settings.
pub fn update_music_volume(
    mut source_settings: Query<(&mut PlaybackSettings, &AudioSystemType)>,
    volume_settings: Res<VolumeSettings>,
) {
    for (mut music_source_settings, audio_system_type) in &mut source_settings {
        if matches!(audio_system_type, AudioSystemType::SFX) {
            music_source_settings.volume = volume_settings.music;
        }
    }
}

/// @system Syncs SFX source volumes with the global volume settings.
pub fn update_sfx_volume(
    mut source_settings: Query<&mut PlaybackSettings>,
    volume_settings: Res<VolumeSettings>,
) {
    for mut sfx_source_settings in &mut source_settings {
        sfx_source_settings.volume = volume_settings.sfx;
    }
}

/// @system Attaches a `DespawnAfterDelay` once a pixel animation finishes.
pub fn delay_despawn<D: Default + Send + Sync + 'static>(
    mut commands: Commands,
    mut query: Query<
        (Entity, &DelayedDespawnOnCxAnimationFinished),
        (With<CxAnimationFinished>, Without<DespawnAfterDelay>),
    >,
    time: Res<Time<D>>,
) {
    for (entity, delayed) in &mut query.iter_mut() {
        let elapsed = time.elapsed();
        commands.entity(entity).insert(DespawnAfterDelay {
            elapsed,
            duration: delayed.0,
        });
    }
}

/// @system Marks entities for despawn once their delay timer expires.
pub fn check_despawn_after_delay<D: Default + Send + Sync + 'static>(
    mut commands: Commands,
    mut query: Query<(Entity, &DespawnAfterDelay)>,
    time: Res<Time<D>>,
) {
    for (entity, despawn_after_delay) in &mut query.iter_mut() {
        if despawn_after_delay.elapsed + despawn_after_delay.duration <= time.elapsed() {
            commands.entity(entity).insert(DespawnMark);
        }
    }
}

/// @system DEBUG — fires `GameStartupEvent` immediately.
pub fn debug_trigger_game_startup(mut commands: Commands) {
    commands.trigger(GameStartupEvent);
}

/// @system Routes boot flow through the splash screen or directly to menu/game.
pub fn on_post_startup(mut commands: Commands, dev_flags: Res<crate::resources::DevFlags>) {
    if dev_flags.skip_splash {
        info!("CARCINISATION_SKIP_SPLASH: skipping splash screen");
        crate::splash::continue_after_splash(&mut commands, &dev_flags);
    } else {
        commands.trigger(SplashStartupEvent);
    }
}
