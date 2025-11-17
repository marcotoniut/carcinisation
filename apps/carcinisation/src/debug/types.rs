//! Registers frequently-inspected types for the Bevy editor/debug tooling.

use bevy::prelude::*;
use cween::linear::components::{TargetingPositionX, TargetingPositionY, TargetingPositionZ};

use crate::{
    components::GBColor,
    cutscene::data::CutsceneAnimationsSpawn,
    layer::Layer,
    stage::{
        attack::components::{EnemyAttackOriginDepth, EnemyAttackOriginPosition},
        components::{
            damage::{DamageFlicker, InflictsDamage},
            interactive::{ColliderData, Health},
            placement::{Depth, Floor, RailPosition, Speed},
            CurrentStageStep, StageElapsedStarted,
        },
        enemy::{
            components::{
                behavior::{EnemyBehaviors, EnemyCurrentBehavior},
                *,
            },
            mosquito::entity::{
                CurrentEnemyMosquitoStep, EnemyMosquito, EnemyMosquitoAnimation,
                EnemyMosquitoAttack, EnemyMosquitoAttacking,
            },
            tardigrade::entity::{
                EnemyTardigrade, EnemyTardigradeAnimation, EnemyTardigradeAttacking,
            },
        },
        pickup::components::HealthRecovery,
        player::components::{CameraShake, PlayerAttack},
    },
};

/// Registers types needed for reflection-based debugging/inspection.
pub fn register_types(app: &mut App) {
    app.register_type::<CameraShake>()
        .register_type::<CircleAround>()
        .register_type::<ColliderData>()
        .register_type::<CurrentEnemyMosquitoStep>()
        .register_type::<CurrentStageStep>()
        .register_type::<CutsceneAnimationsSpawn>()
        .register_type::<DamageFlicker>()
        .register_type::<Depth>()
        .register_type::<EnemyAttackOriginDepth>()
        .register_type::<EnemyAttackOriginPosition>()
        .register_type::<EnemyBehaviors>()
        .register_type::<EnemyCurrentBehavior>()
        .register_type::<EnemyMosquito>()
        .register_type::<EnemyMosquitoAttacking>()
        .register_type::<EnemyMosquitoAttack>()
        .register_type::<EnemyMosquitoAnimation>()
        .register_type::<EnemyTardigrade>()
        .register_type::<EnemyTardigradeAnimation>()
        .register_type::<EnemyTardigradeAttacking>()
        .register_type::<Floor>()
        .register_type::<Health>()
        .register_type::<HealthRecovery>()
        .register_type::<InflictsDamage>()
        .register_type::<GBColor>()
        .register_type::<Layer>()
        .register_type::<LinearMovement>()
        .register_type::<PlayerAttack>()
        .register_type::<RailPosition>()
        .register_type::<Speed>()
        .register_type::<StageElapsedStarted>()
        .register_type::<TargetingPositionX>()
        .register_type::<TargetingPositionY>()
        .register_type::<TargetingPositionZ>();
}
