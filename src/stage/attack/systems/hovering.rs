use crate::{
    components::{DelayedDespawnOnPxAnimationFinished, DespawnMark},
    plugins::movement::linear::components::{LinearTargetReached, TargetingPositionZ},
    stage::{
        attack::components::EnemyHoveringAttackType,
        components::{
            damage::InflictsDamage,
            placement::{Depth, InView},
        },
        events::DamageEvent,
        player::{components::Player, events::CameraShakeEvent},
        resources::StageTime,
    },
    systems::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings},
    Layer,
};
use assert_assets_path::assert_assets_path;
use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

pub fn hovering_damage_on_reached(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut camera_shake_event_writer: EventWriter<CameraShakeEvent>,
    mut damage_event_writer: EventWriter<DamageEvent>,
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
            Added<LinearTargetReached<StageTime, TargetingPositionZ>>,
            With<InView>,
        ),
    >,
    volume_settings: Res<VolumeSettings>,
) {
    for (entity, attack, damage, position, depth) in &mut depth_query.iter() {
        let sound_effect = asset_server.load(assert_assets_path!("audio/sfx/enemy_melee.ogg"));

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
                    ..Default::default()
                },
                ..Default::default()
            },
            AudioSystemBundle {
                system_type: AudioSystemType::SFX,
            },
        ));

        // Depth is One on reached!
        let animation_o = attack.get_animations().hit.get(depth);
        if let Some(animation) = animation_o {
            commands.spawn((
                Name::new(format!("Attack - {} - hit", attack.get_name())),
                PxSubPosition::from(position.0),
                PxSpriteBundle::<Layer> {
                    sprite: assets_sprite
                        .load_animated(animation.sprite_path.clone(), animation.frames),
                    layer: depth.to_layer(),
                    anchor: PxAnchor::Center,
                    ..Default::default()
                },
                animation.make_animation_bundle(),
                DelayedDespawnOnPxAnimationFinished::from_secs_f32(0.4),
            ));
        }

        commands.entity(entity).insert(DespawnMark);
    }
}
