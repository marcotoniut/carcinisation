//! Directional composition configuration.
//!
//! Defines [`MirrorPolicy`] and [`DirectionalConfig`] — the per-entity
//! directional metadata declared in `.composition.toml`.
//!
//! # Configuration format
//!
//! ```toml
//! [directional]
//! authored_directions = ["front", "frontleft", "left", "backleft", "back"]
//! default_direction = "front"
//! mirror_policy = "mirror_allowed"
//!
//! [directional.mirrors]
//! frontright = "frontleft"
//! right = "left"
//! backright = "backleft"
//!
//! [directional.animation_overrides.shoot_fly]
//! mirror_policy = "explicit_required"
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::direction::SpriteDirection;
use crate::layer_order::LayerOrderConfig;

/// How missing right-side directions are handled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorPolicy {
    /// All 8 directions must be explicitly authored. No mirroring.
    ExplicitRequired,
    /// Right-side directions may be derived from left-side via horizontal flip.
    /// Declared mirror mappings in `[directional.mirrors]` define which
    /// physical direction each virtual direction falls back to.
    #[default]
    MirrorAllowed,
    /// Entity only has front-facing sprites. No directional variants.
    FrontOnly,
}

/// Per-animation override of directional policy.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationDirectionalOverride {
    /// Override the mirror policy for this specific animation.
    #[serde(default)]
    pub mirror_policy: Option<MirrorPolicy>,
}

/// Explicit mirror mapping: virtual direction → physical source direction.
///
/// Typically:
/// ```toml
/// [directional.mirrors]
/// frontright = "frontleft"
/// right = "left"
/// backright = "backleft"
/// ```
pub type MirrorMap = HashMap<SpriteDirection, SpriteDirection>;

/// Per-part override for sourcing sprite data from a different direction
/// and/or a different part.
///
/// Used when a specific part (e.g. the human arm) lacks readable authored
/// content for a given viewing direction. The exporter substitutes that part's
/// pose data from the specified source, optionally flipping it.
///
/// ```toml
/// # Source from same part in a different direction:
/// [directional.part_source_overrides.left]
/// arm_l = { source_direction = "front", flip_x = true }
///
/// # Source from a different part in the same direction:
/// [directional.part_source_overrides.left]
/// arm_l = { source_part = "arm_r", flip_x = true }
///
/// # Source from a different part in a different direction:
/// [directional.part_source_overrides.left]
/// arm_l = { source_direction = "front", source_part = "arm_r", flip_x = true }
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PartSourceOverride {
    /// Direction to source pose data from. Defaults to the target direction.
    #[serde(default)]
    pub source_direction: Option<SpriteDirection>,
    /// Part ID to source pose data from. Defaults to the target part.
    #[serde(default)]
    pub source_part: Option<String>,
    /// Whether to horizontally flip the sourced sprite data.
    #[serde(default)]
    pub flip_x: bool,
}

/// Per-direction map of part source overrides.
/// Outer key: viewing direction. Inner key: part ID.
pub type PartSourceOverrides = HashMap<SpriteDirection, HashMap<String, PartSourceOverride>>;

/// Directional composition metadata for an entity.
///
/// Declares which directions are authored, the default direction for non-FPS
/// modes, and how missing directions are resolved.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionalConfig {
    /// Directions physically authored in the Aseprite file.
    pub authored_directions: Vec<SpriteDirection>,

    /// Default direction for non-directional contexts (e.g. ORS).
    #[serde(default = "default_direction")]
    pub default_direction: SpriteDirection,

    /// Entity-level mirror policy.
    #[serde(default)]
    pub mirror_policy: MirrorPolicy,

    /// Explicit mirror mappings: virtual direction → physical source.
    #[serde(default)]
    pub mirrors: MirrorMap,

    /// Per-animation overrides of directional policy.
    #[serde(default)]
    pub animation_overrides: HashMap<String, AnimationDirectionalOverride>,

    /// Layer ordering policy for direction-dependent draw order.
    #[serde(default)]
    pub layer_order: LayerOrderConfig,

    /// Per-direction per-part source overrides. Allows individual parts to
    /// source sprite data from a different authored direction during export.
    #[serde(default)]
    pub part_source_overrides: PartSourceOverrides,
}

fn default_direction() -> SpriteDirection {
    SpriteDirection::Front
}

impl Default for DirectionalConfig {
    fn default() -> Self {
        Self {
            authored_directions: SpriteDirection::PHYSICAL.to_vec(),
            default_direction: SpriteDirection::Front,
            mirror_policy: MirrorPolicy::default(),
            mirrors: default_mirrors(),
            animation_overrides: HashMap::new(),
            layer_order: LayerOrderConfig::default(),
            part_source_overrides: HashMap::new(),
        }
    }
}

impl DirectionalConfig {
    /// Get the effective mirror policy for a specific animation.
    ///
    /// Returns the animation-level override if declared, otherwise the
    /// entity-level default.
    #[must_use]
    pub fn effective_mirror_policy(&self, action: &str) -> MirrorPolicy {
        self.animation_overrides
            .get(action)
            .and_then(|o| o.mirror_policy)
            .unwrap_or(self.mirror_policy)
    }

    /// Resolve a direction to its physical source, applying mirror policy.
    ///
    /// Returns `(physical_direction, flip_x)` or `None` if the direction
    /// cannot be resolved under the current policy.
    #[must_use]
    pub fn resolve_direction(
        &self,
        direction: SpriteDirection,
        action: &str,
    ) -> Option<(SpriteDirection, bool)> {
        let policy = self.effective_mirror_policy(action);

        match policy {
            MirrorPolicy::FrontOnly => {
                // Only front is available.
                Some((self.default_direction, false))
            }
            MirrorPolicy::ExplicitRequired => {
                // Direction must be explicitly authored.
                if self.authored_directions.contains(&direction) {
                    Some((direction, false))
                } else {
                    None
                }
            }
            MirrorPolicy::MirrorAllowed => {
                // Try authored first.
                if self.authored_directions.contains(&direction) {
                    return Some((direction, false));
                }
                // Try declared mirror mapping.
                if let Some(&source) = self.mirrors.get(&direction)
                    && self.authored_directions.contains(&source)
                {
                    return Some((source, true));
                }
                None
            }
        }
    }
}

/// Default mirror mappings: right-side directions mirror left-side.
#[must_use]
pub fn default_mirrors() -> MirrorMap {
    let mut m = HashMap::new();
    m.insert(SpriteDirection::FrontRight, SpriteDirection::FrontLeft);
    m.insert(SpriteDirection::Right, SpriteDirection::Left);
    m.insert(SpriteDirection::BackRight, SpriteDirection::BackLeft);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> DirectionalConfig {
        DirectionalConfig {
            authored_directions: SpriteDirection::PHYSICAL.to_vec(),
            default_direction: SpriteDirection::Front,
            mirror_policy: MirrorPolicy::MirrorAllowed,
            mirrors: default_mirrors(),
            animation_overrides: HashMap::new(),
            layer_order: LayerOrderConfig::default(),
            part_source_overrides: HashMap::new(),
        }
    }

    #[test]
    fn resolve_physical_direction_no_flip() {
        let cfg = test_config();
        let (dir, flip) = cfg
            .resolve_direction(SpriteDirection::Left, "idle")
            .unwrap();
        assert_eq!(dir, SpriteDirection::Left);
        assert!(!flip);
    }

    #[test]
    fn resolve_mirrored_direction_flips() {
        let cfg = test_config();
        let (dir, flip) = cfg
            .resolve_direction(SpriteDirection::Right, "idle")
            .unwrap();
        assert_eq!(dir, SpriteDirection::Left);
        assert!(flip);
    }

    #[test]
    fn explicit_required_rejects_virtual() {
        let cfg = DirectionalConfig {
            mirror_policy: MirrorPolicy::ExplicitRequired,
            ..test_config()
        };
        assert!(
            cfg.resolve_direction(SpriteDirection::Right, "idle")
                .is_none()
        );
    }

    #[test]
    fn explicit_required_accepts_physical() {
        let cfg = DirectionalConfig {
            mirror_policy: MirrorPolicy::ExplicitRequired,
            ..test_config()
        };
        let (dir, flip) = cfg
            .resolve_direction(SpriteDirection::Left, "idle")
            .unwrap();
        assert_eq!(dir, SpriteDirection::Left);
        assert!(!flip);
    }

    #[test]
    fn front_only_always_returns_default() {
        let cfg = DirectionalConfig {
            mirror_policy: MirrorPolicy::FrontOnly,
            ..test_config()
        };
        let (dir, flip) = cfg
            .resolve_direction(SpriteDirection::BackLeft, "idle")
            .unwrap();
        assert_eq!(dir, SpriteDirection::Front);
        assert!(!flip);
    }

    #[test]
    fn per_animation_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "shoot_fly".to_string(),
            AnimationDirectionalOverride {
                mirror_policy: Some(MirrorPolicy::ExplicitRequired),
            },
        );
        let cfg = DirectionalConfig {
            animation_overrides: overrides,
            ..test_config()
        };
        // Default policy still allows mirroring
        assert!(
            cfg.resolve_direction(SpriteDirection::Right, "idle")
                .is_some()
        );
        // shoot_fly override requires explicit
        assert!(
            cfg.resolve_direction(SpriteDirection::Right, "shoot_fly")
                .is_none()
        );
    }

    #[test]
    fn effective_mirror_policy_falls_back() {
        let cfg = test_config();
        assert_eq!(
            cfg.effective_mirror_policy("anything"),
            MirrorPolicy::MirrorAllowed
        );
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = test_config();
        let toml_str = toml::to_string(&cfg).unwrap();
        let parsed: DirectionalConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.mirror_policy, cfg.mirror_policy);
        assert_eq!(
            parsed.authored_directions.len(),
            cfg.authored_directions.len()
        );
    }
}
