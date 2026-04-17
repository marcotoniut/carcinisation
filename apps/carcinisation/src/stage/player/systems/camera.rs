use crate::{
    stage::{
        player::{components::CameraShake, messages::CameraShakeEvent},
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use carapace::prelude::PxSubPosition;

/// Minimum intensity below which the shake is removed.
const SHAKE_THRESHOLD: f32 = 0.3;

/// @system Applies decaying camera shake each frame.
///
/// Each frame: undo previous offset → compute new random offset at current
/// intensity → apply → decay intensity. When intensity drops below threshold,
/// remove component and restore clean position.
///
/// Registered at stage scope (not player scope) because the camera outlives
/// the player — the decay system must keep running after death so the
/// component self-removes. When stage time is frozen (Death/GameOver),
/// `dt = 0` halts decay, so `on_death` and `handle_stage_restart` clear
/// the component explicitly.
pub fn camera_shake(
    mut commands: Commands,
    mut query: Query<(Entity, &mut CameraShake, &mut PxSubPosition)>,
    time: Res<Time<StageTimeDomain>>,
) {
    let dt = time.delta_secs();

    for (entity, mut shake, mut position) in &mut query {
        // Undo previous frame's offset.
        position.0 -= shake.current_offset;

        if shake.intensity < SHAKE_THRESHOLD {
            // Done shaking — clean up.
            shake.current_offset = Vec2::ZERO;
            commands.entity(entity).remove::<CameraShake>();
            continue;
        }

        // Random offset with guaranteed minimum displacement to avoid invisible shakes.
        let angle = rand::random::<f32>() * std::f32::consts::TAU;
        let magnitude = shake.intensity * (0.5 + 0.5 * rand::random::<f32>());
        let offset = Vec2::new(angle.cos() * magnitude, angle.sin() * magnitude);

        position.0 += offset;
        shake.current_offset = offset;

        // Exponential decay.
        shake.intensity *= (-shake.decay * dt).exp();
    }
}

/// @trigger Initiates or reinforces a camera shake on `CameraShakeEvent`.
///
/// If a shake is already active, the new intensity is added to the current
/// intensity rather than replacing it. This prevents hits from canceling
/// ongoing shakes and makes rapid hits compound visually.
pub fn on_camera_shake(
    _trigger: On<CameraShakeEvent>,
    mut commands: Commands,
    mut camera_query: Query<(Entity, Option<&mut CameraShake>), With<CameraPos>>,
) {
    const BASE_INTENSITY: f32 = 3.0;
    const DECAY_RATE: f32 = 12.0;

    if let Ok((entity, existing_shake)) = camera_query.single_mut() {
        if let Some(mut shake) = existing_shake {
            // Reinforce existing shake instead of replacing.
            shake.intensity += BASE_INTENSITY;
        } else {
            commands.entity(entity).insert(CameraShake {
                intensity: BASE_INTENSITY,
                decay: DECAY_RATE,
                current_offset: Vec2::ZERO,
            });
        }
    }
}
