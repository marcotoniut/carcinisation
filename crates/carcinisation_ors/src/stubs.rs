//! Re-exports and ORS-local constants.
//!
//! Gathers shared types from `carcinisation_base`, `carcinisation_core`,
//! and `carcinisation_cutscene` under short import paths for ORS systems.
//! Also defines ORS-specific configuration constants (asset paths, defaults).

// ---------------------------------------------------------------------------
// Game / score — resolved: now in carcinisation_base::game
// ---------------------------------------------------------------------------

pub use carcinisation_base::game::{
    CameraPos, DEATH_SCORE_PENALTY, GameOverEvent, GameProgress, GameProgressState, Lives,
    STARTING_LIVES, Score,
};

// ---------------------------------------------------------------------------
// Debug — resolved: now in carcinisation_core::debug
// ---------------------------------------------------------------------------

pub use carcinisation_core::components::SplashActive;
pub use carcinisation_core::debug::{DebugGodMode, debug_print_shutdown, debug_print_startup};

// ---------------------------------------------------------------------------
// Globals (app-specific)
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const ASSETS_PATH: &str = "../../assets";

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const PATH_SPRITES_ENEMIES: &str = assert_assets_path::assert_assets_path!("sprites/enemies/");

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const PATH_SPRITES_OBJECTS: &str = assert_assets_path::assert_assets_path!("sprites/objects/");

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const DEBUG_STAGESTEP: bool = false;

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const DEFAULT_CROSSHAIR_INDEX: u8 = 1;

// ---------------------------------------------------------------------------
// Cutscene — resolved: now in carcinisation_cutscene::data
// ---------------------------------------------------------------------------

pub use carcinisation_cutscene::data::{CutsceneAnimationSpawn, CutsceneAnimationsSpawn};

// ---------------------------------------------------------------------------
// Systems / spawn — resolved: now in carcinisation_core::components
// ---------------------------------------------------------------------------

pub use carcinisation_core::components::make_music_bundle;

// ---------------------------------------------------------------------------
// Movement — resolved: now in carcinisation_core::globals
// ---------------------------------------------------------------------------

pub use carcinisation_core::globals::PositionSyncSystems;

// ---------------------------------------------------------------------------
// Transitions — resolved: now in carcinisation_cutscene::data
// ---------------------------------------------------------------------------

pub use carcinisation_cutscene::data::TransitionRequest;

// ---------------------------------------------------------------------------
// Systems (delay despawn)
// ---------------------------------------------------------------------------

// delay_despawn / check_despawn_after_delay: resolved — now in carcinisation_core::globals
pub use carcinisation_core::globals::{check_despawn_after_delay, delay_despawn};
