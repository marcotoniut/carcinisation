//! Generic components for composed enemy runtime state.
//!
//! These components support common composed enemy mechanics like death effects
//! and part breakage that any composed enemy species can use.

use bevy::prelude::*;
use std::{collections::HashSet, time::Duration};

/// Tracks a composed enemy in its death animation state.
///
/// When a composed enemy dies, it enters a dying state with a progressive
/// visual fade effect before final despawn. The animation freezes on the
/// death frame while the fade-out progresses.
///
/// # Usage
/// Add this component when an enemy takes lethal damage. The composed visual
/// system will freeze animation progression, and a death effect system should
/// handle the fade-out and eventual despawn.
#[derive(Component, Clone, Debug, Reflect)]
pub struct Dying {
    /// When the death animation started (stage time)
    pub started: Duration,
}

/// Tracks which parts of a composed enemy have been broken.
///
/// This is a generic component that tracks all broken parts by their semantic
/// part IDs. Species-specific systems can add behavioral markers (like `WingsBroken`)
/// based on which parts break.
///
/// # Usage
/// Query for this component and check `is_broken(part_id)` to determine if
/// specific gameplay should be disabled (e.g., flying disabled when wings break).
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct BrokenParts {
    /// Set of semantic part IDs that have been broken
    parts: HashSet<String>,
}

impl BrokenParts {
    /// Creates a new empty broken parts tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            parts: HashSet::new(),
        }
    }

    /// Check if a specific part is broken.
    #[must_use]
    pub fn is_broken(&self, part_id: &str) -> bool {
        self.parts.contains(part_id)
    }

    /// Mark a part as broken.
    pub fn mark_broken(&mut self, part_id: String) {
        self.parts.insert(part_id);
    }

    /// Get all broken part IDs.
    #[must_use]
    pub fn broken_parts(&self) -> &HashSet<String> {
        &self.parts
    }

    /// Get the number of broken parts.
    #[must_use]
    pub fn count(&self) -> usize {
        self.parts.len()
    }
}
