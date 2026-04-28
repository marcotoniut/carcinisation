//! Cutscene vocabulary types for Carcinisation.
//!
//! Defines cutscene sub-layers, runtime components, resources, and lifecycle
//! messages. Serialisable script types (`CutsceneData`, `CutsceneAct`, etc.)
//! remain in the app crate for now because they reference app-specific
//! rendering layer types. See `apps/carcinisation/src/cutscene/data.rs`.

pub mod components;
pub mod layer;
pub mod resources;
