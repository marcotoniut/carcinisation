use crate::stage::{
    attack::{
        components::{
            AttachedToComposedPart, EnemyAttack, EnemyAttackOriginDepth, EnemyAttackOriginPosition,
            EnemyHoveringAttackType, bundles::make_hovering_attack_atlas_bundle,
        },
        data::blood_shot::BloodShotConfig,
    },
    components::{
        StageEntity,
        damage::InflictsDamage,
        interactive::{ColliderData, Health, Hittable},
        placement::{AuthoredDepths, Depth},
    },
    player::components::PLAYER_DEPTH,
    resources::StageTimeDomain,
};
use bevy::prelude::*;
use carapace::prelude::{CxAnchor, CxPresentationTransform, CxSpriteAtlasAsset, WorldPos};

use crate::stage::parallax::ParallaxOffset;
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
    pub stage_entity: StageEntity,
    pub health: Health,
    pub hittable: Hittable,
}

impl Default for BloodShotDefaultBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Attack<BloodShot>"),
            enemy_attack: EnemyAttack,
            stage_entity: StageEntity,
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
/// Far target is recomputed at arm time from the current position and speed
/// direction so attachment-moved projectiles travel correctly.
#[derive(Component, Clone, Debug)]
pub struct PendingBloodShotMotion {
    pub armed_at: Duration,
    pub speed: Vec2,
}

#[derive(Bundle)]
pub struct BloodShotBundle {
    pub enemy_attack_origin_position: EnemyAttackOriginPosition,
    pub enemy_attack_origin_depth: EnemyAttackOriginDepth,
    pub enemy_hovering_attack_type: EnemyHoveringAttackType,
    pub depth: Depth,
    pub inflicts_damage: InflictsDamage,
    pub position: WorldPos,
    pub targeting_value_x: TargetingValueX,
    pub targeting_value_y: TargetingValueY,
    pub targeting_value_z: TargetingValueZ,
    pub default: BloodShotDefaultBundle,
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_blood_shot_attack(
    commands: &mut Commands,
    asset_server: &AssetServer,
    atlas_assets: &Assets<CxSpriteAtlasAsset>,
    stage_time: &Res<Time<StageTimeDomain>>,
    config: &BloodShotConfig,
    target_pos: Vec2,
    current_pos: Vec2,
    depth: &Depth,
    gameplay_scale: f32,
    attachment: Option<AttachedToComposedPart>,
) {
    let attack_type = EnemyHoveringAttackType::BloodShot;
    // TODO should this account for player speed/direction?
    let target_pos = target_pos
        + Vec2::new(
            (1. - rand::random::<f32>()) * config.randomness,
            (1. - rand::random::<f32>()) * config.randomness,
        );

    let (atlas_sprite, animation, collider_data) =
        make_hovering_attack_atlas_bundle(asset_server, atlas_assets, &attack_type);

    let direction = target_pos - current_pos;
    let speed = direction.normalize_or_zero() * config.line_speed;

    let mut entity_commands = commands.spawn(BloodShotBundle {
        enemy_attack_origin_position: EnemyAttackOriginPosition(current_pos),
        enemy_attack_origin_depth: EnemyAttackOriginDepth(*depth),
        enemy_hovering_attack_type: EnemyHoveringAttackType::BloodShot,
        depth: *depth,
        inflicts_damage: InflictsDamage(config.damage),
        position: WorldPos(current_pos),
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

    entity_commands.insert((
        atlas_sprite,
        animation,
        CxAnchor::Center,
        (*depth - 1).to_layer(),
        AuthoredDepths::single(Depth::One),
        ParallaxOffset::default(),
        CxPresentationTransform::default(),
    ));

    if !collider_data.0.is_empty() {
        // Scale the collider by the spawning enemy's gameplay scale so
        // that blood shots from deep-depth (small) enemies have
        // proportionally smaller hit areas.
        let scaled_collider = ColliderData(
            collider_data
                .0
                .into_iter()
                .map(|c| c.new_scaled(gameplay_scale))
                .collect(),
        );
        entity_commands.insert(scaled_collider);
    }

    entity_commands.insert(PendingBloodShotMotion {
        armed_at: stage_time.elapsed() + config.startup_hold(),
        speed,
    });

    if let Some(attachment) = attachment {
        entity_commands.insert(attachment);
    }
}

pub fn arm_pending_blood_shot_motion(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    config: Res<BloodShotConfig>,
    query: Query<(Entity, &PendingBloodShotMotion, &WorldPos, &Depth), With<EnemyAttack>>,
) {
    for (entity, pending, position, depth) in &query {
        if stage_time.elapsed() < pending.armed_at {
            continue;
        }

        // Recompute far_target from the current (possibly attachment-moved)
        // position so the travel direction stays consistent with the actual
        // release point.
        let direction = pending.speed.normalize_or_zero();
        let far_target = position.0 + direction * 1000.0;

        spawn_blood_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
                entity,
                position.0.x,
                far_target.x,
                pending.speed.x,
            ),
            "Blood Shot Tween X",
        );

        spawn_blood_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueY>::new(
                entity,
                position.0.y,
                far_target.y,
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
                config.depth_speed,
            ),
            "Blood Shot Tween Z",
        );

        commands
            .entity(entity)
            .remove::<PendingBloodShotMotion>()
            .remove::<AttachedToComposedPart>();
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
        app.insert_resource(BloodShotConfig::load());
        app.add_systems(Update, arm_pending_blood_shot_motion);

        let attack = app
            .world_mut()
            .spawn((
                EnemyAttack,
                WorldPos(Vec2::new(10.0, 20.0)),
                Depth::Three,
                PendingBloodShotMotion {
                    armed_at: Duration::from_millis(60),
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

    #[test]
    fn arming_removes_attached_to_composed_part() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(BloodShotConfig::load());
        app.add_systems(Update, arm_pending_blood_shot_motion);

        let dummy_source = app.world_mut().spawn_empty().id();

        let attack = app
            .world_mut()
            .spawn((
                EnemyAttack,
                WorldPos(Vec2::new(10.0, 20.0)),
                Depth::Three,
                PendingBloodShotMotion {
                    armed_at: Duration::from_millis(60),
                    speed: Vec2::new(5.0, 6.0),
                },
                AttachedToComposedPart {
                    source_entity: dummy_source,
                    part_id: "head".to_string(),
                    local_offset: IVec2::new(6, 9),
                },
            ))
            .id();

        // Before hold expires: both components present.
        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(59));
        app.update();
        assert!(
            app.world()
                .entity(attack)
                .contains::<AttachedToComposedPart>()
        );

        // After hold expires: both removed.
        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(1));
        app.update();
        assert!(
            !app.world()
                .entity(attack)
                .contains::<PendingBloodShotMotion>()
        );
        assert!(
            !app.world()
                .entity(attack)
                .contains::<AttachedToComposedPart>()
        );
    }
}
