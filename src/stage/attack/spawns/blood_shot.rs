use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxPosition, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    plugins::movement::linear::components::{
        LinearMovementBundle, LinearSpeed, TargetingPositionX, TargetingPositionY,
        TargetingPositionZ,
    },
    stage::{
        attack::{
            components::{
                bundles::make_hovering_attack_animation_bundle, EnemyAttack,
                EnemyHoveringAttackType,
            },
            data::blood_shot::{
                BLOOD_SHOT_ATTACK_DAMAGE, BLOOD_SHOT_ATTACK_DEPTH_SPEED,
                BLOOD_SHOT_ATTACK_LINE_SPEED,
            },
        },
        components::{
            damage::InflictsDamage,
            interactive::{Health, Hittable},
            placement::Depth,
        },
        enemy::components::*,
        player::components::PLAYER_DEPTH,
        resources::StageTime,
    },
};

pub fn spawn_blood_shot_attack(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    stage_time: &Res<StageTime>,
    target_pos: Vec2,
    current_pos: Vec2,
    depth: &Depth,
) {
    let attack_type = EnemyHoveringAttackType::BloodShot;

    let animation_bundle =
        make_hovering_attack_animation_bundle(assets_sprite, &attack_type, depth.clone());

    let mut attacking = EnemyMosquitoAttacking {
        attack: Some(EnemyMosquitoAttack::Ranged),
        last_attack_started: stage_time.elapsed,
    };

    attacking.attack = attacking.attack.clone();
    attacking.last_attack_started = attacking.last_attack_started.clone();

    let direction = target_pos - current_pos;
    let speed = direction.normalize() * BLOOD_SHOT_ATTACK_LINE_SPEED;

    let movement_bundle = (
        TargetingPositionX::new(current_pos.x),
        LinearSpeed::<StageTime, TargetingPositionX>::new(speed.x),
        TargetingPositionY::new(current_pos.y),
        LinearSpeed::<StageTime, TargetingPositionY>::new(speed.y),
        LinearMovementBundle::<StageTime, TargetingPositionZ>::new(
            depth.0.clone() as f32,
            PLAYER_DEPTH + 1.,
            BLOOD_SHOT_ATTACK_DEPTH_SPEED,
        ),
    );

    commands
        .spawn((
            Name::new(format!("Attack - {}", attack_type.get_name())),
            EnemyAttack,
            EnemyHoveringAttackType::BloodShot,
            // PursueTargetPosition::<StageTime, PxSubPosition>::new(target_pos),
            depth.clone(),
            InflictsDamage(BLOOD_SHOT_ATTACK_DAMAGE),
            PxSubPosition(current_pos),
            Hittable,
            Health(1),
        ))
        .insert(movement_bundle)
        .insert(animation_bundle);
}
