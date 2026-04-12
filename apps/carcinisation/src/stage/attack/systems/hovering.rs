#![allow(clippy::type_complexity)]

use crate::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::stage::attack::components::bundles::make_hit_atlas_bundle;
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
use carapace::prelude::{PxAnchor, PxSubPosition};
use cween::linear::components::{LinearValueReached, TargetingValueZ};

/// @system Deals damage and spawns a hit animation when a hovering attack reaches its target depth.
pub fn hovering_damage_on_reached(
    mut commands: Commands,
    mut damage_event_writer: MessageWriter<DamageMessage>,
    mut player_query: Query<Entity, With<Player>>,
    asset_server: Res<AssetServer>,
    depth_query: Query<
        (
            Entity,
            &EnemyHoveringAttackType,
            &InflictsDamage,
            &PxSubPosition,
            &Depth,
        ),
        (
            Added<LinearValueReached<StageTimeDomain, TargetingValueZ>>,
            With<InView>,
        ),
    >,
    volume_settings: Res<VolumeSettings>,
) {
    for (entity, attack, damage, position, depth) in &mut depth_query.iter() {
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

        let (atlas_sprite, animation_bundle) = make_hit_atlas_bundle(&asset_server, attack);

        commands.spawn((
            Name::new(format!("Attack - {} - hit", attack.get_name())),
            PxSubPosition::from(position.0),
            atlas_sprite,
            animation_bundle,
            PxAnchor::Center,
            depth.to_layer(),
            AuthoredDepths::single(Depth::One),
            DelayedDespawnOnPxAnimationFinished::from_secs_f32(0.4),
        ));

        commands.entity(entity).insert(DespawnMark);

        // TODO CameraShake on damage event read instead?
        commands.trigger(CameraShakeEvent);
    }
}
