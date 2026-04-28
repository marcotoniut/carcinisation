pub mod hovering;
pub mod player;

#[cfg(debug_assertions)]
use super::components::EnemyAttackDebugPosition;
use super::components::{EnemyAttack, EnemyHoveringAttackType};
use crate::{
    components::{DelayedDespawnOnCxAnimationFinished, DespawnMark},
    layer::Layer,
    stage::{
        components::{
            StageEntity,
            interactive::{Dead, Health},
            placement::{AuthoredDepths, Depth, InView},
        },
        depth_scale::DepthScaleConfig,
        messages::DepthChangedMessage,
        player::components::PLAYER_DEPTH,
        resources::StageTimeDomain,
    },
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxAtlasSprite, CxPosition, CxPresentationTransform, CxSpriteAtlasAsset, WorldPos,
};
use cween::linear::components::{LinearValueReached, TargetingValueZ, TweenChild};

/// @system Marks entities as `Dead` when their health reaches zero.
// TODO remove in favor of damage taken?
pub fn check_health_at_0(mut commands: Commands, query: Query<(Entity, &Health), Without<Dead>>) {
    for (entity, health) in &mut query.iter() {
        if health.0 == 0 {
            commands.entity(entity).insert(Dead);
        }
    }
}

/// Mirrors non-reflectable attack position state into a reflectable debug
/// component so BRP can inspect exact projectile centers in live gameplay.
#[cfg(debug_assertions)]
pub fn sync_enemy_attack_debug_positions(
    mut query: Query<(&WorldPos, &mut EnemyAttackDebugPosition), With<EnemyAttack>>,
) {
    for (position, mut debug_position) in &mut query {
        debug_position.current = position.0;
    }
}

/// @system Despawns enemy attacks that reached their target depth while off-screen.
pub fn miss_on_reached(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            Added<LinearValueReached<StageTimeDomain, TargetingValueZ>>,
            With<EnemyAttack>,
            Without<InView>,
        ),
    >,
) {
    for entity in &mut query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}

/// @system Updates the hovering-attack render layer when its depth layer changes.
///
/// With the atlas-based approach, visual scaling is handled automatically by
/// `apply_depth_fallback_scale` via `AuthoredDepths`. This system only needs
/// to update the render layer so the entity draws on the correct depth plane.
pub fn on_enemy_attack_depth_changed(
    mut commands: Commands,
    mut event_reader: MessageReader<DepthChangedMessage>,
    query: Query<Entity, (With<EnemyHoveringAttackType>, Without<Dead>)>,
) {
    for event in event_reader.read() {
        if event.depth > PLAYER_DEPTH && query.contains(event.entity) {
            let layer: Layer = (event.depth - 1).to_layer();
            commands.entity(event.entity).insert(layer);
        }
    }
}

/// @system Spawns a destroy animation when a hovering attack is killed by the player,
/// then despawns the attack entity. Falls back to immediate despawn if no destroy
/// animation is authored.
pub fn despawn_dead_attacks(
    mut commands: Commands,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    depth_scale_config: Res<DepthScaleConfig>,
    tween_children: Query<(Entity, &ChildOf), With<TweenChild>>,
    query: Query<
        (
            Entity,
            Option<&EnemyHoveringAttackType>,
            &WorldPos,
            &Depth,
            Option<&CxAtlasSprite>,
        ),
        (Added<Dead>, With<EnemyAttack>),
    >,
) {
    for (entity, attack_type, position, depth, existing_sprite) in query.iter() {
        // Immediately despawn tween children so the depth/position tweens
        // stop advancing between now and the PostUpdate despawn pass.
        for (child_entity, child_of) in &tween_children {
            if child_of.0 == entity {
                commands.entity(child_entity).try_despawn();
            }
        }

        // Remove the sprite so the hover visual does not render alongside the
        // destroy animation on the death frame.  DespawnMark cleanup runs in
        // PostUpdate, but carapace extraction may happen first; removing the
        // sprite component is more reliable than Visibility::Hidden because it
        // does not depend on visibility-propagation ordering.
        commands.entity(entity).remove::<CxAtlasSprite>();

        // Reuse the atlas handle from the attack's own sprite — guaranteed loaded
        // since the attack was already rendering its hover animation.
        if let Some(attack_type) = attack_type
            && let Some(sprite) = existing_sprite
            && let Some(atlas) = atlas_assets.get(&sprite.atlas)
            && let Some(destroy_region) =
                atlas.region_id(super::components::bundles::REGION_DESTROY)
            && let Some(anim) = atlas.animation(super::components::bundles::REGION_DESTROY)
        {
            let destroy_sprite = CxAtlasSprite::new(sprite.atlas.clone(), destroy_region);
            let animation_bundle = carapace::prelude::CxAnimationBundle::from_parts(
                anim.px_direction(),
                anim.px_duration(),
                anim.px_finish_behavior(),
                carapace::prelude::CxFrameTransition::None,
            );
            let authored_depths = AuthoredDepths::single(Depth::One);
            let mut destroy_entity = commands.spawn((
                Name::new(format!("Attack - {} - destroy", attack_type.get_name())),
                WorldPos::from(position.0),
                CxPosition::from(position.0.round().as_ivec2()),
                destroy_sprite,
                animation_bundle,
                CxAnchor::Center,
                *depth,
                depth.to_layer(),
                authored_depths.clone(),
                DelayedDespawnOnCxAnimationFinished::from_secs_f32(0.2),
                StageEntity,
            ));
            // Pre-compute depth-fallback scale so the destroy animation renders
            // at the correct size on its first visible frame.  Both components
            // must be inserted together to prevent apply_depth_fallback_scale
            // from double-applying the ratio on the next frame.
            let ratio = depth_scale_config.resolve_fallback(*depth, &authored_depths);
            if (ratio - 1.0).abs() >= f32::EPSILON {
                let fallback = Vec2::splat(ratio);
                destroy_entity.insert((
                    CxPresentationTransform {
                        scale: fallback,
                        ..default()
                    },
                    crate::stage::depth_scale::DepthFallbackScale(fallback),
                ));
            }
        }
        commands.entity(entity).insert(DespawnMark);
    }
}

#[cfg(test)]
mod tests {
    // Attack runtime tests live with the specific spawn/motion modules.
}
