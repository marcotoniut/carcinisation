pub mod hovering;
pub mod player;

#[cfg(debug_assertions)]
use super::components::EnemyAttackDebugPosition;
use super::components::{AttachedToComposedPart, EnemyAttack, EnemyHoveringAttackType};
use crate::{
    components::{DelayedDespawnOnPxAnimationFinished, DespawnMark},
    layer::Layer,
    stage::{
        components::{
            interactive::{Dead, Health},
            placement::{AuthoredDepths, Depth, InView},
        },
        enemy::composed::ComposedResolvedParts,
        messages::DepthChangedMessage,
        player::components::PLAYER_DEPTH,
        resources::StageTimeDomain,
    },
};
use bevy::prelude::*;
use carapace::prelude::{PxAnchor, PxAtlasSprite, PxSpriteAtlasAsset, PxSubPosition};
use cween::linear::components::{
    LinearValueReached, TargetingValueX, TargetingValueY, TargetingValueZ,
};

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
    mut query: Query<(&PxSubPosition, &mut EnemyAttackDebugPosition), With<EnemyAttack>>,
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
    atlas_assets: Res<Assets<PxSpriteAtlasAsset>>,
    query: Query<
        (
            Entity,
            Option<&EnemyHoveringAttackType>,
            &PxSubPosition,
            &Depth,
            Option<&PxAtlasSprite>,
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
            let destroy_sprite = PxAtlasSprite::new(sprite.atlas.clone(), destroy_region);
            let animation_bundle = crate::pixel::PxAnimationBundle::from_parts(
                anim.px_direction(),
                anim.px_duration(),
                anim.px_finish_behavior(),
                carapace::prelude::PxFrameTransition::None,
            );
            commands.spawn((
                Name::new(format!("Attack - {} - destroy", attack_type.get_name())),
                PxSubPosition::from(position.0),
                destroy_sprite,
                animation_bundle,
                PxAnchor::Center,
                *depth,
                depth.to_layer(),
                AuthoredDepths::single(Depth::One),
                DelayedDespawnOnPxAnimationFinished::from_secs_f32(0.2),
            ));
        }
        commands.entity(entity).insert(DespawnMark);
    }
}

/// @system Keeps attacks with [`AttachedToComposedPart`] locked to their source
/// part's visual position each frame. Syncs `TargetingValueX/Y` so tween start
/// values are consistent when the attachment is removed and travel begins.
pub fn update_attached_attack_positions(
    composed_query: Query<&ComposedResolvedParts>,
    mut attack_query: Query<(
        &AttachedToComposedPart,
        &mut PxSubPosition,
        &mut TargetingValueX,
        &mut TargetingValueY,
    )>,
) {
    for (attached, mut position, mut tx, mut ty) in &mut attack_query {
        let Ok(resolved_parts) = composed_query.get(attached.source_entity) else {
            // Source entity gone (despawned/dead) — hold at current position
            // until arm_pending_blood_shot_motion fires and detaches.
            continue;
        };
        let visual_offset = resolved_parts.visual_offset();
        let Some(part) = resolved_parts
            .parts()
            .iter()
            .find(|p| p.part_id == attached.part_id)
        else {
            continue;
        };
        let visual_pos = part.visual_point_from_local_offset(attached.local_offset, visual_offset);
        position.0 = visual_pos;
        tx.0 = visual_pos.x;
        ty.0 = visual_pos.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::enemy::composed::{ComposedResolvedParts, ResolvedPartState};

    #[test]
    fn attachment_tracks_composed_part_visual_position() {
        let mut app = App::new();
        app.add_systems(Update, update_attached_attack_positions);

        let visual_offset = Vec2::new(0.0, 49.0);
        let head_part = ResolvedPartState {
            part_id: "head".to_string(),
            parent_id: None,
            draw_order: 30,
            sprite_id: "s".to_string(),
            frame_size: UVec2::new(6, 16),
            flip_x: false,
            flip_y: false,
            part_pivot: IVec2::ZERO,
            world_top_left_position: Vec2::new(35.0, 513.0),
            world_pivot_position: Vec2::new(35.0, 513.0),
            tags: vec![],
            targetable: false,
            health_pool: None,
            armour: 0,
            current_durability: None,
            max_durability: None,
            breakable: false,
            broken: false,
            blinking: false,
            collisions: vec![],
        };

        let source = app
            .world_mut()
            .spawn(ComposedResolvedParts::with_parts_and_offset(
                vec![head_part],
                visual_offset,
            ))
            .id();

        let attack = app
            .world_mut()
            .spawn((
                AttachedToComposedPart {
                    source_entity: source,
                    part_id: "head".to_string(),
                    local_offset: IVec2::new(6, 9),
                },
                PxSubPosition(Vec2::ZERO),
                TargetingValueX(0.0),
                TargetingValueY(0.0),
            ))
            .id();

        app.update();

        let world = app.world();
        let pos = world.entity(attack).get::<PxSubPosition>().unwrap();
        let tx = world.entity(attack).get::<TargetingValueX>().unwrap();
        let ty = world.entity(attack).get::<TargetingValueY>().unwrap();

        // game_logic = (35+6, 513-9) = (41, 504)
        // visual = (41, 504+49) = (41, 553)
        assert_eq!(pos.0, Vec2::new(41.0, 553.0));
        assert_eq!(tx.0, 41.0);
        assert_eq!(ty.0, 553.0);
    }
}
