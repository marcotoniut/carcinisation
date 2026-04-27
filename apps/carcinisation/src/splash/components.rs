//! Splash-specific resources.

use bevy::prelude::*;

/// Marker resource: when present, the active cutscene is the splash screen.
/// Used by the `CutsceneShutdownEvent` observer to trigger the post-splash
/// boot path instead of normal cutscene progression.
#[derive(Resource)]
pub struct SplashActive;
