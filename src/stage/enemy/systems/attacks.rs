use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    stage::{
        components::*,
        enemy::{components::EnemyAttack, data::blood_attack::BLOOD_ATTACK_ANIMATIONS},
        player::{components::Player, events::CameraShakeTrigger},
    },
    systems::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings},
    Layer,
};

pub fn miss_on_reached(
    mut commands: Commands,
    query: Query<Entity, (With<EnemyAttack>, With<DepthReached>, Without<InView>)>,
) {
    for entity in &mut query.iter() {
        commands.entity(entity).despawn();
    }
}

// TODO simplify
pub fn blood_attack_damage_on_reached(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut player_query: Query<&mut Health, With<Player>>,
    mut event_writer: EventWriter<CameraShakeTrigger>,
    asset_server: Res<AssetServer>,
    depth_query: Query<
        (Entity, &Damage, &PxSubPosition, &Depth),
        (With<EnemyAttack>, With<DepthReached>, With<InView>),
    >,
    volume_settings: Res<VolumeSettings>,
) {
    for (entity, damage, position, depth) in &mut depth_query.iter() {
        let sound_effect = asset_server.load("audio/sfx/enemy_melee.ogg");

        for mut health in &mut player_query.iter_mut() {
            let new_health = health.0 as i32 - damage.0 as i32;
            health.0 = new_health.max(0) as u32;
        }

        event_writer.send(CameraShakeTrigger {});

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

        commands.entity(entity).despawn();
    }
}
