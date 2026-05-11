//! Splash-specific resources.

/// Marker resource: when present, the active cutscene is the splash screen.
/// Used by the `CutsceneShutdownEvent` observer to trigger the post-splash
/// boot path instead of normal cutscene progression.
pub use carcinisation_core::components::SplashActive;
