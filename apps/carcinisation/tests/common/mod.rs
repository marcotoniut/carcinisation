#![allow(dead_code)]

use std::time::Duration;

use bevy::prelude::*;
use carcinisation::stage::enemy::mosquito::systems::clear_finished_mosquito_attacks;
use carcinisation::stage::{
    components::placement::Depth,
    enemy::{
        components::behavior::EnemyCurrentBehavior,
        composed::ComposedAnimationState,
        data::{
            mosquiton::ACTION_IDLE_FLY,
            steps::{EnemyStep, IdleEnemyStep},
        },
        mosquito::entity::{EnemyMosquito, EnemyMosquitoAttack, EnemyMosquitoAttacking},
        mosquiton::{entity::EnemyMosquiton, systems::assign_mosquiton_animation},
    },
    resources::StageTimeDomain,
};
use carcinisation_base::direction::SpriteDirection;

/// Build a minimal app with attack-timing systems (clear + assign).
pub fn build_attack_timing_app() -> App {
    let mut app = App::new();
    app.insert_resource(Time::<StageTimeDomain>::default());
    app.add_systems(
        Update,
        (clear_finished_mosquito_attacks, assign_mosquiton_animation).chain(),
    );
    app
}

/// Advance stage time and tick one frame.
pub fn advance_stage(app: &mut App, duration: Duration) {
    app.world_mut()
        .resource_mut::<Time<StageTimeDomain>>()
        .advance_by(duration);
    app.update();
}

/// Spawn a legacy mosquito (no `Mosquiton` marker, no `ComposedAnimationState`).
pub fn spawn_legacy_mosquito(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            EnemyMosquito,
            EnemyCurrentBehavior {
                started: Duration::ZERO,
                behavior: EnemyStep::Idle(IdleEnemyStep { duration: 99999.0 }),
            },
            EnemyMosquitoAttacking {
                attack: Some(EnemyMosquitoAttack::Ranged),
                last_attack_started: Duration::ZERO,
            },
            Depth::Three,
        ))
        .id()
}

/// Spawn a composed mosquiton (with `Mosquiton` marker + `ComposedAnimationState`).
pub fn spawn_composed_mosquiton(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            EnemyMosquiton,
            EnemyMosquito,
            EnemyCurrentBehavior {
                started: Duration::ZERO,
                behavior: EnemyStep::Idle(IdleEnemyStep { duration: 99999.0 }),
            },
            EnemyMosquitoAttacking {
                attack: Some(EnemyMosquitoAttack::Ranged),
                last_attack_started: Duration::ZERO,
            },
            ComposedAnimationState::new(SpriteDirection::Front.tag_name(ACTION_IDLE_FLY)),
            Depth::Three,
        ))
        .id()
}
