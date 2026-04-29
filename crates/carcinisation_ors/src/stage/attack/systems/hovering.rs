#![allow(clippy::type_complexity)]

use crate::stage::{
    attack::components::EnemyHoveringAttackType,
    components::{
        StageEntity,
        damage::InflictsDamage,
        interactive::Dead,
        placement::{AuthoredDepths, Depth, InView},
    },
    messages::DamageMessage,
    player::{
        components::{Player, Webbed},
        messages::CameraShakeEvent,
    },
    resources::StageTimeDomain,
};
use assert_assets_path::assert_assets_path;
use bevy::{
    audio::{AudioPlayer, PlaybackMode, PlaybackSettings},
    prelude::*,
};
use carapace::prelude::{CxAnchor, CxAtlasSprite, CxSpriteAtlasAsset, WorldPos};
use carcinisation_core::components::DelayedDespawnOnCxAnimationFinished;
use carcinisation_core::components::DespawnMark;
use carcinisation_core::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use cween::linear::components::{LinearValueReached, TargetingValueZ};

use crate::stage::attack::components::bundles::REGION_HIT;

/// @system Deals damage and spawns a hit animation when a hovering attack reaches its target depth.
pub fn hovering_damage_on_reached(
    mut commands: Commands,
    mut damage_event_writer: MessageWriter<DamageMessage>,
    mut player_query: Query<(Entity, Option<&mut Webbed>), With<Player>>,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    stage_time: Res<Time<StageTimeDomain>>,
    depth_query: Query<
        (
            Entity,
            &EnemyHoveringAttackType,
            &InflictsDamage,
            &WorldPos,
            &Depth,
            Option<&CxAtlasSprite>,
        ),
        (
            Added<LinearValueReached<StageTimeDomain, TargetingValueZ>>,
            With<InView>,
            Without<Dead>,
        ),
    >,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    let is_spider_shot =
        |attack: &EnemyHoveringAttackType| matches!(attack, EnemyHoveringAttackType::SpiderShot);

    for (entity, attack, damage, position, depth, existing_sprite) in &mut depth_query.iter() {
        let sound_effect: Handle<AudioSource> =
            asset_server.load(assert_assets_path!("audio/sfx/enemy_melee.ogg"));

        for (player_entity, ref mut webbed) in &mut player_query.iter_mut() {
            damage_event_writer.write(DamageMessage::new(player_entity, damage.0));

            // Spider shots apply/refresh the Webbed debuff on hit.
            if is_spider_shot(attack) {
                if let Some(webbed) = webbed {
                    webbed.refresh(stage_time.elapsed());
                } else {
                    commands
                        .entity(player_entity)
                        .insert(Webbed::new(stage_time.elapsed()));
                }
            }
        }

        commands.spawn((
            AudioPlayer(sound_effect),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: volume_settings.sfx,
                ..Default::default()
            },
            AudioSystemBundle {
                system_type: AudioSystemType::SFX,
            },
        ));

        // Reuse the atlas handle from the attack's own sprite when available.
        let atlas_handle = existing_sprite.map_or_else(
            || asset_server.load(attack.atlas_path()),
            |s| s.atlas.clone(),
        );
        let hit_region = atlas_assets
            .get(&atlas_handle)
            .and_then(|a| a.region_id(REGION_HIT))
            .unwrap_or_default();
        let hit_anim = atlas_assets
            .get(&atlas_handle)
            .and_then(|a| a.animation(REGION_HIT))
            .map(|a| {
                carapace::prelude::CxAnimationBundle::from_parts(
                    a.px_direction(),
                    a.px_duration(),
                    a.px_finish_behavior(),
                    carapace::prelude::CxFrameTransition::None,
                )
            })
            .unwrap_or_default();

        // Spider shot hit effect stays visible for the full web duration
        // (attached at impact position). Other attacks use a short fade.
        let hit_despawn_delay = if is_spider_shot(attack) {
            DelayedDespawnOnCxAnimationFinished::from_secs_f32(3.0)
        } else {
            DelayedDespawnOnCxAnimationFinished::from_secs_f32(0.4)
        };

        commands.spawn((
            Name::new(format!("Attack - {} - hit", attack.get_name())),
            WorldPos::from(position.0),
            CxAtlasSprite::new(atlas_handle, hit_region),
            hit_anim,
            CxAnchor::Center,
            *depth,
            depth.to_layer(),
            AuthoredDepths::single(Depth::One),
            hit_despawn_delay,
            StageEntity,
        ));

        commands.entity(entity).insert(DespawnMark);

        // TODO CameraShake on damage event read instead?
        commands.trigger(CameraShakeEvent);
    }
}
