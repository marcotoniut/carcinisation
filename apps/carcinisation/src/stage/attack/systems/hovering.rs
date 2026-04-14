#![allow(clippy::type_complexity)]

use crate::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::{
    components::{DelayedDespawnOnPxAnimationFinished, DespawnMark},
    stage::{
        attack::components::EnemyHoveringAttackType,
        components::{
            damage::InflictsDamage,
            placement::{AuthoredDepths, Depth, InView},
        },
        messages::DamageMessage,
        player::{components::Player, messages::CameraShakeEvent},
        resources::StageTimeDomain,
    },
};
use assert_assets_path::assert_assets_path;
use bevy::{
    audio::{AudioPlayer, PlaybackMode, PlaybackSettings},
    prelude::*,
};
use carapace::prelude::{PxAnchor, PxAtlasSprite, PxSpriteAtlasAsset, PxSubPosition};
use cween::linear::components::{LinearValueReached, TargetingValueZ};

/// @system Deals damage and spawns a hit animation when a hovering attack reaches its target depth.
pub fn hovering_damage_on_reached(
    mut commands: Commands,
    mut damage_event_writer: MessageWriter<DamageMessage>,
    mut player_query: Query<Entity, With<Player>>,
    atlas_assets: Res<Assets<PxSpriteAtlasAsset>>,
    depth_query: Query<
        (
            Entity,
            &EnemyHoveringAttackType,
            &InflictsDamage,
            &PxSubPosition,
            &Depth,
            Option<&PxAtlasSprite>,
        ),
        (
            Added<LinearValueReached<StageTimeDomain, TargetingValueZ>>,
            With<InView>,
        ),
    >,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    for (entity, attack, damage, position, depth, existing_sprite) in &mut depth_query.iter() {
        let sound_effect: Handle<AudioSource> =
            asset_server.load(assert_assets_path!("audio/sfx/enemy_melee.ogg"));

        for entity in &mut player_query.iter_mut() {
            damage_event_writer.write(DamageMessage::new(entity, damage.0));
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
        use crate::stage::attack::components::bundles::REGION_HIT;
        let atlas_handle = existing_sprite
            .map(|s| s.atlas.clone())
            .unwrap_or_else(|| asset_server.load(attack.atlas_path()));
        let hit_region = atlas_assets
            .get(&atlas_handle)
            .and_then(|a| a.region_id(REGION_HIT))
            .unwrap_or_default();
        let hit_anim = atlas_assets
            .get(&atlas_handle)
            .and_then(|a| a.animation(REGION_HIT))
            .map(|a| {
                crate::pixel::PxAnimationBundle::from_parts(
                    a.px_direction(),
                    a.px_duration(),
                    a.px_finish_behavior(),
                    carapace::prelude::PxFrameTransition::None,
                )
            })
            .unwrap_or_default();

        commands.spawn((
            Name::new(format!("Attack - {} - hit", attack.get_name())),
            PxSubPosition::from(position.0),
            PxAtlasSprite::new(atlas_handle, hit_region),
            hit_anim,
            PxAnchor::Center,
            *depth,
            depth.to_layer(),
            AuthoredDepths::single(Depth::One),
            DelayedDespawnOnPxAnimationFinished::from_secs_f32(0.4),
        ));

        commands.entity(entity).insert(DespawnMark);

        // TODO CameraShake on damage event read instead?
        commands.trigger(CameraShakeEvent);
    }
}
