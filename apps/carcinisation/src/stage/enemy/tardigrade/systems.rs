use super::entity::{EnemyTardigrade, EnemyTardigradeAnimation};
use crate::pixel::{CxAssets, CxSpriteBundle};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    globals::SCREEN_RESOLUTION_F32_H,
    layer::Layer,
    stage::{
        attack::{
            data::boulder_throw::BoulderThrowConfig,
            spawns::boulder_throw::spawn_boulder_throw_attack,
        },
        components::{
            StageEntity,
            interactive::Dead,
            placement::{Depth, InView},
        },
        enemy::{
            bundles::make_enemy_animation_bundle, components::behavior::EnemyCurrentBehavior,
            data::tardigrade::TARDIGRADE_ANIMATIONS, tardigrade::entity::EnemyTardigradeAttacking,
        },
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxPresentationTransform, CxSprite, CxSpriteAtlasAsset, WorldPos,
};

use crate::stage::parallax::ParallaxOffset;
use std::time::Duration;

pub const ENEMY_TARDIGRADE_ATTACK_SPEED: f32 = 3.;

/// @system Picks the tardigrade idle sprite for the current depth.
pub fn assign_tardigrade_animation(
    mut commands: Commands,
    query: Query<
        (Entity, &EnemyCurrentBehavior, &WorldPos, &Depth),
        (With<EnemyTardigrade>, Without<EnemyTardigradeAnimation>),
    >,
    mut assets_sprite: CxAssets<CxSprite>,
) {
    for (entity, _current_behavior, position, depth) in &mut query.iter() {
        let bundle_o = TARDIGRADE_ANIMATIONS.idle.get(depth).map(|animation| {
            (
                EnemyTardigradeAnimation::Idle,
                make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
            )
        });

        if let Some((animation, (sprite_bundle, animation_bundle))) = bundle_o {
            commands.entity(entity).insert((
                WorldPos(position.0),
                animation,
                sprite_bundle,
                animation_bundle,
            ));
        }
    }
}

/// @system Spawns a death animation and awards score when a tardigrade dies.
pub fn despawn_dead_tardigrade(
    mut commands: Commands,
    assets_sprite: CxAssets<CxSprite>,
    mut score: ResMut<Score>,
    query: Query<(Entity, &EnemyTardigrade, &WorldPos, &Depth), Added<Dead>>,
) {
    for (entity, tardigrade, position, depth) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

        let animation_o = TARDIGRADE_ANIMATIONS.death.get(depth);

        if let Some(animation) = animation_o {
            let texture =
                assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

            commands.spawn((
                Name::new("Dead - Tardigrade"),
                WorldPos::from(position.0),
                CxSpriteBundle::<Layer> {
                    sprite: texture.into(),
                    layer: depth.to_layer(),
                    anchor: CxAnchor::Center,
                    ..default()
                },
                animation.make_animation_bundle(),
                *depth,
                StageEntity,
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ));
        }

        score.add_u(tardigrade.kill_score());
    }
}

/// @system Fires boulder attacks from idle in-view tardigrades on a cooldown.
///
/// # Panics
///
/// Panics if the camera entity is missing from the world.
#[allow(clippy::too_many_arguments)]
pub fn check_idle_tardigrade(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    boulder_config: Res<BoulderThrowConfig>,
    camera_query: Query<&WorldPos, With<CameraPos>>,
    stage_time: Res<Time<StageTimeDomain>>,
    query: Query<
        (
            Entity,
            &EnemyTardigrade,
            &mut EnemyTardigradeAttacking,
            &WorldPos,
            Option<&CxPresentationTransform>,
            &Depth,
        ),
        With<InView>,
    >,
) {
    let camera_pos = camera_query.single().unwrap();
    for (entity, _enemy, attacking, position, presentation, depth) in &mut query.iter() {
        if attacking.attack {
            // if let EnemyStep::Idle { duration } = enemy.current_step() {
            if attacking.last_attack_started
                < stage_time.elapsed() + Duration::from_secs_f32(ENEMY_TARDIGRADE_ATTACK_SPEED)
            {
                #[cfg(debug_assertions)]
                info!("Tardigrade {:?} is attacking", entity);

                commands
                    .entity(entity)
                    .remove::<EnemyTardigradeAnimation>()
                    .insert(EnemyTardigradeAttacking {
                        attack: true,
                        last_attack_started: stage_time.elapsed(),
                    });

                spawn_boulder_throw_attack(
                    &mut commands,
                    &asset_server,
                    &atlas_assets,
                    &stage_time,
                    &boulder_config,
                    *SCREEN_RESOLUTION_F32_H + camera_pos.0,
                    position.0,
                    presentation,
                    depth,
                );
            }
        }
    }
}
