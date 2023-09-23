use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::stage::{
    components::{Dead, Health},
    pickup::components::HealthRecovery,
    player::components::{Player, PLAYER_MAX_HEALTH},
    score::components::Score,
};

pub fn pickup_health(
    mut commands: Commands,
    mut score: ResMut<Score>,
    query: Query<(Entity, &HealthRecovery, &PxSubPosition), With<Dead>>,
    mut player_query: Query<&mut Health, With<Player>>,
) {
    if let Ok(mut health) = player_query.get_single_mut() {
        for (entity, recovery, position) in query.iter() {
            commands.entity(entity).despawn();

            health.0 += recovery.0;
            if health.0 > PLAYER_MAX_HEALTH {
                health.0 = PLAYER_MAX_HEALTH;
            }

            score.value -= recovery.score_deduction();

            // TODO spawn animated pickup
        }
    }
}
