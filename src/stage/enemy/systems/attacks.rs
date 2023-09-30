use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    components::DespawnMark,
    stage::{
        components::*,
        enemy::{components::EnemyAttack, data::blood_attack::BLOOD_ATTACK_ANIMATIONS},
        events::DamageEvent,
        player::{components::Player, events::CameraShakeEvent},
    },
    systems::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings},
    Layer,
};

pub fn miss_on_reached(
    mut commands: Commands,
    query: Query<Entity, (Added<DepthReached>, With<EnemyAttack>, Without<InView>)>,
) {
    for entity in &mut query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}

// TODO simplify
pub fn blood_attack_damage_on_reached(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut camera_shake_event_writer: EventWriter<CameraShakeEvent>,
    mut damage_event_writer: EventWriter<DamageEvent>,
    mut player_query: Query<Entity, With<Player>>,
    asset_server: Res<AssetServer>,
    depth_query: Query<
        (Entity, &InflictsDamage, &PxSubPosition, &Depth),
        (Added<DepthReached>, With<EnemyAttack>, With<InView>),
    >,
    volume_settings: Res<VolumeSettings>,
) {
    for (entity, damage, position, depth) in &mut depth_query.iter() {
        let sound_effect = asset_server.load("audio/sfx/enemy_melee.ogg");

        for entity in &mut player_query.iter_mut() {
            damage_event_writer.send(DamageEvent::new(entity, damage.0));
        }

        // TODO CameraShake on damage event read instead?
        camera_shake_event_writer.send(CameraShakeEvent);

        commands.spawn((
            AudioBundle {
                source: sound_effect,
                settings: PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::new_relative(volume_settings.2 * 1.0),
                    ..default()
                },
                ..default()
            },
            AudioSystemBundle {
                system_type: AudioSystemType::SFX,
            },
        ));

        let animation_o = BLOOD_ATTACK_ANIMATIONS.splat.get(&depth.0);
        if let Some(animation) = animation_o {
            commands.spawn((
                Name::new("Bloodsplat"),
                PxSubPosition::from(position.0),
                PxSpriteBundle::<Layer> {
                    sprite: assets_sprite.load(animation.sprite_path.clone()),
                    layer: Layer::Middle(depth.0),
                    anchor: PxAnchor::Center,
                    ..default()
                },
                animation.get_animation_bundle(),
            ));
        }

        commands.entity(entity).insert(DespawnMark);
    }
}
