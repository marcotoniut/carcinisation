//! Registers frequently-inspected types for the Bevy editor/debug tooling.

use bevy::prelude::*;
use cween::linear::components::{TargetingValueX, TargetingValueY, TargetingValueZ};

#[cfg(debug_assertions)]
use crate::debug::{
    DebugComposedDamageProbe, DebugComposedDamageProbeRequest, DebugComposedDamageProbeResult,
};
#[cfg(all(debug_assertions, feature = "gallery"))]
use crate::gallery::resources::{GalleryCharacter, GalleryState};
use crate::{
    components::GBColor,
    cutscene::data::CutsceneAnimationsSpawn,
    layer::Layer,
    stage::{
        attack::components::{EnemyAttackOriginDepth, EnemyAttackOriginPosition},
        components::{
            CurrentStageStep, StageElapsedStarted,
            damage::{DamageFlicker, InflictsDamage},
            interactive::{ColliderData, Health},
            placement::{Depth, Floor, RailPosition, Speed},
        },
        enemy::{
            components::{
                CircleAround, LinearTween,
                behavior::{EnemyBehaviors, EnemyCurrentBehavior},
            },
            composed::{
                ComposedAnimationState, ComposedCollisionState, ComposedHealthPools,
                ComposedPartStates, ComposedResolvedParts, PartGameplayState,
                ResolvedCollisionVolume, ResolvedPartCollision, ResolvedPartState,
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
    #[cfg(debug_assertions)]
    app.register_type::<DebugComposedDamageProbe>()
        .register_type::<DebugComposedDamageProbeRequest>()
        .register_type::<DebugComposedDamageProbeResult>();

    app.register_type::<CameraShake>()
        .register_type::<CircleAround>()
        .register_type::<ComposedAnimationState>()
        .register_type::<ComposedCollisionState>()
        .register_type::<ComposedHealthPools>()
        .register_type::<ComposedPartStates>()
        .register_type::<ComposedResolvedParts>()
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
        .register_type::<LinearTween>()
        .register_type::<PartGameplayState>()
        .register_type::<PlayerAttack>()
        .register_type::<RailPosition>()
        .register_type::<ResolvedCollisionVolume>()
        .register_type::<ResolvedPartCollision>()
        .register_type::<ResolvedPartState>()
        .register_type::<Speed>()
        .register_type::<StageElapsedStarted>()
        .register_type::<TargetingValueX>()
        .register_type::<TargetingValueY>()
        .register_type::<TargetingValueZ>();

    #[cfg(feature = "gallery")]
    {
        app.register_type::<GalleryCharacter>()
            .register_type::<GalleryState>();
    }
}
