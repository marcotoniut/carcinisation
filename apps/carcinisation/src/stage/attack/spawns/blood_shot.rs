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
    linear::components::{TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildBundle},
    structs::{Constructor, Magnitude},
};
use seldom_pixel::prelude::{PxSprite, PxSubPosition};

fn spawn_blood_shot_tween_child<P>(
    commands: &mut Commands,
    bundle: TweenChildBundle<StageTimeDomain, P>,
    axis_name: &'static str,
) where
    P: Constructor<f32> + Component + Magnitude,
{
    commands.spawn((bundle, BloodShotTween, Name::new(axis_name)));
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

/// Marker component for blood shot tween children.
#[derive(Component, Clone, Debug)]
pub struct BloodShotTween;

#[derive(Bundle)]
pub struct BloodShotBundle {
    pub enemy_attack_origin_position: EnemyAttackOriginPosition,
    pub enemy_attack_origin_depth: EnemyAttackOriginDepth,
    pub enemy_hovering_attack_type: EnemyHoveringAttackType,
    pub depth: Depth,
    pub inflicts_damage: InflictsDamage,
    pub position: PxSubPosition,
    pub targeting_value_x: TargetingValueX,
    pub targeting_value_y: TargetingValueY,
    pub targeting_value_z: TargetingValueZ,
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
        targeting_value_x: current_pos.x.into(),
        targeting_value_y: current_pos.y.into(),
        targeting_value_z: depth.to_f32().into(),
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

    // Spawn tween children for X
    spawn_blood_shot_tween_child(
        commands,
        TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
            blood_shot_entity,
            current_pos.x,
            far_target.x,
            speed.x,
        ),
        "Blood Shot Tween X",
    );

    // Spawn tween children for Y
    spawn_blood_shot_tween_child(
        commands,
        TweenChildBundle::<StageTimeDomain, TargetingValueY>::new(
            blood_shot_entity,
            current_pos.y,
            far_target.y,
            speed.y,
        ),
        "Blood Shot Tween Y",
    );

    // Spawn tween children for Z (toward player depth)
    spawn_blood_shot_tween_child(
        commands,
        TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
            blood_shot_entity,
            depth.to_f32(),
            PLAYER_DEPTH.to_f32(),
            BLOOD_SHOT_ATTACK_DEPTH_SPEED,
        ),
        "Blood Shot Tween Z",
    );
}
