//! Registers frequently-inspected types for the Bevy editor/debug tooling.

use bevy::prelude::*;
use cween::linear::components::{TargetingValueX, TargetingValueY, TargetingValueZ};
use cween::structs::TweenDirection;

#[cfg(debug_assertions)]
use crate::debug::{
    DebugComposedDamageProbe, DebugComposedDamageProbeRequest, DebugComposedDamageProbeResult,
    DebugGodMode,
};
#[cfg(all(debug_assertions, feature = "gallery"))]
use crate::gallery::resources::{GalleryCharacter, GalleryState};
use crate::{
    components::GBColor,
    cutscene::data::{CutsceneAnimationSpawn, CutsceneAnimationsSpawn, TargetMovement},
    layer::Layer,
    stage::{
        attack::components::{EnemyAttackOriginDepth, EnemyAttackOriginPosition},
        components::{
            CinematicStageStep, CurrentStageStep, StageElapsedStarted, StopStageStep,
            TweenStageStep,
            damage::{DamageFlicker, InflictsDamage},
            interactive::{ColliderData, Health},
            placement::{Depth, Floor, Speed},
        },
        data::{
            ContainerSpawn, EnemyDropSpawn, EnemySpawn, ObjectSpawn, ObjectType, PickupDropSpawn,
            PickupSpawn, PickupType, SkyboxData, StageSpawn, StageStep,
        },
        destructible::{components::DestructibleType, data::DestructibleSpawn},
        enemy::{
            components::{
                CircleAround, LinearTween,
                behavior::{EnemyBehaviors, EnemyCurrentBehavior},
            },
            composed::{
                ComposedAnimationState, ComposedCollisionState, ComposedHealthPools,
                ComposedPartStates, ComposedResolvedParts, PartGameplayState, PartHitBlinkState,
                ResolvedCollisionVolume, ResolvedPartCollision, ResolvedPartState,
            },
            data::steps::{
                AttackEnemyStep, CircleAroundEnemyStep, EnemyStep, IdleEnemyStep, JumpEnemyStep,
                LinearTweenEnemyStep,
            },
            entity::EnemyType,
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
    transitions::data::TransitionRequest,
};

/// Registers types needed for reflection-based debugging/inspection.
pub fn register_types(app: &mut App) {
    #[cfg(debug_assertions)]
    app.register_type::<DebugComposedDamageProbe>()
        .register_type::<DebugComposedDamageProbeRequest>()
        .register_type::<DebugComposedDamageProbeResult>()
        .register_type::<DebugGodMode>();

    app.register_type::<AttackEnemyStep>()
        .register_type::<CameraShake>()
        .register_type::<CinematicStageStep>()
        .register_type::<CircleAround>()
        .register_type::<CircleAroundEnemyStep>()
        .register_type::<ComposedAnimationState>()
        .register_type::<ComposedCollisionState>()
        .register_type::<ComposedHealthPools>()
        .register_type::<ComposedPartStates>()
        .register_type::<ComposedResolvedParts>()
        .register_type::<ColliderData>()
        .register_type::<ContainerSpawn>()
        .register_type::<CurrentEnemyMosquitoStep>()
        .register_type::<CurrentStageStep>()
        .register_type::<CutsceneAnimationSpawn>()
        .register_type::<CutsceneAnimationsSpawn>()
        .register_type::<DamageFlicker>()
        .register_type::<Depth>()
        .register_type::<DestructibleSpawn>()
        .register_type::<DestructibleType>()
        .register_type::<EnemyAttackOriginDepth>()
        .register_type::<EnemyAttackOriginPosition>()
        .register_type::<EnemyBehaviors>()
        .register_type::<EnemyCurrentBehavior>()
        .register_type::<EnemyDropSpawn>()
        .register_type::<EnemySpawn>()
        .register_type::<EnemyStep>()
        .register_type::<EnemyType>()
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
        .register_type::<IdleEnemyStep>()
        .register_type::<InflictsDamage>()
        .register_type::<JumpEnemyStep>()
        .register_type::<GBColor>()
        .register_type::<Layer>()
        .register_type::<LinearTween>()
        .register_type::<LinearTweenEnemyStep>()
        .register_type::<ObjectSpawn>()
        .register_type::<ObjectType>()
        .register_type::<PartHitBlinkState>()
        .register_type::<PartGameplayState>()
        .register_type::<PickupDropSpawn>()
        .register_type::<PickupSpawn>()
        .register_type::<PickupType>()
        .register_type::<PlayerAttack>()
        .register_type::<ResolvedCollisionVolume>()
        .register_type::<ResolvedPartCollision>()
        .register_type::<ResolvedPartState>()
        .register_type::<SkyboxData>()
        .register_type::<Speed>()
        .register_type::<StageSpawn>()
        .register_type::<StageStep>()
        .register_type::<StopStageStep>()
        .register_type::<StageElapsedStarted>()
        .register_type::<TargetingValueX>()
        .register_type::<TargetingValueY>()
        .register_type::<TargetingValueZ>()
        .register_type::<TargetMovement>()
        .register_type::<TransitionRequest>()
        .register_type::<TweenDirection>()
        .register_type::<TweenStageStep>();

    #[cfg(debug_assertions)]
    app.register_type::<crate::stage::attack::components::EnemyAttackDebugPosition>();

    #[cfg(feature = "gallery")]
    {
        app.register_type::<GalleryCharacter>()
            .register_type::<GalleryState>();
    }
}
