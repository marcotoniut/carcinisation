//! Splash startup/shutdown: constructs CutsceneData from per-track RON config
//! and delegates to CutscenePlugin.

use super::{components::SplashActive, messages::SplashStartupEvent, timeline::SplashConfig};
use crate::cutscene::{
    data::{
        CutsceneAct, CutsceneBackgroundPrimitive, CutsceneData, CutsceneImageSpawn,
        CutsceneImagesSpawn, CutsceneSkipMode,
    },
    messages::{CutsceneShutdownEvent, CutsceneStartupEvent},
    resources::CutsceneTimeDomain,
};
use bevy::prelude::*;
use carcinisation_core::core::time::TimeMultiplier;
use std::sync::Arc;

/// Builds a `CutsceneData` from the splash RON config.
#[must_use]
pub fn build_splash_cutscene_data() -> (SplashConfig, CutsceneData) {
    let config = SplashConfig::load();

    let image_spawns: Vec<CutsceneImageSpawn> = config
        .tracks
        .iter()
        .map(|track| {
            let mut spawn = CutsceneImageSpawn::new(track.asset.clone(), crate::layer::Layer::UI);
            if let Some(appear_ms) = track.appear_ms {
                spawn = spawn.with_appear_ms(Some(appear_ms));
            }
            if let Some(ref keyframes) = track.rotation {
                spawn.rotation_keyframes_o = Some(keyframes.clone());
            }
            if let Some((px, py)) = track.pivot {
                spawn.rotation_pivot_o = Some(Vec2::new(px, py));
            }
            if let Some((x, y)) = track.position {
                spawn.coordinates = Vec2::new(x as f32, y as f32);
            }
            if let Some(ref tag) = track.tag {
                spawn = spawn.with_tag(tag.clone());
            }
            if let Some(ref follow_tag) = track.follow_rotation_tag {
                spawn = spawn.with_follow_rotation_tag(follow_tag.clone());
            }
            spawn
        })
        .collect();

    let act = CutsceneAct::new()
        .with_elapse(config.total_duration_ms as f32 / 1000.0)
        .spawn_images(CutsceneImagesSpawn::new().with_spawns(image_spawns))
        .with_background_primitive(CutsceneBackgroundPrimitive {
            palette_index: config.bg_palette_index,
            layer: crate::layer::Layer::UIBackground,
        });

    let data = CutsceneData::new("Splash".to_string())
        .set_steps(vec![act])
        .with_skip_mode(CutsceneSkipMode::AnyGameplayKey)
        .with_respect_skip_cutscenes(false);

    (config, data)
}

/// Constructs a CutsceneData from the per-track splash config.
pub fn on_splash_startup(_trigger: On<SplashStartupEvent>, mut commands: Commands) {
    info!("Splash: startup");

    let (config, cutscene_data) = build_splash_cutscene_data();

    #[cfg(debug_assertions)]
    if config.slowdown > 1 {
        commands.insert_resource(TimeMultiplier::<CutsceneTimeDomain>::new(
            1.0 / config.slowdown as f32,
        ));
    }

    commands.insert_resource(SplashActive);
    commands.trigger(CutsceneStartupEvent {
        data: Arc::new(cutscene_data),
    });
}

/// When the cutscene shuts down during a splash, continue the boot path.
pub fn on_cutscene_shutdown_during_splash(
    _trigger: On<CutsceneShutdownEvent>,
    mut commands: Commands,
    splash_active: Option<Res<SplashActive>>,
    dev_flags: Res<crate::resources::DevFlags>,
) {
    if splash_active.is_none() {
        return;
    }

    info!("Splash: finished");
    commands.remove_resource::<SplashActive>();
    #[cfg(debug_assertions)]
    commands.remove_resource::<TimeMultiplier<CutsceneTimeDomain>>();

    super::continue_after_splash(&mut commands, &dev_flags);
}
