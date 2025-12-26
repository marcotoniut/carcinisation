use crate::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::pixel::{PxAnimationBundle, PxAssets, PxSpriteBundle};
use crate::{
    layer::Layer,
    stage::{
        components::placement::Depth,
        player::attacks::{
            AttackDefinition, AttackEffectState, AttackHitTracker, AttackId, AttackLifetime,
        },
    },
};
use bevy::{
    audio::{AudioPlayer, PlaybackMode, PlaybackSettings},
    prelude::*,
};
use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationDuration, PxSprite, PxSubPosition};

#[derive(Component)]
pub struct Player;

pub const PLAYER_SPEED: f32 = 125.;
pub const PLAYER_SIZE: f32 = 0.;
pub const PLAYER_MAX_HEALTH: u32 = 100;
pub const PLAYER_DEPTH: Depth = Depth::Zero;

#[derive(Clone, Component, Copy, Debug, Reflect)]
pub struct PlayerAttack {
    pub attack_id: AttackId,
    pub position: Vec2,
    // TODO reach
}

impl PlayerAttack {
    pub fn make_bundles(
        &self,
        definition: &AttackDefinition,
        assets_sprite: &mut PxAssets<PxSprite>,
        asset_server: &AssetServer,
        volume_settings: &VolumeSettings,
    ) -> (
        (
            Self,
            PxSpriteBundle<Layer>,
            PxAnimationBundle,
            PxSubPosition,
            AttackHitTracker,
            AttackEffectState,
            AttackLifetime,
            Name,
        ),
        (AudioPlayer, PlaybackSettings, AudioSystemBundle),
    ) {
        let position = PxSubPosition::from(self.position);
        let name = Name::new(format!("PlayerAttack<{}>", definition.name));

        let sprite =
            assets_sprite.load_animated(definition.sprite.sprite_path, definition.sprite.frames);
        let sprite_bundle = PxSpriteBundle::<Layer> {
            sprite: sprite.into(),
            anchor: definition.sprite.anchor,
            canvas: definition.sprite.canvas,
            layer: definition.sprite.layer.clone(),
            ..default()
        };
        let animation_bundle = PxAnimationBundle::from_parts(
            PxAnimationDirection::default(),
            PxAnimationDuration::millis_per_animation(definition.sprite.speed_ms),
            definition.sprite.finish_behavior,
            definition.sprite.frame_transition,
        );
        let audio_player = AudioPlayer(asset_server.load(definition.sfx_path));
        let playback_settings = PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: volume_settings.sfx,
            ..Default::default()
        };
        let audio_system_bundle = AudioSystemBundle {
            system_type: AudioSystemType::SFX,
        };

        (
            (
                *self,
                sprite_bundle,
                animation_bundle,
                position,
                AttackHitTracker::default(),
                AttackEffectState::default(),
                AttackLifetime::new(definition.duration_secs),
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
