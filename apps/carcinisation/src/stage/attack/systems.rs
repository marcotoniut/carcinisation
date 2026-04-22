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
        messages::DepthChangedMessage,
        player::components::PLAYER_DEPTH,
        resources::StageTimeDomain,
    },
};
use bevy::prelude::*;
use carapace::prelude::{CxAnchor, CxAtlasSprite, CxSpriteAtlasAsset, WorldPos};
use cween::linear::components::{LinearValueReached, TargetingValueZ};

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
            let animation_bundle = crate::pixel::CxAnimationBundle::from_parts(
                anim.px_direction(),
                anim.px_duration(),
                anim.px_finish_behavior(),
                carapace::prelude::CxFrameTransition::None,
            );
            commands.spawn((
                Name::new(format!("Attack - {} - destroy", attack_type.get_name())),
                WorldPos::from(position.0),
                destroy_sprite,
                animation_bundle,
                CxAnchor::Center,
                *depth,
                depth.to_layer(),
                AuthoredDepths::single(Depth::One),
                DelayedDespawnOnCxAnimationFinished::from_secs_f32(0.2),
                StageEntity,
            ));
        }
        commands.entity(entity).insert(DespawnMark);
    }
}

#[cfg(test)]
mod tests {
    // Attack runtime tests live with the specific spawn/motion modules.
}
