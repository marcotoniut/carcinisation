//! Data structures that are processed and used at runtime by gameplay systems.

use bevy::{prelude::*, utils::HashMap};
use carcinisation_core::data::AnimationData;
use carcinisation_core::stage::components::placement::Depth;

/// The central resource that holds all compiled attack configurations.
/// Gameplay systems will read from this.
#[derive(Debug, Default, Resource)]
pub struct AttackRuntimeConfigs {
    pub configs: HashMap<String, AttackTuning>,
}

/// Processed, game-ready tuning values for a single attack.
#[derive(Debug, Clone)]
pub struct AttackTuning {
    pub depth_speed: f32,
    pub line_speed: f32,
    pub damage: u32,
    pub randomness: f32,
    pub animations: HoveringAttackAnimations,
}

/// A collection of animations for an attack, separated by depth.
#[derive(Debug, Clone, Default)]
pub struct HoveringAttackAnimations {
    /// Animation played while the attack is active/hovering.
    /// The `Depth` key corresponds to the visual and logical layer of the attack.
    pub hovering: HashMap<Depth, AnimationData>,
    /// Animation played when the attack hits a target.
    pub hit: HashMap<Depth, AnimationData>,
}
