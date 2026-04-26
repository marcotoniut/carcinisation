use crate::stage::{
    attack::{
        components::{
            EnemyAttack, EnemyAttackOriginDepth, EnemyAttackOriginPosition,
            EnemyHoveringAttackType, bundles::make_hovering_attack_atlas_bundle,
        },
        data::spider_shot::SpiderShotConfig,
        spawns::{ProjectileSpawnSourceBasis, projectile_spawn_world_pos_from_source},
    },
    components::{
        StageEntity,
        damage::InflictsDamage,
        interactive::{ColliderData, Health, Hittable},
        placement::{AuthoredDepths, Depth},
    },
    depth_scale::{DepthFallbackScale, DepthScaleConfig},
    player::components::PLAYER_DEPTH,
    resources::StageTimeDomain,
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxPosition, CxPresentationTransform, CxSpriteAtlasAsset, WorldPos,
};

use cween::{
    linear::components::{TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildBundle},
    structs::{Constructor, Magnitude},
};
use std::time::Duration;

fn spawn_spider_shot_tween_child<P>(
    commands: &mut Commands,
    bundle: TweenChildBundle<StageTimeDomain, P>,
    axis_name: &'static str,
) where
    P: Constructor<f32> + Component + Magnitude,
{
    commands.spawn((bundle, SpiderShotTween, Name::new(axis_name)));
}

#[derive(Bundle)]
pub struct SpiderShotDefaultBundle {
    pub name: Name,
    pub enemy_attack: EnemyAttack,
    pub stage_entity: StageEntity,
    pub health: Health,
    pub hittable: Hittable,
}

impl Default for SpiderShotDefaultBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Attack<SpiderShot>"),
            enemy_attack: EnemyAttack,
            stage_entity: StageEntity,
            health: Health(1),
            hittable: Hittable,
        }
    }
}

/// Marker component for spider shot tween children.
#[derive(Component, Clone, Debug)]
pub struct SpiderShotTween;

/// Holds a freshly spawned spider shot at its authored cue origin briefly so
/// the first visible frame reads as emerging from the spinneret before travel.
#[derive(Component, Clone, Debug)]
pub struct PendingSpiderShotMotion {
    pub armed_at: Duration,
    pub speed: Vec2,
}

#[derive(Bundle)]
pub struct SpiderShotBundle {
    pub enemy_attack_origin_position: EnemyAttackOriginPosition,
    pub enemy_attack_origin_depth: EnemyAttackOriginDepth,
    pub enemy_hovering_attack_type: EnemyHoveringAttackType,
    pub depth: Depth,
    pub inflicts_damage: InflictsDamage,
    pub position: WorldPos,
    pub targeting_value_x: TargetingValueX,
    pub targeting_value_y: TargetingValueY,
    pub targeting_value_z: TargetingValueZ,
    pub default: SpiderShotDefaultBundle,
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_spider_shot_attack(
    commands: &mut Commands,
    asset_server: &AssetServer,
    atlas_assets: &Assets<CxSpriteAtlasAsset>,
    stage_time: &Res<Time<StageTimeDomain>>,
    config: &SpiderShotConfig,
    depth_scale_config: &DepthScaleConfig,
    target_pos: Vec2,
    source_muzzle_world_pos: Vec2,
    source_presentation: Option<&CxPresentationTransform>,
    depth: &Depth,
) {
    let attack_type = EnemyHoveringAttackType::SpiderShot;
    let target_pos = target_pos
        + Vec2::new(
            (1. - rand::random::<f32>()) * config.randomness,
            (1. - rand::random::<f32>()) * config.randomness,
        );

    let (atlas_sprite, animation, _circle_collider) =
        make_hovering_attack_atlas_bundle(asset_server, atlas_assets, &attack_type);

    let spawn_world_pos = projectile_spawn_world_pos_from_source(
        source_muzzle_world_pos,
        source_presentation,
        ProjectileSpawnSourceBasis::WorldSpace,
    );
    // Time-matched velocity: the web arrives at the target's 2D position
    // exactly when its depth tween reaches the player depth.  This prevents
    // overshooting at deep spawn depths where constant-speed projectiles
    // would drift far past the target during the long depth transit.
    let depth_distance = (depth.to_f32() - PLAYER_DEPTH.to_f32()).abs();
    let depth_time = if config.depth_speed.abs() > f32::EPSILON && depth_distance > f32::EPSILON {
        depth_distance / config.depth_speed.abs()
    } else {
        1.0
    };
    let displacement = target_pos - spawn_world_pos;
    let speed = displacement / depth_time;

    let mut entity_commands = commands.spawn(SpiderShotBundle {
        enemy_attack_origin_position: EnemyAttackOriginPosition(spawn_world_pos),
        enemy_attack_origin_depth: EnemyAttackOriginDepth(*depth),
        enemy_hovering_attack_type: EnemyHoveringAttackType::SpiderShot,
        depth: *depth,
        inflicts_damage: InflictsDamage(config.damage),
        position: WorldPos(spawn_world_pos),
        targeting_value_x: spawn_world_pos.x.into(),
        targeting_value_y: spawn_world_pos.y.into(),
        targeting_value_z: depth.to_f32().into(),
        default: default(),
    });

    #[cfg(debug_assertions)]
    entity_commands.insert(crate::stage::attack::components::EnemyAttackDebugPosition {
        current: spawn_world_pos,
        origin: spawn_world_pos,
    });

    // Insert CxPosition explicitly so the sprite renders at the correct
    // position on frame 0 (the auto-inserted default is (0,0) and the sync
    // system doesn't run until PostUpdate).
    let authored_depths = AuthoredDepths::single(Depth::One);
    entity_commands.insert(CxPosition::from(spawn_world_pos.round().as_ivec2()));

    // Pre-compute depth-fallback scale so the sprite has the correct size
    // on its first visible frame rather than flashing at 1× then shrinking.
    let fallback_ratio = depth_scale_config.resolve_fallback(*depth, &authored_depths);
    if (fallback_ratio - 1.0).abs() >= f32::EPSILON {
        entity_commands.insert((
            CxPresentationTransform {
                scale: Vec2::splat(fallback_ratio),
                ..default()
            },
            DepthFallbackScale(Vec2::splat(fallback_ratio)),
        ));
    }

    entity_commands.insert((
        atlas_sprite,
        animation,
        CxAnchor::Center,
        (*depth - 1).to_layer(),
        authored_depths,
    ));

    // Use closed pixel-mask collision: the web's opaque pixels plus interior
    // transparent holes (scanline-filled) define the hitbox. This gives a
    // solid collision shape matching the web's visual outline.
    entity_commands.insert(ColliderData::from_one(
        carcinisation_collision::Collider::new(
            carcinisation_collision::ColliderShape::SpriteMaskClosed,
        ),
    ));

    entity_commands.insert(PendingSpiderShotMotion {
        armed_at: stage_time.elapsed() + config.startup_hold(),
        speed,
    });
}

pub fn arm_pending_spider_shot_motion(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    config: Res<SpiderShotConfig>,
    query: Query<(Entity, &PendingSpiderShotMotion, &WorldPos, &Depth), With<EnemyAttack>>,
) {
    for (entity, pending, position, depth) in &query {
        if stage_time.elapsed() < pending.armed_at {
            continue;
        }

        let direction = pending.speed.normalize_or_zero();
        let far_target = position.0 + direction * 1000.0;

        spawn_spider_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
                entity,
                position.0.x,
                far_target.x,
                pending.speed.x,
            ),
            "Spider Shot Tween X",
        );

        spawn_spider_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueY>::new(
                entity,
                position.0.y,
                far_target.y,
                pending.speed.y,
            ),
            "Spider Shot Tween Y",
        );

        spawn_spider_shot_tween_child(
            &mut commands,
            TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
                entity,
                depth.to_f32(),
                PLAYER_DEPTH.to_f32(),
                config.depth_speed,
            ),
            "Spider Shot Tween Z",
        );

        commands.entity(entity).remove::<PendingSpiderShotMotion>();
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
    fn pending_spider_shot_motion_arms_only_after_hold() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(SpiderShotConfig::load());
        app.add_systems(Update, arm_pending_spider_shot_motion);

        let attack = app
            .world_mut()
            .spawn((
                EnemyAttack,
                WorldPos(Vec2::new(10.0, 20.0)),
                Depth::Three,
                PendingSpiderShotMotion {
                    armed_at: Duration::from_millis(80),
                    speed: Vec2::new(5.0, 6.0),
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(79));
        app.update();

        assert!(
            app.world()
                .entity(attack)
                .contains::<PendingSpiderShotMotion>()
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
                .contains::<PendingSpiderShotMotion>()
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
