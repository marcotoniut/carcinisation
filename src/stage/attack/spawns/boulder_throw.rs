use crate::{
    plugins::movement::linear::components::{
        LinearAcceleration, LinearMovementBundle, LinearSpeed, TargetingPositionX,
        TargetingPositionY, TargetingPositionZ,
    },
    stage::{
        attack::{
            components::{
                bundles::make_hovering_attack_animation_bundle, EnemyAttack,
                EnemyHoveringAttackType,
            },
            data::boulder_throw::{
                BOULDER_THROW_ATTACK_DAMAGE, BOULDER_THROW_ATTACK_DEPTH_SPEED,
                BOULDER_THROW_ATTACK_LINE_Y_ACCELERATION, BOULDER_THROW_ATTACK_RANDOMNESS,
            },
        },
        components::{
            damage::InflictsDamage,
            interactive::{Flickerer, Health, Hittable},
            placement::Depth,
        },
        enemy::tardigrade::entity::EnemyTardigradeAttacking,
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
pub struct BoulderThrowDefaultBundle {
    pub enemy_attack: EnemyAttack,
    pub enemy_attack_type: EnemyHoveringAttackType,
    pub flickerer: Flickerer,
    pub health: Health,
    pub hittable: Hittable,
    pub name: Name,
}

impl Default for BoulderThrowDefaultBundle {
    fn default() -> Self {
        Self {
            enemy_attack: EnemyAttack,
            flickerer: Flickerer,
            health: Health(100),
            hittable: Hittable,
            enemy_attack_type: EnemyHoveringAttackType::BoulderThrow,
            name: Name::new("Attack<BoulderShot>"),
        }
    }
}

#[derive(Bundle)]
pub struct BoulderThrowMovementBundle {
    // TODO shouldn't be using "TargetingPosition" for this, since it isn't really targeting
    pub targeting_position_x: TargetingPositionX,
    pub linear_speed_x: LinearSpeed<StageTime, TargetingPositionX>,
    pub targeting_position_y: TargetingPositionY,
    pub linear_speed_y: LinearSpeed<StageTime, TargetingPositionY>,
    pub linear_acceleration_y: LinearAcceleration<StageTime, TargetingPositionY>,
    pub linear_movement_z: LinearMovementBundle<StageTime, TargetingPositionZ>,
}

#[derive(Bundle)]
pub struct BoulderThrowBundle {
    pub depth: Depth,
    pub inflicts_damage: InflictsDamage,
    pub position: PxSubPosition,
    pub movement: BoulderThrowMovementBundle,
    pub default: BoulderThrowDefaultBundle,
}

impl BoulderThrowMovementBundle {
    pub fn new(depth: &Depth, current_pos: Vec2, target_pos: Vec2) -> Self {
        let depth_f32 = depth.to_f32();
        let target_depth = PLAYER_DEPTH;

        let speed_z = BOULDER_THROW_ATTACK_DEPTH_SPEED;
        let t = (target_depth.to_f32() - depth.to_f32()) / speed_z;

        let d = target_pos - current_pos;

        let speed_x = d.x / t;

        // TODO: remember that boulder throws in outter space wouldn't have as much gravity, if any
        let value = d.y - 0.5 * BOULDER_THROW_ATTACK_LINE_Y_ACCELERATION * t.powi(2);
        let speed_y = if value / t >= 0.0 { value / t } else { 0.0 };

        Self {
            targeting_position_x: current_pos.x.into(),
            linear_speed_x: LinearSpeed::<StageTime, TargetingPositionX>::new(speed_x),
            targeting_position_y: current_pos.y.into(),
            linear_speed_y: LinearSpeed::<StageTime, TargetingPositionY>::new(speed_y),
            linear_acceleration_y: LinearAcceleration::<StageTime, TargetingPositionY>::new(
                BOULDER_THROW_ATTACK_LINE_Y_ACCELERATION,
            ),
            linear_movement_z: LinearMovementBundle::<StageTime, TargetingPositionZ>::new(
                depth_f32,
                target_depth.to_f32(),
                BOULDER_THROW_ATTACK_DEPTH_SPEED,
            ),
        }
    }
}

pub fn spawn_boulder_throw_attack(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    stage_time: &Res<StageTime>,
    target_pos: Vec2,
    current_pos: Vec2,
    depth: &Depth,
) {
    let attack_type = EnemyHoveringAttackType::BoulderThrow;
    let target_pos = target_pos
        + Vec2::new(
            (1. - rand::random::<f32>()) * BOULDER_THROW_ATTACK_RANDOMNESS,
            (1. - rand::random::<f32>()) * BOULDER_THROW_ATTACK_RANDOMNESS,
        );

    let (sprite, animation, collider_data) =
        make_hovering_attack_animation_bundle(assets_sprite, &attack_type, depth.clone());

    let mut attacking = EnemyTardigradeAttacking {
        attack: true,
        last_attack_started: stage_time.elapsed,
    };

    attacking.attack = attacking.attack.clone();
    attacking.last_attack_started = attacking.last_attack_started.clone();

    let mut entity_commands = commands.spawn(BoulderThrowBundle {
        depth: depth.clone(),
        inflicts_damage: InflictsDamage(BOULDER_THROW_ATTACK_DAMAGE),
        position: PxSubPosition(current_pos),
        movement: BoulderThrowMovementBundle::new(depth, current_pos, target_pos),
        default: default(),
    });
    entity_commands.insert((sprite, animation));

    if !collider_data.0.is_empty() {
        entity_commands.insert(collider_data);
    }
}
