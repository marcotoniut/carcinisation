//! Entity spawning systems for enemies, pickups, destructibles, and objects.

use crate::pixel::{CxAssets, CxSpriteBundle};
use crate::stage::{
    components::{
        SpawnDrop, StageEntity,
        interactive::{Collider, ColliderData, Dead},
        placement::{Airborne, AuthoredDepths, Depth},
    },
    data::ContainerSpawn,
    depth_scale::{DepthFallbackScale, DepthScaleConfig},
    destructible::{
        components::{DestructibleState, make_animation_bundle},
        data::destructibles::DESTRUCTIBLE_ANIMATIONS,
    },
    enemy::{
        composed::{ComposedAnimationState, ComposedEnemyVisual},
        data::{mosquiton::TAG_IDLE_FLY, spidey::TAG_IDLE as SPIDEY_TAG_IDLE},
        entity::EnemyType,
        mosquito::entity::{
            ENEMY_MOSQUITO_BASE_HEALTH, ENEMY_MOSQUITO_RADIUS, MosquitoBundle,
            MosquitoDefaultBundle,
        },
        mosquiton::entity::{MosquitonBundle, MosquitonDefaultBundle},
        spidey::entity::{
            ENEMY_SPIDEY_BASE_HEALTH, EnemySpideyAttacking, EnemySpideyBehaviorLoop, SpideyBundle,
            SpideyDefaultBundle,
        },
        tardigrade::entity::{
            ENEMY_TARDIGRADE_BASE_HEALTH, ENEMY_TARDIGRADE_RADIUS, TardigradeBundle,
            TardigradeDefaultBundle,
        },
    },
    floors::ActiveFloors,
    parallax::{ActiveParallaxAttenuation, ParallaxOffset, parallax_offset_for},
    player::{attacks::AttackHitTracker, components::PlayerAttack},
    resources::{ActiveProjection, ProjectionView, StageStepSpawner, StageTimeDomain},
    spawn_placement,
};
use crate::{
    layer::Layer,
    stage::{
        components::{
            interactive::{Flickerer, Health, HealthOverride, Hittable, Object},
            placement::Speed,
        },
        data::{EnemySpawn, ObjectSpawn, ObjectType, PickupSpawn, PickupType, StageSpawn},
        destructible::{components::Destructible, data::DestructibleSpawn},
        enemy::components::{
            Enemy, EnemyContinuousDepth,
            behavior::{EnemyBehaviors, GroundedEnemyFall},
        },
        messages::StageSpawnEvent,
        pickup::components::HealthRecovery,
    },
    systems::camera::CameraPos,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::{CxAnchor, CxPresentationTransform, CxSprite, WorldPos};

/// Build an [`AuthoredDepths`] component from spawn data.
///
/// If the spawn specifies explicit authored depths, uses those.
/// For composed enemies (e.g. Mosquiton, Spidey), defaults to the canonical
/// authored depth from [`EnemyType::composed_authored_depth`] — assets exist
/// only at that depth, and other depths use fallback scaling.
/// Otherwise defaults to the spawn's own depth as the only authored depth.
fn authored_depths_from_spawn(
    depth: Depth,
    enemy_type: Option<EnemyType>,
    explicit: Option<&Vec<Depth>>,
) -> AuthoredDepths {
    if let Some(depths) = explicit {
        AuthoredDepths::new(depths.clone())
    } else {
        let base = enemy_type
            .and_then(|et| et.composed_authored_depth())
            .unwrap_or(depth);
        AuthoredDepths::single(base)
    }
}

fn composed_root_visibility() -> Visibility {
    Visibility::Hidden
}

fn seed_grounded_enemy_fall_if_spawned_above_floor(
    commands: &mut Commands,
    entity: Entity,
    depth: Depth,
    position: Vec2,
    floors: &ActiveFloors,
) {
    let Some(floor_y) = floors.highest_solid_y_at_or_below(depth, position.y) else {
        return;
    };

    if position.y > floor_y {
        commands.entity(entity).insert((
            Airborne,
            GroundedEnemyFall {
                vertical_velocity: 0.0,
            },
        ));
    }
}

/// Spawn-time presentation basis for a composed enemy root.
///
/// This mirrors the steady-state runtime pipeline:
/// - depth fallback uses [`DepthScaleConfig::resolve_fallback`]
/// - parallax uses [`parallax_offset_for`]
/// - composed offsets are written into [`CxPresentationTransform`]
///
/// The root stays hidden until the composed visual pipeline has produced its
/// first valid frame, but that hidden state is **not** a grace period for
/// same-frame repair. Spawn must write the correct basis up front so any later
/// visibility flip is already safe.
struct ComposedPresentationBasis {
    parallax: ParallaxOffset,
    presentation: CxPresentationTransform,
    depth_fallback: Option<DepthFallbackScale>,
}

/// Computes the authoritative spawn-time presentation basis for a composed
/// entity.
///
/// This enforces the core invariant:
/// any composed entity that becomes visible must already have correct
/// presentation state in that same frame.
///
/// Spawn must **not** depend on same-frame maintenance systems to fix scale or
/// offsets later, and reveal must never repair presentation. Runtime systems
/// only maintain this basis after spawn.
fn compute_presentation_basis(
    _position: Vec2,
    depth: Depth,
    authored: &AuthoredDepths,
    depth_scale_config: &DepthScaleConfig,
    projection: Option<&ActiveProjection>,
    projection_view: Option<&ProjectionView>,
    parallax_attenuation: Option<f32>,
) -> ComposedPresentationBasis {
    let scale_factor = depth_scale_config.resolve_fallback(depth, authored);
    let collision_offset = match (projection, projection_view) {
        (Some(projection), Some(view)) => parallax_offset_for(
            &projection.0,
            view.lateral_view_offset,
            parallax_attenuation.unwrap_or(1.0),
            projection.0.floor_y_for_depth(depth.to_i8()),
        ),
        _ => Vec2::ZERO,
    };

    ComposedPresentationBasis {
        parallax: ParallaxOffset(collision_offset),
        presentation: CxPresentationTransform {
            scale: Vec2::splat(scale_factor),
            rotation: 0.0,
            visual_offset: collision_offset,
            collision_offset,
        },
        depth_fallback: ((scale_factor - 1.0).abs() >= f32::EPSILON)
            .then_some(DepthFallbackScale(Vec2::splat(scale_factor))),
    }
}

/// @system Drains the step spawner queue, triggering spawns whose elapsed time has come.
pub fn check_step_spawn(
    mut commands: Commands,
    mut stage_step_spawner_query: Query<&mut StageStepSpawner>,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for mut stage_step_spawner in &mut stage_step_spawner_query.iter_mut() {
        let mut elapsed = stage_step_spawner.elapsed + stage_time.delta();

        stage_step_spawner.spawns.retain_mut(|spawn| {
            let spawn_elapsed = spawn.get_elapsed();
            if spawn_elapsed <= elapsed {
                elapsed -= spawn_elapsed;
                commands.trigger(StageSpawnEvent {
                    spawn: spawn.clone(),
                });
                false
            } else {
                true
            }
        });

        stage_step_spawner.elapsed = elapsed;
    }
}

/// @trigger Spawns an entity (enemy, pickup, destructible, or object) from a stage spawn event.
pub fn on_stage_spawn(
    trigger: On<StageSpawnEvent>,
    mut commands: Commands,
    mut assets_sprite: CxAssets<CxSprite>,
    asset_server: Res<AssetServer>,
    active_floors: Res<ActiveFloors>,
    depth_scale_config: Res<DepthScaleConfig>,
    active_projection: Option<Res<ActiveProjection>>,
    projection_view: Option<Res<ProjectionView>>,
    parallax_attenuation: Option<Res<ActiveParallaxAttenuation>>,
    camera_query: Query<&WorldPos, With<CameraPos>>,
) {
    match &trigger.event().spawn {
        StageSpawn::Destructible(x) => {
            spawn_destructible(&mut commands, &mut assets_sprite, x);
        }
        StageSpawn::Enemy(x) => {
            let camera_pos = camera_query.single().unwrap();
            spawn_enemy(
                &mut commands,
                &asset_server,
                camera_pos.0,
                x,
                &active_floors,
                &depth_scale_config,
                active_projection.as_deref(),
                projection_view.as_deref(),
                parallax_attenuation.map(|a| a.0),
            );
        }
        StageSpawn::Object(x) => {
            spawn_object(&mut commands, &mut assets_sprite, x);
        }
        StageSpawn::Pickup(x) => {
            let camera_pos = camera_query.single().unwrap();
            spawn_pickup(&mut commands, &mut assets_sprite, camera_pos.0, x);
        }
    }
}

pub fn spawn_pickup(
    commands: &mut Commands,
    assets_sprite: &mut CxAssets<CxSprite>,
    offset: Vec2,
    spawn: &PickupSpawn,
) -> Entity {
    let PickupSpawn {
        pickup_type,
        coordinates,
        ..
    } = spawn;
    let position = WorldPos::from(offset + *coordinates);
    let authored = authored_depths_from_spawn(spawn.depth, None, spawn.authored_depths.as_ref());
    match pickup_type {
        PickupType::BigHealthpack => {
            let sprite = assets_sprite.load(assert_assets_path!(
                "sprites/pickups/health_6.px_sprite.png"
            ));
            commands
                .spawn((
                    spawn.get_name(),
                    Hittable,
                    StageEntity,
                    CxSpriteBundle::<Layer> {
                        sprite: sprite.into(),
                        anchor: CxAnchor::Center,
                        layer: spawn.depth.to_layer(),
                        ..default()
                    },
                    position,
                    spawn.depth,
                    authored,
                    Health(1),
                    ColliderData::from_one(Collider::new_box(Vec2::new(12., 8.))),
                    HealthRecovery(100),
                    ParallaxOffset::default(),
                    CxPresentationTransform::default(),
                ))
                .id()
        }
        PickupType::SmallHealthpack => {
            let sprite = assets_sprite.load(assert_assets_path!(
                "sprites/pickups/health_4.px_sprite.png"
            ));
            commands
                .spawn((
                    spawn.get_name(),
                    Hittable,
                    StageEntity,
                    CxSpriteBundle::<Layer> {
                        sprite: sprite.into(),
                        anchor: CxAnchor::BottomCenter,
                        layer: spawn.depth.to_layer(),
                        ..default()
                    },
                    position,
                    spawn.depth,
                    authored.clone(),
                    Health(1),
                    ColliderData::from_one(Collider::new_box(Vec2::new(7., 5.))),
                    HealthRecovery(30),
                    ParallaxOffset::default(),
                    CxPresentationTransform::default(),
                ))
                .id()
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn spawn_enemy(
    commands: &mut Commands,
    asset_server: &AssetServer,
    offset: Vec2,
    spawn: &EnemySpawn,
    floors: &ActiveFloors,
    depth_scale_config: &DepthScaleConfig,
    active_projection: Option<&ActiveProjection>,
    projection_view: Option<&ProjectionView>,
    parallax_attenuation: Option<f32>,
) -> Entity {
    let EnemySpawn {
        enemy_type,
        speed,
        health,
        steps,
        contains,
        depth,
        ..
    } = spawn;
    let name = spawn.enemy_type.get_name();
    let position = spawn_placement::resolve_enemy_position(spawn, offset, floors);
    let behaviors = EnemyBehaviors::new(steps.clone());
    let continuous_depth = EnemyContinuousDepth::from_depth(*depth);
    let authored =
        authored_depths_from_spawn(*depth, Some(*enemy_type), spawn.authored_depths.as_ref());
    match enemy_type {
        EnemyType::Mosquito => {
            let collider: Collider =
                Collider::new_circle(ENEMY_MOSQUITO_RADIUS).with_offset(Vec2::new(0., 2.));
            let critical_collider = collider.new_scaled(0.4).with_defense(0.4);

            let entity = commands
                .spawn((
                    MosquitoBundle {
                        depth: *depth,
                        speed: Speed(*speed),
                        behaviors,
                        position: WorldPos::from(position),
                        collider_data: ColliderData::from_many(vec![critical_collider, collider]),
                        default: MosquitoDefaultBundle {
                            health: Health(health.unwrap_or(ENEMY_MOSQUITO_BASE_HEALTH)),
                            ..MosquitoDefaultBundle::default()
                        },
                    },
                    authored.clone(),
                    continuous_depth,
                    ParallaxOffset::default(),
                    CxPresentationTransform::default(),
                ))
                .id();

            if let Some(contains) = contains {
                commands.entity(entity).insert(SpawnDrop {
                    contains: *contains.clone(),
                    entity,
                });
            }
            entity
        }
        EnemyType::Mosquiton => {
            let initial_presentation = compute_presentation_basis(
                position,
                *depth,
                &authored,
                depth_scale_config,
                active_projection,
                projection_view,
                parallax_attenuation,
            );
            let entity = commands
                .spawn((
                    MosquitonBundle {
                        behaviors,
                        composed_animation: ComposedAnimationState::new(TAG_IDLE_FLY),
                        composed_visual: ComposedEnemyVisual::for_enemy(
                            asset_server,
                            EnemyType::Mosquiton,
                            EnemyType::Mosquiton
                                .composed_authored_depth()
                                .unwrap_or(*depth),
                        ),
                        transform: Transform::default(),
                        global_transform: GlobalTransform::default(),
                        // Hidden is not a repair window. Spawn must write the
                        // correct presentation basis now; reveal later only
                        // publishes the first valid composed frame.
                        visibility: composed_root_visibility(),
                        inherited_visibility: InheritedVisibility::VISIBLE,
                        depth: *depth,
                        position: WorldPos::from(position),
                        speed: Speed(*speed),
                        default: MosquitonDefaultBundle {
                            health: Health(health.unwrap_or(ENEMY_MOSQUITO_BASE_HEALTH)),
                            ..MosquitonDefaultBundle::default()
                        },
                    },
                    authored.clone(),
                    continuous_depth,
                    initial_presentation.parallax,
                    initial_presentation.presentation,
                ))
                .id();

            if let Some(depth_fallback) = initial_presentation.depth_fallback {
                commands.entity(entity).insert(depth_fallback);
            }

            // Flying mosquitons (altitude-based) start airborne.
            if spawn.altitude.is_some() {
                commands.entity(entity).insert(Airborne);
            }

            if let Some(health) = health {
                commands.entity(entity).insert(HealthOverride(*health));
            }

            if let Some(contains) = contains {
                commands.entity(entity).insert(SpawnDrop {
                    contains: *contains.clone(),
                    entity,
                });
            }
            entity
        }
        EnemyType::Spidey => {
            let initial_presentation = compute_presentation_basis(
                position,
                *depth,
                &authored,
                depth_scale_config,
                active_projection,
                projection_view,
                parallax_attenuation,
            );
            let entity = commands
                .spawn((
                    SpideyBundle {
                        behaviors,
                        composed_animation: ComposedAnimationState::new(SPIDEY_TAG_IDLE),
                        composed_visual: ComposedEnemyVisual::for_enemy(
                            asset_server,
                            EnemyType::Spidey,
                            EnemyType::Spidey
                                .composed_authored_depth()
                                .unwrap_or(*depth),
                        ),
                        transform: Transform::default(),
                        global_transform: GlobalTransform::default(),
                        // Hidden is not a repair window. Spawn must write the
                        // correct presentation basis now; reveal later only
                        // publishes the first valid composed frame.
                        visibility: composed_root_visibility(),
                        inherited_visibility: InheritedVisibility::VISIBLE,
                        depth: *depth,
                        position: WorldPos::from(position),
                        speed: Speed(*speed),
                        default: SpideyDefaultBundle {
                            health: Health(health.unwrap_or(ENEMY_SPIDEY_BASE_HEALTH)),
                            ..SpideyDefaultBundle::default()
                        },
                    },
                    authored.clone(),
                    continuous_depth,
                    initial_presentation.parallax,
                    initial_presentation.presentation,
                ))
                .id();

            commands
                .entity(entity)
                .insert(EnemySpideyAttacking::default());

            if let Some(depth_fallback) = initial_presentation.depth_fallback {
                commands.entity(entity).insert(depth_fallback);
            }

            if let Some(health) = health {
                commands.entity(entity).insert(HealthOverride(*health));
            }

            if let Some(contains) = contains {
                commands.entity(entity).insert(SpawnDrop {
                    contains: *contains.clone(),
                    entity,
                });
            }
            if !steps.is_empty() {
                commands
                    .entity(entity)
                    .insert(EnemySpideyBehaviorLoop(steps.clone()));
            }
            seed_grounded_enemy_fall_if_spawned_above_floor(
                commands, entity, *depth, position, floors,
            );
            entity
        }
        EnemyType::Kyle | EnemyType::Marauder | EnemyType::Spidomonsta => commands
            .spawn((name, Enemy, continuous_depth, behaviors, authored.clone()))
            .id(),
        EnemyType::Tardigrade => {
            let collider =
                Collider::new_circle(ENEMY_TARDIGRADE_RADIUS).with_offset(Vec2::new(-3., 2.));
            let critical_collider = collider.new_scaled(0.4).with_defense(0.2);

            commands
                .spawn((
                    TardigradeBundle {
                        depth: *depth,
                        speed: Speed(*speed),
                        behaviors,
                        position: WorldPos::from(position),
                        collider_data: ColliderData::from_many(vec![critical_collider, collider]),
                        default: TardigradeDefaultBundle {
                            health: Health(health.unwrap_or(ENEMY_TARDIGRADE_BASE_HEALTH)),
                            ..default()
                        },
                    },
                    authored,
                    continuous_depth,
                    ParallaxOffset::default(),
                    CxPresentationTransform::default(),
                ))
                .id()
        }
    }
}

/// Spawns a destructible entity with its animation bundle.
// TODO move to Destructible mod?
pub fn spawn_destructible(
    commands: &mut Commands,
    assets_sprite: &mut CxAssets<CxSprite>,
    spawn: &DestructibleSpawn,
) -> Entity {
    let animations_map = &DESTRUCTIBLE_ANIMATIONS.get_animation_data(&spawn.destructible_type);
    let animation_bundle_o = make_animation_bundle(
        assets_sprite,
        animations_map,
        &DestructibleState::Base,
        &spawn.depth,
    );
    let animation_bundle = animation_bundle_o.unwrap();
    let authored = authored_depths_from_spawn(spawn.depth, None, spawn.authored_depths.as_ref());

    commands
        .spawn((
            Destructible,
            Flickerer,
            Hittable,
            spawn.get_name(),
            spawn.depth,
            authored,
            Health(spawn.health),
            spawn.destructible_type,
            animation_bundle,
            WorldPos::from(spawn.coordinates),
            StageEntity,
            ParallaxOffset::default(),
            CxPresentationTransform::default(),
        ))
        .id()
}

pub fn spawn_object(
    commands: &mut Commands,
    assets_sprite: &mut CxAssets<CxSprite>,
    spawn: &ObjectSpawn,
) -> Entity {
    let sprite = assets_sprite.load(match spawn.object_type {
        ObjectType::BenchBig => assert_assets_path!("sprites/objects/bench_big.px_sprite.png"),
        ObjectType::BenchSmall => assert_assets_path!("sprites/objects/bench_small.px_sprite.png"),
        ObjectType::Fibertree => assert_assets_path!("sprites/objects/fiber_tree.px_sprite.png"),
        ObjectType::RugparkSign => {
            assert_assets_path!("sprites/objects/rugpark_sign.px_sprite.png")
        }
    });
    let authored = authored_depths_from_spawn(spawn.depth, None, spawn.authored_depths.as_ref());
    commands
        .spawn((
            spawn.get_name(),
            Object,
            CxSpriteBundle::<Layer> {
                sprite: sprite.into(),
                anchor: CxAnchor::BottomCenter,
                layer: spawn.depth.to_layer(),
                ..default()
            },
            spawn.depth,
            authored,
            WorldPos::from(spawn.coordinates),
            StageEntity,
            ParallaxOffset::default(),
            CxPresentationTransform::default(),
        ))
        .id()
}

/// @system Spawns contained items when a carrier entity dies.
pub fn check_dead_drop(
    mut commands: Commands,
    mut assets_sprite: CxAssets<CxSprite>,
    asset_server: Res<AssetServer>,
    active_floors: Res<ActiveFloors>,
    depth_scale_config: Res<DepthScaleConfig>,
    active_projection: Option<Res<ActiveProjection>>,
    projection_view: Option<Res<ProjectionView>>,
    parallax_attenuation: Option<Res<ActiveParallaxAttenuation>>,
    mut attack_query: Query<&mut AttackHitTracker, With<PlayerAttack>>,
    query: Query<(&SpawnDrop, &WorldPos, &Depth), Added<Dead>>,
) {
    for (spawn_drop, position, depth) in &mut query.iter() {
        let entity = match spawn_drop.contains.clone() {
            ContainerSpawn::Pickup(spawn) => spawn_pickup(
                &mut commands,
                &mut assets_sprite,
                Vec2::ZERO,
                &spawn.from_spawn(position.0, *depth),
            ),
            ContainerSpawn::Enemy(spawn) => spawn_enemy(
                &mut commands,
                &asset_server,
                Vec2::ZERO,
                &spawn.from_spawn(position.0, *depth),
                &active_floors,
                &depth_scale_config,
                active_projection.as_deref(),
                projection_view.as_deref(),
                parallax_attenuation.as_ref().map(|a| a.0),
            ),
        };

        for mut hit_tracker in &mut attack_query.iter_mut() {
            hit_tracker.inherit_hit(spawn_drop.entity, entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{composed_root_visibility, compute_presentation_basis, spawn_enemy};
    use crate::{
        globals::ASSETS_PATH,
        layer::Layer,
        stage::{
            components::placement::{AuthoredDepths, Depth},
            data::EnemySpawn,
            depth_scale::{DepthFallbackScale, DepthScaleConfig, apply_depth_fallback_scale},
            enemy::composed::{
                ComposedEnemyVisualReady, ComposedFrameOutput, CompositionAtlasAsset,
                apply_composed_enemy_visuals,
            },
            floors::ActiveFloors,
            parallax::{
                ActiveParallaxAttenuation, ParallaxOffset, compose_presentation_offsets,
                update_parallax_offset,
            },
            projection::ProjectionProfile,
            resources::{ActiveProjection, ProjectionView},
            spawn_placement,
        },
    };
    use bevy::{
        asset::{AssetMetaCheck, AssetPlugin},
        input::InputPlugin,
        prelude::{
            App, AssetApp, AssetServer, Handle, IVec2, IntoScheduleConfigs, MinimalPlugins,
            PostUpdate, UVec2, Update, Vec2, Visibility, default,
        },
    };
    use carapace::{
        CxPlugin,
        prelude::{
            CxAnchor, CxAuthoritativeCompositeMetrics, CxCompositePart, CxCompositeSprite,
            CxPresentationTransform, CxSpriteAsset, WorldPos,
        },
    };

    fn priming_runtime_app(
        config: DepthScaleConfig,
        projection: ActiveProjection,
        view: ProjectionView,
        attenuation: f32,
    ) -> App {
        let mut app = App::new();
        app.insert_resource(config);
        app.insert_resource(projection);
        app.insert_resource(view);
        app.insert_resource(ActiveParallaxAttenuation(attenuation));
        app.add_systems(
            Update,
            (
                apply_depth_fallback_scale,
                update_parallax_offset,
                compose_presentation_offsets,
            )
                .chain(),
        );
        app
    }

    fn first_visible_frame_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin));
        app.add_plugins(AssetPlugin {
            file_path: ASSETS_PATH.into(),
            meta_check: AssetMetaCheck::Never,
            ..default()
        });
        CxPlugin::<Layer>::new(UVec2::ONE, "palette/base.png").build_headless(&mut app);
        app.init_asset::<CompositionAtlasAsset>();
        app.add_systems(PostUpdate, apply_composed_enemy_visuals);
        app
    }

    #[test]
    fn composed_root_visibility_starts_hidden() {
        assert_eq!(composed_root_visibility(), Visibility::Hidden);
    }

    #[test]
    fn composed_spawn_primes_non_authored_depth_presentation() {
        let config = DepthScaleConfig::default();
        let authored = AuthoredDepths::single(Depth::Three);
        let projection = ActiveProjection(ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        });
        let view = ProjectionView {
            lateral_view_offset: 40.0,
            lateral_anchor_x: 0.0,
        };

        let initial = compute_presentation_basis(
            Vec2::new(72.0, 0.0),
            Depth::Five,
            &authored,
            &config,
            Some(&projection),
            Some(&view),
            Some(1.0),
        );

        assert!(initial.depth_fallback.is_some());
        assert_ne!(initial.parallax.0, Vec2::ZERO);
        assert_ne!(initial.presentation.scale, Vec2::ONE);
        assert_ne!(initial.presentation.visual_offset, Vec2::ZERO);
        assert_eq!(
            initial.presentation.collision_offset, initial.parallax.0,
            "spawn-time presentation must match the parallax contributor"
        );
        assert_eq!(
            initial.presentation.visual_offset, initial.parallax.0,
            "composed spawn should not invent a separate visual-only offset"
        );
    }

    #[test]
    fn composed_spawn_keeps_identity_basis_when_already_authored_and_unshifted() {
        let config = DepthScaleConfig::default();
        let authored = AuthoredDepths::single(Depth::Three);
        let initial = compute_presentation_basis(
            Vec2::new(72.0, 0.0),
            Depth::Three,
            &authored,
            &config,
            None,
            None,
            None,
        );

        assert!(initial.depth_fallback.is_none());
        assert_eq!(initial.parallax.0, ParallaxOffset::default().0);
        assert_eq!(
            initial.presentation.scale,
            CxPresentationTransform::default().scale
        );
        assert_eq!(
            initial.presentation.visual_offset,
            CxPresentationTransform::default().visual_offset
        );
        assert_eq!(
            initial.presentation.collision_offset,
            CxPresentationTransform::default().collision_offset
        );
    }

    #[test]
    fn composed_spawn_priming_matches_runtime_presentation_pipeline() {
        let config = DepthScaleConfig::default();
        let authored = AuthoredDepths::single(Depth::Three);
        let position = Vec2::new(72.0, 24.0);
        let depth = Depth::Five;
        let projection = ActiveProjection(ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        });
        let view = ProjectionView {
            lateral_view_offset: 40.0,
            lateral_anchor_x: 0.0,
        };
        let attenuation = 0.75;

        let initial = compute_presentation_basis(
            position,
            depth,
            &authored,
            &config,
            Some(&projection),
            Some(&view),
            Some(attenuation),
        );

        let mut app = priming_runtime_app(config.clone(), projection, view, attenuation);
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(position),
                depth,
                authored,
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let runtime_parallax = app
            .world()
            .entity(entity)
            .get::<ParallaxOffset>()
            .expect("runtime parallax should exist");
        let runtime_presentation = app
            .world()
            .entity(entity)
            .get::<CxPresentationTransform>()
            .expect("runtime presentation should exist");
        let runtime_depth_fallback = app
            .world()
            .entity(entity)
            .get::<DepthFallbackScale>()
            .copied();

        assert_eq!(runtime_parallax.0, initial.parallax.0);
        assert_eq!(runtime_presentation.scale, initial.presentation.scale);
        assert_eq!(
            runtime_presentation.visual_offset,
            initial.presentation.visual_offset
        );
        assert_eq!(
            runtime_presentation.collision_offset,
            initial.presentation.collision_offset
        );
        assert_eq!(
            runtime_depth_fallback.map(|value| value.0),
            initial.depth_fallback.map(|value| value.0)
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn composed_spawn_reveal_integration_keeps_primed_presentation_basis() {
        let mut app = first_visible_frame_app();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let floors = ActiveFloors::default();
        let config = DepthScaleConfig::default();
        let spawn = EnemySpawn::mosquiton_base()
            .with_coordinates(Vec2::new(72.0, 24.0))
            .with_depth(Depth::Five);
        let authored = AuthoredDepths::single(Depth::Three);
        let projection = ActiveProjection(ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        });
        let view = ProjectionView {
            lateral_view_offset: 40.0,
            lateral_anchor_x: 0.0,
        };
        let attenuation = 0.75;
        let resolved_position =
            spawn_placement::resolve_enemy_position(&spawn, Vec2::ZERO, &floors);
        let expected = compute_presentation_basis(
            resolved_position,
            spawn.depth,
            &authored,
            &config,
            Some(&projection),
            Some(&view),
            Some(attenuation),
        );

        let entity = spawn_enemy(
            &mut app.world_mut().commands(),
            &asset_server,
            Vec2::ZERO,
            &spawn,
            &floors,
            &config,
            Some(&projection),
            Some(&view),
            Some(attenuation),
        );
        app.world_mut().flush();

        let entity_ref = app.world().entity(entity);
        assert_eq!(
            entity_ref
                .get::<Visibility>()
                .expect("spawned composed root should have visibility"),
            &Visibility::Hidden
        );
        assert_eq!(
            entity_ref
                .get::<ParallaxOffset>()
                .expect("spawned composed root should have parallax")
                .0,
            expected.parallax.0
        );
        let presentation = entity_ref
            .get::<CxPresentationTransform>()
            .expect("spawned composed root should have presentation");
        assert_eq!(presentation.scale, expected.presentation.scale);
        assert_eq!(
            presentation.visual_offset,
            expected.presentation.visual_offset
        );
        assert_eq!(
            presentation.collision_offset,
            expected.presentation.collision_offset
        );
        assert_eq!(
            entity_ref.get::<DepthFallbackScale>().map(|value| value.0),
            expected.depth_fallback.map(|value| value.0)
        );

        app.world_mut().entity_mut(entity).insert((
            ComposedFrameOutput::visible_frame(
                vec![CxCompositePart::new(Handle::<CxSpriteAsset>::default())],
                IVec2::ZERO,
                UVec2::ONE,
                CxAnchor::Center,
            ),
            CxAuthoritativeCompositeMetrics,
            CxCompositeSprite::default(),
            ComposedEnemyVisualReady,
        ));
        app.world_mut().flush();

        assert_eq!(
            app.world()
                .entity(entity)
                .get::<Visibility>()
                .expect("ready root should still be hidden before reveal"),
            &Visibility::Hidden
        );

        app.update();

        let entity_ref = app.world().entity(entity);
        assert_eq!(
            entity_ref
                .get::<Visibility>()
                .expect("revealed composed root should have visibility"),
            &Visibility::Visible
        );
        assert_eq!(
            entity_ref
                .get::<ParallaxOffset>()
                .expect("revealed composed root should retain parallax")
                .0,
            expected.parallax.0
        );
        let presentation = entity_ref
            .get::<CxPresentationTransform>()
            .expect("revealed composed root should retain presentation");
        assert_eq!(presentation.scale, expected.presentation.scale);
        assert_eq!(
            presentation.visual_offset,
            expected.presentation.visual_offset
        );
        assert_eq!(
            presentation.collision_offset,
            expected.presentation.collision_offset
        );
        assert_eq!(
            entity_ref.get::<DepthFallbackScale>().map(|value| value.0),
            expected.depth_fallback.map(|value| value.0)
        );
    }
}
