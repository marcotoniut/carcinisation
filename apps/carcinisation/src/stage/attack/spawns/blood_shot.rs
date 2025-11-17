use crate::pixel::PxAssets;
use crate::stage::{
    attack::{
        components::{
            bundles::make_hovering_attack_animation_bundle, EnemyAttack, EnemyAttackOriginDepth,
            EnemyAttackOriginPosition, EnemyHoveringAttackType,
        },
        data::blood_shot::{
            BLOOD_SHOT_ATTACK_DAMAGE, BLOOD_SHOT_ATTACK_DEPTH_SPEED, BLOOD_SHOT_ATTACK_LINE_SPEED,
            BLOOD_SHOT_ATTACK_RANDOMNESS,
        },
    },
    components::{
        damage::InflictsDamage,
        interactive::{Health, Hittable},
        placement::Depth,
    },
    player::components::PLAYER_DEPTH,
    resources::StageTimeDomain,
};
use bevy::prelude::*;
use cween::{
    linear::components::{
        MovementChildBundle, TargetingPositionX, TargetingPositionY, TargetingPositionZ,
    },
    structs::{Constructor, Magnitude},
};
use seldom_pixel::prelude::{PxSprite, PxSubPosition};

fn spawn_blood_shot_movement_child<P>(
    commands: &mut Commands,
    bundle: MovementChildBundle<StageTimeDomain, P>,
    axis_name: &'static str,
) where
    P: Constructor<f32> + Component + Magnitude,
{
    commands.spawn((bundle, BloodShotMovement, Name::new(axis_name)));
}

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

/// Marker component for blood shot movement children.
#[derive(Component, Clone, Debug)]
pub struct BloodShotMovement;

#[derive(Bundle)]
pub struct BloodShotBundle {
    pub enemy_attack_origin_position: EnemyAttackOriginPosition,
    pub enemy_attack_origin_depth: EnemyAttackOriginDepth,
    pub enemy_hovering_attack_type: EnemyHoveringAttackType,
    pub depth: Depth,
    pub inflicts_damage: InflictsDamage,
    pub position: PxSubPosition,
    pub targeting_position_x: TargetingPositionX,
    pub targeting_position_y: TargetingPositionY,
    pub targeting_position_z: TargetingPositionZ,
    pub default: BloodShotDefaultBundle,
}

pub fn spawn_blood_shot_attack(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    _stage_time: &Res<Time<StageTimeDomain>>,
    target_pos: Vec2,
    current_pos: Vec2,
    depth: &Depth,
) {
    let attack_type = EnemyHoveringAttackType::BloodShot;
    // TODO should this account for player speed/direction?
    let target_pos = target_pos
        + Vec2::new(
            (1. - rand::random::<f32>()) * BLOOD_SHOT_ATTACK_RANDOMNESS,
            (1. - rand::random::<f32>()) * BLOOD_SHOT_ATTACK_RANDOMNESS,
        );

    let (sprite, animation, collider_data) =
        make_hovering_attack_animation_bundle(assets_sprite, &attack_type, *depth);

    let direction = target_pos - current_pos;
    let speed = direction.normalize_or_zero() * BLOOD_SHOT_ATTACK_LINE_SPEED;

    let mut entity_commands = commands.spawn(BloodShotBundle {
        enemy_attack_origin_position: EnemyAttackOriginPosition(current_pos),
        enemy_attack_origin_depth: EnemyAttackOriginDepth(*depth),
        enemy_hovering_attack_type: EnemyHoveringAttackType::BloodShot,
        depth: *depth,
        inflicts_damage: InflictsDamage(BLOOD_SHOT_ATTACK_DAMAGE),
        position: PxSubPosition(current_pos),
        targeting_position_x: current_pos.x.into(),
        targeting_position_y: current_pos.y.into(),
        targeting_position_z: depth.to_f32().into(),
        default: default(),
    });

    entity_commands.insert((sprite, animation));

    if !collider_data.0.is_empty() {
        entity_commands.insert(collider_data);
    }

    let blood_shot_entity = entity_commands.id();

    // Blood shots don't have a fixed target - they travel in a straight line
    // Use a very large target position to approximate infinite travel
    let far_target = current_pos + direction.normalize_or_zero() * 1000.0;

    // Spawn movement children for X
    spawn_blood_shot_movement_child(
        commands,
        MovementChildBundle::<StageTimeDomain, TargetingPositionX>::new(
            blood_shot_entity,
            current_pos.x,
            far_target.x,
            speed.x,
        ),
        "Blood Shot Movement X",
    );

    // Spawn movement children for Y
    spawn_blood_shot_movement_child(
        commands,
        MovementChildBundle::<StageTimeDomain, TargetingPositionY>::new(
            blood_shot_entity,
            current_pos.y,
            far_target.y,
            speed.y,
        ),
        "Blood Shot Movement Y",
    );

    // Spawn movement children for Z (toward player depth)
    spawn_blood_shot_movement_child(
        commands,
        MovementChildBundle::<StageTimeDomain, TargetingPositionZ>::new(
            blood_shot_entity,
            depth.to_f32(),
            PLAYER_DEPTH.to_f32(),
            BLOOD_SHOT_ATTACK_DEPTH_SPEED,
        ),
        "Blood Shot Movement Z",
    );
}
