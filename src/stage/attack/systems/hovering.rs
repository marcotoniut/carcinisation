use crate::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::{
    components::{DelayedDespawnOnPxAnimationFinished, DespawnMark},
    layer::Layer,
    plugins::movement::linear::components::{LinearTargetReached, TargetingPositionZ},
    stage::{
        attack::components::EnemyHoveringAttackType,
        components::{
            damage::InflictsDamage,
            placement::{Depth, InView},
        },
        events::DamageEvent,
        player::{components::Player, events::CameraShakeTrigger},
        resources::StageTime,
    },
};
use assert_assets_path::assert_assets_path;
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::prelude::{PxAnchor, PxSprite, PxSubPosition};

pub fn hovering_damage_on_reached(
    mut commands: Commands,
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

        commands.spawn((
            AudioPlayer::new(sound_effect),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: volume_settings.sfx.clone(),
                ..default()
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
                PxSprite(asset_server.load(animation.sprite_path.clone())),
                depth.to_layer(),
                PxAnchor::Center,
                animation.make_animation_bundle(),
                DelayedDespawnOnPxAnimationFinished::from_secs_f32(0.4),
            ));
        }

        commands.entity(entity).insert(DespawnMark);

        // TODO CameraShake on damage event read instead?
        commands.trigger(CameraShakeTrigger);
    }
}
