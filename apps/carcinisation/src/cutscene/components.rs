//! Core cutscene markers used for spawned entities.

use bevy::prelude::*;
use std::time::Duration;

#[derive(Component)]
/// Root entity for the active cutscene scene graph.
pub struct CutsceneEntity;

#[derive(Component)]
/// Marks entities that should only run during cinematic playback.
pub struct Cinematic;

#[derive(Component)]
/// Marks spawned graphics belonging to the cutscene (cleanup helper).
pub struct CutsceneGraphic;

/// Element that follows the shared `CutsceneTimelineConfig` rotation curve.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct TimelineCurveFollower {
    /// Time (cutscene-domain) when this element becomes visible.
    pub appear_at: Duration,
    /// Playback speed through the shared curve (1.0 = normal, <1.0 = lag).
    pub time_scale: f32,
    /// Static angle offset in radians (pre-rotated art compensation).
    pub angle_offset: f32,
}

/// Element that should appear at a specific cutscene time (non-rotating).
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CutsceneAppearAt(pub Duration);

/// Element whose rotation = leader's rotation + relative offset keyframes.
/// The leader is found by matching `leader_tag` against [`Tag`] components.
#[derive(Component, Clone, Debug)]
pub struct RotationFollower {
    pub leader_tag: String,
}
