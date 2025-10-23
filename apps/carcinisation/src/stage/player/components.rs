use crate::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::pixel::{PxAnimationBundle, PxAssets, PxSpriteBundle};
use crate::{layer::Layer, stage::components::placement::Depth};
use assert_assets_path::assert_assets_path;
use bevy::{
    audio::{AudioPlayer, PlaybackMode, PlaybackSettings},
    prelude::*,
};
use seldom_pixel::prelude::{
    PxAnchor, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior, PxCanvas,
    PxFrameTransition, PxSprite, PxSubPosition,
};
use std::collections::HashSet;

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
        assets_sprite: &mut PxAssets<PxSprite>,
        asset_server: Res<AssetServer>,
        volume_settings: Res<VolumeSettings>,
    ) -> (
        (
            Self,
            PxSpriteBundle<Layer>,
            PxAnimationBundle,
            PxSubPosition,
            UnhittableList,
            Name,
        ),
        (AudioPlayer, PlaybackSettings, AudioSystemBundle),
    ) {
        let position = PxSubPosition::from(self.position);
        let name = Name::new("PlayerAttack");

        // TODO sprite
        let (sprite_bundle, animation_bundle, audio_player, playback_settings, audio_system_bundle) =
            match self.weapon {
                Weapon::Pincer => {
                    let melee_slash_sound =
                        asset_server.load(assert_assets_path!("audio/sfx/player_melee.ogg"));
                    let sprite = assets_sprite
                        .load_animated(assert_assets_path!("sprites/melee_slash.px_sprite.png"), 9);
                    (
                        PxSpriteBundle::<Layer> {
                            sprite: sprite.into(),
                            anchor: PxAnchor::Center,
                            canvas: PxCanvas::Camera,
                            layer: Layer::Attack,
                            ..default()
                        },
                        PxAnimationBundle::from_parts(
                            PxAnimationDirection::default(),
                            PxAnimationDuration::millis_per_animation(500),
                            PxAnimationFinishBehavior::Despawn,
                            PxFrameTransition::default(),
                        ),
                        AudioPlayer(melee_slash_sound),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            volume: volume_settings.sfx.clone(),
                            ..Default::default()
                        },
                        AudioSystemBundle {
                            system_type: AudioSystemType::SFX,
                        },
                    )
                }
                Weapon::Gun => {
                    let shoot_sound =
                        asset_server.load(assert_assets_path!("audio/sfx/player_shot.ogg"));
                    let sprite = assets_sprite.load_animated(
                        assert_assets_path!("sprites/bullet_particles.px_sprite.png"),
                        4,
                    );
                    (
                        PxSpriteBundle::<Layer> {
                            sprite: sprite.into(),
                            anchor: PxAnchor::Center,
                            canvas: PxCanvas::Camera,
                            layer: Layer::Attack,
                            ..default()
                        },
                        PxAnimationBundle::from_parts(
                            PxAnimationDirection::default(),
                            PxAnimationDuration::millis_per_animation(80),
                            PxAnimationFinishBehavior::Despawn,
                            PxFrameTransition::default(),
                        ),
                        AudioPlayer(shoot_sound),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            volume: volume_settings.sfx.clone(),
                            ..Default::default()
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
                sprite_bundle,
                animation_bundle,
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
