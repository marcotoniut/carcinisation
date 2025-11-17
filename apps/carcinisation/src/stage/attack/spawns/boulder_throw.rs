use crate::pixel::PxAssets;
use crate::stage::{
    attack::{
        components::{
            bundles::make_hovering_attack_animation_bundle, EnemyAttack, EnemyHoveringAttackType,
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
    player::components::PLAYER_DEPTH,
    resources::StageTimeDomain,
};
use bevy::prelude::*;
use cween::{
    linear::components::{
        TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildAcceleratedBundle,
        TweenChildBundle,
    },
    structs::{Constructor, Magnitude},
};
use seldom_pixel::prelude::{PxSprite, PxSubPosition};

fn spawn_boulder_throw_tween_child<P>(
    commands: &mut Commands,
    bundle: TweenChildBundle<StageTimeDomain, P>,
    label: &'static str,
) where
    P: Constructor<f32> + Component + Magnitude,
{
    commands.spawn((bundle, BoulderThrowTween, Name::new(label)));
}

fn spawn_boulder_throw_tween_child_accelerated<P>(
    commands: &mut Commands,
    bundle: TweenChildAcceleratedBundle<StageTimeDomain, P>,
    label: &'static str,
) where
    P: Constructor<f32> + Component + Magnitude,
{
    commands.spawn((bundle, BoulderThrowTween, Name::new(label)));
}

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

/// Marker component for boulder throw tween children.
#[derive(Component, Clone, Debug)]
pub struct BoulderThrowTween;

#[derive(Bundle)]
pub struct BoulderThrowBundle {
    pub depth: Depth,
    pub inflicts_damage: InflictsDamage,
    pub position: PxSubPosition,
    pub targeting_value_x: TargetingValueX,
    pub targeting_value_y: TargetingValueY,
    pub targeting_value_z: TargetingValueZ,
    pub default: BoulderThrowDefaultBundle,
}

pub fn spawn_boulder_throw_attack(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    _stage_time: &Res<Time<StageTimeDomain>>,
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
        make_hovering_attack_animation_bundle(assets_sprite, &attack_type, *depth);

    let depth_f32 = depth.to_f32();
    let target_depth = PLAYER_DEPTH;

    let speed_z = BOULDER_THROW_ATTACK_DEPTH_SPEED;
    let t = (target_depth.to_f32() - depth.to_f32()) / speed_z;

    let d = target_pos - current_pos;

    let speed_x = d.x / t;

    // TODO: remember that boulder throws in outer space wouldn't have as much gravity, if any
    let value = d.y - 0.5 * BOULDER_THROW_ATTACK_LINE_Y_ACCELERATION * t.powi(2);
    let speed_y = if value / t >= 0.0 { value / t } else { 0.0 };

    let mut entity_commands = commands.spawn(BoulderThrowBundle {
        depth: *depth,
        inflicts_damage: InflictsDamage(BOULDER_THROW_ATTACK_DAMAGE),
        position: PxSubPosition(current_pos),
        targeting_value_x: current_pos.x.into(),
        targeting_value_y: current_pos.y.into(),
        targeting_value_z: depth_f32.into(),
        default: default(),
    });
    entity_commands.insert((sprite, animation));

    if !collider_data.0.is_empty() {
        entity_commands.insert(collider_data);
    }

    let boulder_entity = entity_commands.id();

    // Spawn tween children for X (constant speed)
    spawn_boulder_throw_tween_child(
        commands,
        TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
            boulder_entity,
            current_pos.x,
            target_pos.x,
            speed_x,
        ),
        "Boulder Throw Tween X",
    );

    // Spawn tween children for Y (accelerated - gravity)
    spawn_boulder_throw_tween_child_accelerated(
        commands,
        TweenChildAcceleratedBundle::<StageTimeDomain, TargetingValueY>::new(
            boulder_entity,
            current_pos.y,
            target_pos.y,
            speed_y,
            BOULDER_THROW_ATTACK_LINE_Y_ACCELERATION,
        ),
        "Boulder Throw Tween Y",
    );

    // Spawn tween children for Z (constant speed toward player depth)
    spawn_boulder_throw_tween_child(
        commands,
        TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
            boulder_entity,
            depth_f32,
            target_depth.to_f32(),
            BOULDER_THROW_ATTACK_DEPTH_SPEED,
        ),
        "Boulder Throw Tween Z",
    );
}
