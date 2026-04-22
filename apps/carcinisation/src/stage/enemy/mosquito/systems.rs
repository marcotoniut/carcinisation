use super::entity::{
    EnemyMosquito, EnemyMosquitoAnimation, EnemyMosquitoAttack, EnemyMosquitoAttacking,
};
use crate::pixel::{CxAnimationBundle, CxAssets, CxSpriteBundle};
use crate::stage::enemy::composed::ComposedAnimationState;
use crate::stage::enemy::mosquiton::entity::EnemyMosquiton;
use crate::{
    components::{DelayedDespawnOnCxAnimationFinished, DespawnMark},
    game::score::components::Score,
    globals::SCREEN_RESOLUTION_F32_H,
    layer::Layer,
    stage::{
        attack::{data::blood_shot::BloodShotConfig, spawns::blood_shot::spawn_blood_shot_attack},
        components::{
            StageEntity,
            interactive::Dead,
            placement::{Depth, InView},
        },
        enemy::{
            bundles::make_enemy_animation_bundle,
            components::behavior::EnemyCurrentBehavior,
            data::{
                mosquito::MOSQUITO_ANIMATIONS,
                steps::{EnemyStep, JumpEnemyStep},
            },
        },
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxAnimationDuration, CxAnimationFinishBehavior, CxPresentationTransform, CxSprite,
    CxSpriteAtlasAsset, WorldPos,
};

use crate::stage::parallax::ParallaxOffset;
use std::time::Duration;

pub const ENEMY_MOSQUITO_ATTACK_SPEED: f32 = 3.;
/// Keep the transient ranged-attack marker alive for the full authored
/// Mosquiton `shoot_fly` clip.
///
/// The attack state is the only gameplay-side signal that tells composed
/// enemies to request `shoot_fly`. If we clear it before the authored clip can
/// read, the shot technically fires but the action is barely visible. If we
/// never clear it, the enemy gets stuck in attack presentation. This duration
/// must therefore stay long enough for a readable shot while still remaining
/// shorter than the 3s attack cadence.
pub const ENEMY_MOSQUITO_RANGED_PRESENTATION: Duration = Duration::from_millis(1400);
const ENEMY_MOSQUITO_DEATH_LINGER: Duration = Duration::from_secs(2);

fn mosquito_attack_cooldown_ready(cooldown_anchor: Duration, stage_elapsed: Duration) -> bool {
    stage_elapsed >= cooldown_anchor + Duration::from_secs_f32(ENEMY_MOSQUITO_ATTACK_SPEED)
}

fn mosquito_attack_presentation_finished(
    last_attack_started: Duration,
    stage_elapsed: Duration,
) -> bool {
    stage_elapsed >= last_attack_started + ENEMY_MOSQUITO_RANGED_PRESENTATION
}

/// @system Picks the correct mosquito sprite for the current behavior and depth.
pub fn assign_mosquito_animation(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            &WorldPos,
            &EnemyMosquitoAttacking,
            &Depth,
        ),
        (
            With<EnemyMosquito>,
            Without<EnemyMosquitoAnimation>,
            Without<EnemyMosquiton>,
        ),
    >,
    mut assets_sprite: CxAssets<CxSprite>,
) {
    for (entity, behavior, position, attacking, depth) in &mut query.iter() {
        let step = behavior.behavior;

        let bundle_o = if let Some(attack) = &attacking.attack {
            match attack {
                EnemyMosquitoAttack::Melee => {
                    let animation_o = MOSQUITO_ANIMATIONS.melee_attack.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyMosquitoAttack::Ranged => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
            }
        } else {
            match step {
                EnemyStep::Attack { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::Circle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::Idle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Idle,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::LinearTween { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::Jump(JumpEnemyStep { .. }) => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
            }
        };

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

/// @system Spawns a death animation and awards score when a mosquito dies.
pub fn despawn_dead_mosquitoes(
    mut commands: Commands,
    assets_sprite: CxAssets<CxSprite>,
    mut score: ResMut<Score>,
    query: Query<
        (Entity, &EnemyMosquito, &WorldPos, &Depth),
        (Added<Dead>, Without<EnemyMosquiton>),
    >,
) {
    for (entity, mosquito, position, depth) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

        let animation_o = MOSQUITO_ANIMATIONS.death.get(depth);

        if let Some(animation) = animation_o {
            let texture =
                assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

            // Death visuals should remain visible after the final frame rather
            // than immediately despawning with the source animation.
            commands.spawn((
                Name::new("Dead - Mosquito"),
                WorldPos::from(position.0),
                CxSpriteBundle::<Layer> {
                    sprite: texture.into(),
                    layer: depth.to_layer(),
                    anchor: CxAnchor::Center,
                    ..default()
                },
                CxAnimationBundle::from_parts(
                    animation.direction,
                    CxAnimationDuration::millis_per_animation(animation.speed),
                    CxAnimationFinishBehavior::Mark,
                    animation.frame_transition,
                ),
                DelayedDespawnOnCxAnimationFinished(ENEMY_MOSQUITO_DEATH_LINGER),
                StageEntity,
                *depth,
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ));
        }

        score.add_u(mosquito.kill_score());
    }
}

/// @system Clears short-lived mosquito attack presentation state once the shot
/// has been visible long enough.
///
/// The idle behaviour remains active while ranged attacks are fired. Attack
/// state is therefore a transient visual/action marker, not a long-lived
/// behaviour state. Clearing it restores idle visuals after the shot.
pub fn clear_finished_mosquito_attacks(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &mut EnemyMosquitoAttacking), (With<EnemyMosquito>, Without<Dead>)>,
) {
    for (entity, mut attacking) in &mut query {
        if attacking.attack.is_none()
            || !mosquito_attack_presentation_finished(
                attacking.last_attack_started,
                stage_time.elapsed(),
            )
        {
            continue;
        }

        attacking.attack = None;
        commands.entity(entity).remove::<EnemyMosquitoAnimation>();
    }
}

/// @system Fires ranged attacks from idle in-view mosquitoes on a cooldown.
///
/// # Panics
///
/// Panics if the camera entity is missing from the world.
pub fn check_idle_mosquito(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    blood_shot_config: Res<BloodShotConfig>,
    camera_query: Query<&WorldPos, With<CameraPos>>,
    // TODO
    // event_writer: MessageWriter<BloodAttackEvent>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            &mut EnemyMosquitoAttacking,
            Option<&ComposedAnimationState>,
            Option<&EnemyMosquiton>,
            &WorldPos,
            &Depth,
        ),
        (With<InView>, With<EnemyMosquito>),
    >,
) {
    let camera_pos = camera_query.single().unwrap();
    for (entity, behavior, attacking, composed_animation, mosquiton, position, depth) in &mut query
    {
        if attacking.attack.is_some()
            || !matches!(
                behavior.behavior,
                EnemyStep::Idle { .. } | EnemyStep::Attack { .. } | EnemyStep::Circle { .. }
            )
        {
            continue;
        }

        // Mosquitons beyond max ranged depth are restricted from blood shots.
        if mosquiton.is_some() && !EnemyMosquiton::can_ranged_attack(depth) {
            continue;
        }
        let cooldown_anchor = attacking.last_attack_started.max(behavior.started);
        if !mosquito_attack_cooldown_ready(cooldown_anchor, stage_time.elapsed()) {
            continue;
        }

        #[cfg(debug_assertions)]
        info!("Mosquito {:?} is attacking", entity);

        commands
            .entity(entity)
            .remove::<EnemyMosquitoAnimation>()
            .insert(EnemyMosquitoAttacking {
                attack: Some(EnemyMosquitoAttack::Ranged),
                last_attack_started: stage_time.elapsed(),
            });

        // Composed enemies own projectile timing through authored animation
        // cues. Legacy atlas-strip mosquitoes still spawn immediately here.
        if composed_animation.is_none() {
            spawn_blood_shot_attack(
                &mut commands,
                &asset_server,
                &atlas_assets,
                &stage_time,
                &blood_shot_config,
                *SCREEN_RESOLUTION_F32_H + camera_pos.0,
                position.0,
                depth,
                1.0, // Legacy mosquitoes have per-depth sprites, no fallback scaling.
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mosquito_attack_cooldown_ready;
    use std::time::Duration;

    #[test]
    fn mosquito_attack_waits_for_full_cooldown() {
        let cooldown = Duration::from_secs_f32(super::ENEMY_MOSQUITO_ATTACK_SPEED);
        assert!(!mosquito_attack_cooldown_ready(
            Duration::ZERO,
            cooldown.checked_sub(Duration::from_millis(1)).unwrap()
        ));
        assert!(mosquito_attack_cooldown_ready(Duration::ZERO, cooldown));
    }

    #[test]
    fn mosquito_attack_uses_last_attack_timestamp_as_absolute_time() {
        let last_attack_started = Duration::from_secs(5);
        assert!(!mosquito_attack_cooldown_ready(
            last_attack_started,
            (last_attack_started + Duration::from_secs_f32(super::ENEMY_MOSQUITO_ATTACK_SPEED))
                .checked_sub(Duration::from_millis(1))
                .unwrap(),
        ));
        assert!(mosquito_attack_cooldown_ready(
            last_attack_started,
            last_attack_started + Duration::from_secs_f32(super::ENEMY_MOSQUITO_ATTACK_SPEED),
        ));
    }

    #[test]
    fn mosquito_attack_waits_for_idle_entry_even_when_stage_elapsed_is_large() {
        let idle_started = Duration::from_secs(29);
        let stage_elapsed = Duration::from_secs(30);

        assert!(!mosquito_attack_cooldown_ready(idle_started, stage_elapsed));
        assert!(mosquito_attack_cooldown_ready(
            idle_started,
            idle_started + Duration::from_secs_f32(super::ENEMY_MOSQUITO_ATTACK_SPEED),
        ));
    }

    #[test]
    fn mosquito_ranged_presentation_clears_after_short_hold() {
        let attack_started = Duration::from_secs(3);
        let presentation_duration = super::ENEMY_MOSQUITO_RANGED_PRESENTATION;

        assert!(!super::mosquito_attack_presentation_finished(
            attack_started,
            (attack_started + presentation_duration)
                .checked_sub(Duration::from_nanos(1))
                .unwrap(),
        ));
        assert!(super::mosquito_attack_presentation_finished(
            attack_started,
            attack_started + presentation_duration,
        ));
    }

    #[test]
    fn mosquito_death_linger_is_at_least_two_seconds() {
        assert!(super::ENEMY_MOSQUITO_DEATH_LINGER >= Duration::from_secs(2));
    }
}
