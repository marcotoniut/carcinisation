use crate::{
    plugins::movement::linear::components::{
        LinearMovementBundle, LinearSpeed, TargetingPositionX, TargetingPositionY,
        TargetingPositionZ,
    },
    stage::{
        attack::{
            components::{
                bundles::make_hovering_attack_animation_bundle, EnemyAttack,
                EnemyAttackOriginDepth, EnemyAttackOriginPosition, EnemyHoveringAttackType,
            },
            data::blood_shot::{
                BLOOD_SHOT_ATTACK_DAMAGE, BLOOD_SHOT_ATTACK_DEPTH_SPEED,
                BLOOD_SHOT_ATTACK_LINE_SPEED, BLOOD_SHOT_ATTACK_RANDOMNESS,
            },
        },
        components::{
            damage::InflictsDamage,
            interactive::{Health, Hittable},
            placement::Depth,
        },
        enemy::mosquito::entity::{EnemyMosquitoAttack, EnemyMosquitoAttacking},
        player::components::PLAYER_DEPTH,
        resources::StageTime,
    },
};
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAssets, PxSubPosition},
    sprite::PxSprite,
};

#[derive(Bundle)]
pub struct BloodShotDefaultBundle {
    pub name: Name,
    pub enemy_attack: EnemyAttack,
    pub health: Health,
    pub hittable: Hittable,
}

impl Default for BloodShotDefaultBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Attack<BloodShot>"),
            enemy_attack: EnemyAttack,
            health: Health(1),
            hittable: Hittable,
        }
    }
}

#[derive(Bundle)]
pub struct BloodShotMovementBundle {
    // TODO shouldn't be using "TargetingPosition" for this, since it isn't really targeting
    targeting_position_x: TargetingPositionX,
    linear_speed_x: LinearSpeed<StageTime, TargetingPositionX>,
    targeting_position_y: TargetingPositionY,
    linear_speed_y: LinearSpeed<StageTime, TargetingPositionY>,
    linear_movement_z: LinearMovementBundle<StageTime, TargetingPositionZ>,
}

impl BloodShotMovementBundle {
    pub fn new(depth: &Depth, current_pos: Vec2, target_pos: Vec2) -> Self {
        let direction = target_pos - current_pos;
        let speed = direction.normalize_or_zero() * BLOOD_SHOT_ATTACK_LINE_SPEED;

        Self {
            targeting_position_x: TargetingPositionX::new(current_pos.x),
            linear_speed_x: LinearSpeed::<StageTime, TargetingPositionX>::new(speed.x),
            targeting_position_y: TargetingPositionY::new(current_pos.y),
            linear_speed_y: LinearSpeed::<StageTime, TargetingPositionY>::new(speed.y),
            linear_movement_z: LinearMovementBundle::<StageTime, TargetingPositionZ>::new(
                depth.to_f32(),
                PLAYER_DEPTH.to_f32(),
                BLOOD_SHOT_ATTACK_DEPTH_SPEED,
            ),
        }
    }
}

#[derive(Bundle)]
pub struct BloodShotBundle {
    pub enemy_attack_origin_position: EnemyAttackOriginPosition,
    pub enemy_attack_origin_depth: EnemyAttackOriginDepth,
    pub enemy_hovering_attack_type: EnemyHoveringAttackType,
    pub depth: Depth,
    pub inflicts_damage: InflictsDamage,
    pub position: PxSubPosition,
    pub movement: BloodShotMovementBundle,
    pub default: BloodShotDefaultBundle,
}

pub fn spawn_blood_shot_attack(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    stage_time: &Res<StageTime>,
    target_pos: Vec2,
    current_pos: Vec2,
    depth: &Depth,
) {
    let attack_type = EnemyHoveringAttackType::BloodShot;
    // TODO should this account for player speed?
    let target_pos = target_pos
        + Vec2::new(
            (1. - rand::random::<f32>()) * BLOOD_SHOT_ATTACK_RANDOMNESS,
            (1. - rand::random::<f32>()) * BLOOD_SHOT_ATTACK_RANDOMNESS,
        );

    let (sprite, animation, collider_data) =
        make_hovering_attack_animation_bundle(assets_sprite, &attack_type, depth.clone());

    let mut attacking = EnemyMosquitoAttacking {
        attack: Some(EnemyMosquitoAttack::Ranged),
        last_attack_started: stage_time.elapsed,
    };

    attacking.attack = attacking.attack.clone();
    attacking.last_attack_started = attacking.last_attack_started.clone();

    let mut entity_commands = commands.spawn(BloodShotBundle {
        enemy_attack_origin_position: EnemyAttackOriginPosition(current_pos),
        enemy_attack_origin_depth: EnemyAttackOriginDepth(depth.clone()),
        enemy_hovering_attack_type: EnemyHoveringAttackType::BloodShot,
        depth: depth.clone(),
        inflicts_damage: InflictsDamage(BLOOD_SHOT_ATTACK_DAMAGE),
        position: PxSubPosition(current_pos),
        movement: BloodShotMovementBundle::new(depth, current_pos, target_pos),
        default: Default::default(),
    });

    entity_commands.insert((sprite, animation));

    if !collider_data.0.is_empty() {
        entity_commands.insert(collider_data);
    }
}
