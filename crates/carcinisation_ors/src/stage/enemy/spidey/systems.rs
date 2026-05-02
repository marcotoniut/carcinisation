use super::entity::{EnemySpidey, EnemySpideyAnimation, EnemySpideyAttacking};
use crate::stage::{
    attack::{data::spider_shot::SpiderShotConfig, spawns::spider_shot::spawn_spider_shot_attack},
    components::{
        interactive::{BurningCorpse, Dead},
        placement::{Depth, InView},
    },
    depth_scale::DepthScaleConfig,
    enemy::{
        components::{
            behavior::{EnemyCurrentBehavior, JumpTween},
            composed_state::Dying,
        },
        composed::{ComposedAnimationState, ComposedPartStates, ComposedResolvedParts},
        data::{
            spidey::{
                TAG_IDLE, TAG_JUMP, TAG_LANDING, TAG_LOUNGE, TAG_SHOOT,
                apply_spidey_animation_state_with_hold,
            },
            steps::{EnemyStep, JumpEnemyStep},
        },
    },
    messages::ComposedAnimationCueMessage,
    player::components::Player,
    resources::StageTimeDomain,
};
use crate::stubs::Score;
use bevy::prelude::*;
use carapace::prelude::{CxPresentationTransform, CxSpriteAtlasAsset, WorldPos};
use carcinisation_core::components::DespawnMark;
use std::time::Duration;

/// Cooldown between consecutive spider shot attacks.
pub const ENEMY_SPIDEY_ATTACK_COOLDOWN: Duration = Duration::from_millis(4000);

/// How long the shoot animation plays before the attack state clears.
/// 7 frames * 200ms = 1400ms.
pub const ENEMY_SPIDEY_RANGED_PRESENTATION: Duration = Duration::from_millis(1400);

/// Event ID authored in the composed RON `shoot` animation.
const SPIDEY_SPIDER_SHOT_EVENT_ID: &str = "spider_shot";

fn spidey_attack_cooldown_ready(cooldown_anchor: Duration, stage_elapsed: Duration) -> bool {
    stage_elapsed >= cooldown_anchor + ENEMY_SPIDEY_ATTACK_COOLDOWN
}

fn spidey_attack_presentation_finished(
    last_attack_started: Duration,
    stage_elapsed: Duration,
) -> bool {
    stage_elapsed >= last_attack_started + ENEMY_SPIDEY_RANGED_PRESENTATION
}

/// Selects the animation tag for a spidey based on its current behavior.
///
/// Dead spideys are excluded to prevent overriding death animations.
pub fn assign_spidey_animation(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            Option<&EnemySpideyAttacking>,
            Option<&EnemySpideyAnimation>,
            Option<&JumpTween>,
            &mut ComposedAnimationState,
        ),
        (With<EnemySpidey>, Without<Dead>, Without<BurningCorpse>),
    >,
) {
    for (entity, behavior, attacking, current_animation, jump_tween, mut animation_state) in
        &mut query
    {
        let (next_animation, next_tag, hold_last_frame) = if attacking.is_some_and(|a| a.active) {
            (EnemySpideyAnimation::Shoot, TAG_SHOOT, false)
        } else {
            match behavior.behavior {
                EnemyStep::Jump(JumpEnemyStep { .. }) => {
                    let jump_progress =
                        jump_tween.map_or(0.0, |jump| jump.progress_at(stage_time.elapsed()));
                    if jump_progress < 0.5 {
                        (EnemySpideyAnimation::Jump, TAG_JUMP, true)
                    } else {
                        (EnemySpideyAnimation::Landing, TAG_LANDING, false)
                    }
                }
                EnemyStep::Idle { .. } => (EnemySpideyAnimation::Idle, TAG_IDLE, false),
                EnemyStep::Attack { .. }
                | EnemyStep::Circle { .. }
                | EnemyStep::LinearTween { .. } => {
                    (EnemySpideyAnimation::Lounge, TAG_LOUNGE, false)
                }
            }
        };

        if current_animation != Some(&next_animation) {
            commands.entity(entity).insert(next_animation);
        }
        apply_spidey_animation_state_with_hold(&mut animation_state, next_tag, hold_last_frame);
    }
}

/// Awards score and begins the death effect when a spidey takes lethal damage.
pub fn despawn_dead_spideys(
    mut commands: Commands,
    mut score: ResMut<Score>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (
            Entity,
            &EnemySpidey,
            &mut ComposedAnimationState,
            &mut ComposedPartStates,
        ),
        (Added<Dead>, Without<BurningCorpse>),
    >,
) {
    for (entity, spidey, _animation_state, mut part_states) in &mut query {
        info!("Spidey {:?} died - beginning death effect", entity);

        // Clear all active hit blinks to prevent flickering during death.
        for (_part_id, part_state) in part_states.iter_mut() {
            part_state.hit_blink = None;
        }

        commands.entity(entity).insert(Dying {
            started: stage_time.elapsed(),
        });

        score.add_u(spidey.kill_score());
    }
}

/// Applies a progressive pixel-disappearing effect to dying spideys.
///
/// Reuses the same visual pattern as the mosquiton death effect: parts
/// flicker with increasing frequency before being fully hidden, then the
/// entity is despawned.
///
/// # Panics
///
/// Panics if `stage_time.elapsed()` is less than `dying.started`, which should
/// never happen in normal operation as the death timestamp is set from the same clock.
pub fn update_spidey_death_effect(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &Dying, &mut ComposedPartStates), With<EnemySpidey>>,
) {
    use std::time::Duration;

    const DEATH_DURATION: Duration = Duration::from_millis(750);

    for (entity, dying, mut part_states) in &mut query {
        let elapsed = stage_time.elapsed().checked_sub(dying.started).unwrap();

        if elapsed >= DEATH_DURATION {
            info!("Spidey {:?} death effect complete - despawning", entity);
            commands.entity(entity).insert(DespawnMark);
            continue;
        }

        for (_part_id, part_state) in part_states.iter_mut() {
            let part_progress = elapsed.as_secs_f32() / DEATH_DURATION.as_secs_f32();

            if part_progress >= 1.0 {
                part_state.visible = false;
            } else if part_progress >= 0.9 {
                // Fast flicker - visible only 25% of the time.
                part_state.visible = (elapsed.as_millis() / 50).is_multiple_of(4);
            } else if part_progress >= 0.7 {
                // Medium flicker - visible 50% of the time.
                part_state.visible = (elapsed.as_millis() / 75).is_multiple_of(2);
            } else if part_progress >= 0.4 {
                // Slow flicker - visible 75% of the time.
                part_state.visible = (elapsed.as_millis() / 100) % 4 != 3;
            }
            // else: fully visible (part_progress < 0.4)
        }
    }
}

/// Clears the active attack flag once the shoot presentation window expires.
pub fn clear_finished_spidey_attacks(
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        &mut EnemySpideyAttacking,
        (With<EnemySpidey>, Without<Dead>, Without<BurningCorpse>),
    >,
) {
    for mut attacking in &mut query {
        if !attacking.active
            || !spidey_attack_presentation_finished(
                attacking.last_attack_started,
                stage_time.elapsed(),
            )
        {
            continue;
        }

        attacking.active = false;
    }
}

/// Fires ranged attacks from idle in-view spideys on a cooldown.
pub fn check_idle_spidey(
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (&EnemyCurrentBehavior, &mut EnemySpideyAttacking),
        (
            With<InView>,
            With<EnemySpidey>,
            Without<Dead>,
            Without<BurningCorpse>,
        ),
    >,
) {
    for (behavior, mut attacking) in &mut query {
        if attacking.active
            || !matches!(
                behavior.behavior,
                EnemyStep::Idle { .. } | EnemyStep::Attack { .. } | EnemyStep::Circle { .. }
            )
        {
            continue;
        }

        let cooldown_anchor = attacking.last_attack_started.max(behavior.started);
        if !spidey_attack_cooldown_ready(cooldown_anchor, stage_time.elapsed()) {
            continue;
        }

        attacking.active = true;
        attacking.last_attack_started = stage_time.elapsed();
    }
}

/// Consumes composed-animation cues and spawns spider shot projectiles.
#[allow(clippy::too_many_arguments)]
pub fn trigger_spidey_authored_attack_cues(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    spider_shot_config: Res<SpiderShotConfig>,
    depth_scale_config: Res<DepthScaleConfig>,
    player_query: Query<&WorldPos, With<Player>>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut cue_reader: MessageReader<ComposedAnimationCueMessage>,
    query: Query<
        (
            &EnemySpidey,
            &WorldPos,
            Option<&CxPresentationTransform>,
            &Depth,
            Option<&ComposedResolvedParts>,
        ),
        (Without<Dead>, Without<BurningCorpse>),
    >,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    for cue in cue_reader.read() {
        if cue.id != SPIDEY_SPIDER_SHOT_EVENT_ID
            || cue.kind != asset_pipeline::aseprite::AnimationEventKind::ProjectileSpawn
        {
            continue;
        }

        let Ok((_spidey, position, presentation, depth, resolved_parts)) = query.get(cue.entity)
        else {
            continue;
        };

        let scaled_offset = resolved_parts.map_or(
            Vec2::ZERO,
            super::super::composed::ComposedResolvedParts::scaled_visual_offset,
        );

        let resolved_part = cue.part_id.as_deref().and_then(|part_id| {
            resolved_parts
                .and_then(|parts| parts.parts().iter().find(|part| part.part_id == part_id))
        });

        let local_offset = IVec2::new(cue.local_offset.x, cue.local_offset.y);

        let current_pos = resolved_part.map_or(position.0 + scaled_offset, |part| {
            part.visual_point_from_local_offset(local_offset, scaled_offset)
        });

        #[cfg(debug_assertions)]
        {
            let method = if resolved_part.is_some() {
                "resolved_part"
            } else {
                "FALLBACK_entity_visual_center"
            };
            let entity_pos = position.0;
            let cue_part_id = &cue.part_id;
            let off_x = cue.local_offset.x;
            let off_y = cue.local_offset.y;
            info!(
                "Spider shot cue: method={method}, origin={current_pos:?}, entity_pos={entity_pos:?}, \
                 part_id={cue_part_id:?}, offset=({off_x},{off_y})",
            );
        }

        spawn_spider_shot_attack(
            &mut commands,
            &asset_server,
            &atlas_assets,
            &stage_time,
            &spider_shot_config,
            &depth_scale_config,
            player_pos.0,
            current_pos,
            presentation,
            depth,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::enemy::data::spidey::{TAG_JUMP, TAG_LANDING};
    use std::time::Duration;

    #[test]
    fn spidey_attack_cooldown_gates_attack() {
        assert!(!spidey_attack_cooldown_ready(
            Duration::ZERO,
            Duration::ZERO
        ));
        assert!(spidey_attack_cooldown_ready(
            Duration::ZERO,
            ENEMY_SPIDEY_ATTACK_COOLDOWN
        ));
    }

    #[test]
    fn spidey_presentation_window_clears() {
        let started = Duration::from_secs(1);
        assert!(!spidey_attack_presentation_finished(started, started));
        assert!(spidey_attack_presentation_finished(
            started,
            started + ENEMY_SPIDEY_RANGED_PRESENTATION
        ));
    }

    #[test]
    fn jump_animation_holds_terminal_frame_then_switches_to_landing() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.add_systems(Update, assign_spidey_animation);

        let entity = app
            .world_mut()
            .spawn((
                EnemySpidey,
                EnemyCurrentBehavior {
                    started: Duration::ZERO,
                    behavior: EnemyStep::Jump(JumpEnemyStep::base()),
                },
                JumpTween::new(Duration::ZERO, 1.0, false),
                ComposedAnimationState::new(TAG_IDLE),
            ))
            .id();

        app.update();
        {
            let world = app.world();
            let state = world
                .entity(entity)
                .get::<ComposedAnimationState>()
                .expect("spidey should have animation state");
            let animation = world
                .entity(entity)
                .get::<EnemySpideyAnimation>()
                .expect("spidey semantic animation should be assigned");
            assert_eq!(state.requested_tag, TAG_JUMP);
            assert!(state.hold_last_frame);
            assert_eq!(*animation, EnemySpideyAnimation::Jump);
        }

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(600));
        app.update();

        let world = app.world();
        let state = world
            .entity(entity)
            .get::<ComposedAnimationState>()
            .expect("spidey should have animation state");
        let animation = world
            .entity(entity)
            .get::<EnemySpideyAnimation>()
            .expect("spidey semantic animation should be assigned");
        assert_eq!(state.requested_tag, TAG_LANDING);
        assert!(!state.hold_last_frame);
        assert_eq!(*animation, EnemySpideyAnimation::Landing);
    }
}
