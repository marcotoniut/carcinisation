use bevy::{prelude::*, time::Fixed};
use bevy_framepace::Limiter;
use carapace::prelude::{CxOverlayCamera, WorldPos};
use carcinisation_base::game::CameraPos;

/// @system Caps the frame limiter to the target Game Boy refresh rate.
pub fn set_framespace(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(59.727_500_569_606);
}

/// @system Aligns the fixed timestep with the target framerate.
pub fn set_fixed_timestep(mut fixed_time: ResMut<Time<Fixed>>) {
    fixed_time.set_timestep_hz(59.727_500_569_606);
}

/// @system Spawns the 2D camera, an overlay camera for gizmos, and a
/// `CameraPos` tracking entity.
///
/// The overlay camera (order 1, `CxOverlayCamera`) renders Bevy gizmos on
/// top of the pixel-art post-process output.  Without it, gizmos draw
/// beneath the fullscreen `CxPlugin` quad and are invisible.
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        CxOverlayCamera,
    ));
    commands.spawn((WorldPos::default(), CameraPos));
}
