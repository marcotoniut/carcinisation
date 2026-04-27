//! Boot-time splash screen — thin wrapper that constructs CutsceneData and
//! delegates to CutscenePlugin.

pub mod components;
pub mod messages;
pub mod systems;
mod timeline;

use self::systems::{on_cutscene_shutdown_during_splash, on_splash_startup};
use bevy::prelude::*;

/// Registers splash lifecycle observers. The actual rendering and timeline
/// are handled by CutscenePlugin — this plugin just bridges the boot flow.
pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_splash_startup)
            .add_observer(on_cutscene_shutdown_during_splash);
    }
}

/// After the splash finishes (or is skipped), resume the normal boot path.
pub(crate) fn continue_after_splash(
    commands: &mut Commands,
    dev_flags: &crate::resources::DevFlags,
) {
    if dev_flags.skip_menu {
        info!("CARCINISATION_SKIP_MENU: skipping main menu, starting game directly");
        commands.trigger(crate::game::messages::GameStartupEvent);
    } else {
        activable::activate::<crate::main_menu::MainMenuPlugin>(commands);
    }
}
