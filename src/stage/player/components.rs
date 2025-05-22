use crate::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::{layer::Layer, stage::components::placement::Depth};
use assert_assets_path::assert_assets_path;
use bevy::{audio::PlaybackMode, prelude::*, utils::HashSet};
use seldom_pixel::prelude::{
    PxAnchor, PxAnimation, PxAnimationDuration, PxAnimationFinishBehavior, PxCanvas, PxSprite,
    PxSubPosition,
};

#[derive(Component)]
pub struct Player;

pub const PLAYER_SPEED: f32 = 125.;
pub const PLAYER_SIZE: f32 = 0.;
pub const PLAYER_MAX_HEALTH: u32 = 100;
pub const PLAYER_DEPTH: Depth = Depth::Zero;

pub const ATTACK_PINCER_DAMAGE: u32 = 70;
pub const ATTACK_GUN_DAMAGE: u32 = 30;

#[derive(Clone, Copy, Debug, Reflect)]
pub enum Weapon {
    Pincer,
    Gun,
}

#[derive(Clone, Component, Copy, Debug, Reflect)]
pub struct PlayerAttack {
    pub weapon: Weapon,
    pub position: Vec2,
    // TODO reach
}

#[derive(Component, Clone, Debug)]
pub struct UnhittableList(pub HashSet<Entity>);

impl PlayerAttack {
    pub fn make_bundles(
        &self,
        asset_server: Res<AssetServer>,
        volume_settings: Res<VolumeSettings>,
    ) -> (
        // TODO Bundles
        (
            Self,
            PxSprite,
            PxAnchor,
            PxCanvas,
            Layer,
            PxAnimation,
            PxSubPosition,
            UnhittableList,
            Name,
        ),
        (
            AudioPlayer<AudioSource>,
            PlaybackSettings,
            AudioSystemBundle,
        ),
    ) {
        let position = PxSubPosition::from(self.position);
        let name = Name::new("PlayerAttack");

        // TODO sprite
        let (
            sprite,
            anchor,
            canvas,
            layer,
            animation,
            audio_player,
            playback_settings,
            audio_system_bundle,
        ) = match self.weapon {
            Weapon::Pincer => {
                let melee_slash_sound =
                    asset_server.load(assert_assets_path!("audio/sfx/player_melee.ogg"));
                let sprite =
                    PxSprite(asset_server.load(assert_assets_path!("sprites/melee_slash.png")));
                // TODO animate , 9
                (
                    sprite,
                    PxAnchor::Center,
                    PxCanvas::Camera,
                    Layer::Attack,
                    PxAnimation {
                        duration: PxAnimationDuration::millis_per_animation(500),
                        on_finish: PxAnimationFinishBehavior::Despawn,
                        ..default()
                    },
                    AudioPlayer::new(melee_slash_sound),
                    PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: volume_settings.sfx.clone(),
                        ..default()
                    },
                    AudioSystemBundle {
                        system_type: AudioSystemType::SFX,
                    },
                )
            }
            Weapon::Gun => {
                let shoot_sound =
                    asset_server.load(assert_assets_path!("audio/sfx/player_shot.ogg"));
                let sprite = PxSprite(
                    asset_server.load(assert_assets_path!("sprites/bullet_particles.png")),
                );
                // TODO animate 4
                (
                    sprite,
                    PxAnchor::Center,
                    PxCanvas::Camera,
                    Layer::Attack,
                    PxAnimation {
                        duration: PxAnimationDuration::millis_per_animation(80),
                        on_finish: PxAnimationFinishBehavior::Despawn,
                        ..default()
                    },
                    AudioPlayer::new(shoot_sound),
                    PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: volume_settings.sfx.clone(),
                        ..default()
                    },
                    AudioSystemBundle {
                        system_type: AudioSystemType::SFX,
                    },
                )
            }
        };

        (
            (
                *self,
                sprite,
                anchor,
                canvas,
                layer,
                animation,
                position,
                UnhittableList(HashSet::default()),
                name,
            ),
            (audio_player, playback_settings, audio_system_bundle),
        )
    }
}

#[derive(Component, Reflect)]
pub struct CameraShake {
    pub timer: Timer,
    pub intensity: f32,
    pub original_position: Vec2,
    pub shaking: bool,
}
