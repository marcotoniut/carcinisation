use crate::pixel::PxAssets;
use crate::stage::{
    attack::{
        components::{
            EnemyAttack, EnemyAttackOriginDepth, EnemyAttackOriginPosition,
            EnemyHoveringAttackType, bundles::make_hovering_attack_animation_bundle,
        },
        data::blood_shot::{
            BLOOD_SHOT_ATTACK_DAMAGE, BLOOD_SHOT_ATTACK_DEPTH_SPEED, BLOOD_SHOT_ATTACK_LINE_SPEED,
            BLOOD_SHOT_ATTACK_RANDOMNESS, BLOOD_SHOT_ATTACK_STARTUP_HOLD,
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
use carapace::prelude::{PxSprite, PxSubPosition};
use cween::{
    linear::components::{TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildBundle},
    structs::{Constructor, Magnitude},
};
use std::time::Duration;

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

/// Holds a freshly spawned blood shot at its authored cue origin briefly so
/// the first visible frame reads as emerging from the mouth before travel.
#[derive(Component, Clone, Debug)]
pub struct PendingBloodShotMotion {
    pub armed_at: Duration,
    pub far_target: Vec2,
    pub speed: Vec2,
}

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
    stage_time: &Res<Time<StageTimeDomain>>,
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

    #[cfg(debug_assertions)]
    entity_commands.insert(crate::stage::attack::components::EnemyAttackDebugPosition {
        current: current_pos,
        origin: current_pos,
    });

    entity_commands.insert((sprite, animation));

    if !collider_data.0.is_empty() {
        entity_commands.insert(collider_data);
    }

    // Blood shots don't have a fixed target - they travel in a straight line.
    // Use a very large target position to approximate infinite travel once the
    // short startup hold finishes.
    let far_target = current_pos + direction.normalize_or_zero() * 1000.0;

    entity_commands.insert(PendingBloodShotMotion {
        armed_at: stage_time.elapsed() + BLOOD_SHOT_ATTACK_STARTUP_HOLD,
        far_target,
        speed,
    });
}

pub fn arm_pending_blood_shot_motion(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    query: Query<(Entity, &PendingBloodShotMotion, &PxSubPosition, &Depth), With<EnemyAttack>>,
) {
    for (entity, pending, position, depth) in &query {
        if stage_time.elapsed() < pending.armed_at {
            continue;
        }

        spawn_blood_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
                entity,
                position.0.x,
                pending.far_target.x,
                pending.speed.x,
            ),
            "Blood Shot Tween X",
        );

        spawn_blood_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueY>::new(
                entity,
                position.0.y,
                pending.far_target.y,
                pending.speed.y,
            ),
            "Blood Shot Tween Y",
        );

        spawn_blood_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
                entity,
                depth.to_f32(),
                PLAYER_DEPTH.to_f32(),
                BLOOD_SHOT_ATTACK_DEPTH_SPEED,
            ),
            "Blood Shot Tween Z",
        );

        commands.entity(entity).remove::<PendingBloodShotMotion>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cween::linear::components::{
        TargetingValueX, TargetingValueY, TargetingValueZ, TweenChild,
    };
    use std::time::Duration;

    #[test]
    fn pending_blood_shot_motion_arms_only_after_hold() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.add_systems(Update, arm_pending_blood_shot_motion);

        let attack = app
            .world_mut()
            .spawn((
                EnemyAttack,
                PxSubPosition(Vec2::new(10.0, 20.0)),
                Depth::Three,
                PendingBloodShotMotion {
                    armed_at: Duration::from_millis(60),
                    far_target: Vec2::new(100.0, 120.0),
                    speed: Vec2::new(5.0, 6.0),
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(59));
        app.update();

        assert!(
            app.world()
                .entity(attack)
                .contains::<PendingBloodShotMotion>()
        );
        {
            let world = app.world_mut();
            let mut child_query = world.query_filtered::<Entity, With<TweenChild>>();
            assert_eq!(child_query.iter(world).count(), 0);
        }

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(1));
        app.update();

        assert!(
            !app.world()
                .entity(attack)
                .contains::<PendingBloodShotMotion>()
        );

        {
            let world = app.world_mut();
            let mut child_query = world.query_filtered::<(
                Option<&TargetingValueX>,
                Option<&TargetingValueY>,
                Option<&TargetingValueZ>,
            ), With<TweenChild>>();
            assert_eq!(child_query.iter(world).count(), 3);
        }
    }
}
