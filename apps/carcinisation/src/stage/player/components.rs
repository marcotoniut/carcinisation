use crate::components::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::pixel::{CxAnimationBundle, CxAssets, CxSpriteBundle};
use crate::{
    layer::Layer,
    stage::{
        components::interactive::ColliderData,
        components::placement::Depth,
        player::attacks::{
            AttackCollisionMode, AttackDefinition, AttackEffectState, AttackHitTracker, AttackId,
            AttackLifetime,
        },
    },
};
use bevy::{
    audio::{AudioPlayer, PlaybackMode, PlaybackSettings},
    prelude::*,
};
use carapace::prelude::{CxAnimationDirection, CxAnimationDuration, CxSprite, WorldPos};
use std::time::Duration;

#[derive(Component)]
pub struct Player;

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
        assets_sprite: &mut CxAssets<CxSprite>,
        asset_server: &AssetServer,
        volume_settings: &VolumeSettings,
    ) -> (
        (
            Self,
            CxSpriteBundle<Layer>,
            CxAnimationBundle,
            WorldPos,
            AttackHitTracker,
            AttackEffectState,
            AttackLifetime,
            ColliderData,
            Name,
        ),
        Option<(AudioPlayer, PlaybackSettings, AudioSystemBundle)>,
    ) {
        let position = WorldPos::from(self.position);
        let name = Name::new(format!("PlayerAttack<{}>", definition.name));

        let sprite =
            assets_sprite.load_animated(definition.sprite.sprite_path, definition.sprite.frames);
        let sprite_bundle = CxSpriteBundle::<Layer> {
            sprite: sprite.into(),
            anchor: definition.sprite.anchor,
            canvas: definition.sprite.canvas,
            layer: definition.sprite.layer.clone(),
            ..default()
        };
        let animation_bundle = CxAnimationBundle::from_parts(
            CxAnimationDirection::default(),
            CxAnimationDuration::millis_per_animation(definition.sprite.speed_ms),
            definition.sprite.finish_behavior,
            definition.sprite.frame_transition,
        );
        let sound_bundle = definition.sfx_path.map(|sfx_path| {
            let audio_player = AudioPlayer(asset_server.load(sfx_path));
            let playback_settings = PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: volume_settings.sfx,
                ..Default::default()
            };
            let audio_system_bundle = AudioSystemBundle {
                system_type: AudioSystemType::SFX,
            };
            (audio_player, playback_settings, audio_system_bundle)
        });

        let collider_data = match definition.collision {
            AttackCollisionMode::SpriteMask => {
                ColliderData::from_one(carcinisation_collision::Collider::new(
                    carcinisation_collision::ColliderShape::SpriteMask,
                ))
            }
            AttackCollisionMode::Point => {
                let offsets = if definition.hit_offsets.is_empty() {
                    &[IVec2::ZERO][..]
                } else {
                    definition.hit_offsets.as_slice()
                };
                ColliderData::from_many(
                    offsets
                        .iter()
                        .map(|o| {
                            carcinisation_collision::Collider::new(
                                carcinisation_collision::ColliderShape::Circle(0.5),
                            )
                            .with_offset(o.as_vec2())
                        })
                        .collect(),
                )
            }
            AttackCollisionMode::Radial { radius } => {
                ColliderData::from_one(carcinisation_collision::Collider::new(
                    carcinisation_collision::ColliderShape::Circle(radius),
                ))
            }
            AttackCollisionMode::None => ColliderData::default(),
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
                collider_data,
                name,
            ),
            sound_bundle,
        )
    }
}

/// Duration of the web slow effect.
pub const WEBBED_DURATION: Duration = Duration::from_millis(3000);

/// Movement speed multiplier while webbed.
pub const WEBBED_SPEED_MULTIPLIER: f32 = 0.4;

/// Temporary movement debuff applied when hit by a spider shot.
///
/// While active, player movement speed is multiplied by `speed_multiplier`.
/// The SHIFT slow modifier is overridden — the webbed speed always wins.
/// Repeated hits refresh `expires_at` without stacking the multiplier.
#[derive(Component, Clone, Debug, Reflect)]
pub struct Webbed {
    pub expires_at: Duration,
    pub speed_multiplier: f32,
}

impl Webbed {
    #[must_use]
    pub fn new(now: Duration) -> Self {
        Self {
            expires_at: now + WEBBED_DURATION,
            speed_multiplier: WEBBED_SPEED_MULTIPLIER,
        }
    }

    /// Refreshes the duration without changing the multiplier.
    pub fn refresh(&mut self, now: Duration) {
        self.expires_at = now + WEBBED_DURATION;
    }
}

/// Decaying camera shake applied as an offset to the camera position.
///
/// Each frame applies a random offset scaled by the current intensity, then
/// decays the intensity exponentially. When intensity drops below a threshold
/// the component is removed and the offset is cleaned up.
#[derive(Component, Reflect)]
pub struct CameraShake {
    /// Current shake intensity in pixels. Decays each frame.
    pub intensity: f32,
    /// Decay rate per second. Higher = faster fade.
    pub decay: f32,
    /// Accumulated offset applied this frame. Subtracted on cleanup.
    pub current_offset: Vec2,
}
