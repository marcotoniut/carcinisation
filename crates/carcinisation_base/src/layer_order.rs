//! Layer ordering policy for directional sprite composition.
//!
//! Controls how part `draw_order` values are interpreted across directions.
//! The canonical `draw_order` in `.composition.toml` applies to front-facing
//! sprites. Back-facing sprites may need reversed or custom ordering.
//!
//! # Configuration format
//!
//! ```toml
//! [layer_order]
//! default_policy = "canonical"
//!
//! [layer_order.direction.back]
//! policy = "reverse"
//!
//! [layer_order.direction.backleft]
//! policy = "reverse"
//!
//! # Explicit per-direction part overrides:
//! [layer_order.direction.left.parts]
//! weapon = 70
//! arm_l = 60
//! body = 50
//! arm_r = 40
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::direction::SpriteDirection;

/// How part `draw_order` values are applied for a given direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerOrderMode {
    /// Use the canonical `draw_order` from `composition.toml` as-is.
    #[default]
    Canonical,
    /// Invert `draw_order`: `max_draw_order - part.draw_order`.
    /// Body renders on top, arms/weapons behind (viewer sees entity's back).
    Reverse,
}

/// Per-direction layer ordering override.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionLayerOrder {
    /// The ordering mode for this direction.
    #[serde(default)]
    pub policy: LayerOrderMode,

    /// Explicit per-part `draw_order` overrides for this direction.
    /// Part IDs map to their direction-specific `draw_order` values.
    /// If present, these take precedence over policy-based ordering.
    #[serde(default)]
    pub parts: HashMap<String, u8>,
}

/// Top-level layer ordering configuration for an entity.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayerOrderConfig {
    /// Default ordering mode applied to all directions unless overridden.
    #[serde(default)]
    pub default_policy: LayerOrderMode,

    /// Per-direction ordering overrides.
    #[serde(default)]
    pub direction: HashMap<SpriteDirection, DirectionLayerOrder>,
}

impl LayerOrderConfig {
    /// Get the effective ordering mode for a direction.
    #[must_use]
    pub fn effective_policy(&self, dir: SpriteDirection) -> LayerOrderMode {
        self.direction
            .get(&dir)
            .map_or(self.default_policy, |d| d.policy)
    }

    /// Get explicit per-part `draw_order` overrides for a direction, if any.
    #[must_use]
    pub fn part_overrides(&self, dir: SpriteDirection) -> Option<&HashMap<String, u8>> {
        self.direction
            .get(&dir)
            .map(|d| &d.parts)
            .filter(|p| !p.is_empty())
    }

    /// Compute effective `draw_order` for a part in a given direction.
    ///
    /// Priority:
    /// 1. Explicit per-direction per-part override
    /// 2. Policy-based transformation of canonical `draw_order`
    #[must_use]
    pub fn resolve_draw_order(
        &self,
        dir: SpriteDirection,
        part_id: &str,
        canonical_draw_order: u8,
        max_draw_order: u8,
    ) -> u8 {
        // Check explicit part override first.
        if let Some(overrides) = self.part_overrides(dir)
            && let Some(&order) = overrides.get(part_id)
        {
            return order;
        }
        // Apply policy-based transformation.
        match self.effective_policy(dir) {
            LayerOrderMode::Canonical => canonical_draw_order,
            LayerOrderMode::Reverse => max_draw_order.saturating_sub(canonical_draw_order),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_preserves_order() {
        let cfg = LayerOrderConfig::default();
        assert_eq!(
            cfg.resolve_draw_order(SpriteDirection::Front, "body", 20, 80),
            20
        );
    }

    #[test]
    fn reverse_inverts_order() {
        let mut cfg = LayerOrderConfig::default();
        cfg.direction.insert(
            SpriteDirection::Back,
            DirectionLayerOrder {
                policy: LayerOrderMode::Reverse,
                parts: HashMap::new(),
            },
        );
        // max=80, canonical=20 → reversed=60
        assert_eq!(
            cfg.resolve_draw_order(SpriteDirection::Back, "body", 20, 80),
            60
        );
        // Front still canonical
        assert_eq!(
            cfg.resolve_draw_order(SpriteDirection::Front, "body", 20, 80),
            20
        );
    }

    #[test]
    fn explicit_part_overrides_take_precedence() {
        let mut parts = HashMap::new();
        parts.insert("weapon".to_string(), 70);
        let mut cfg = LayerOrderConfig::default();
        cfg.direction.insert(
            SpriteDirection::Left,
            DirectionLayerOrder {
                policy: LayerOrderMode::Canonical,
                parts,
            },
        );
        // Explicit override wins
        assert_eq!(
            cfg.resolve_draw_order(SpriteDirection::Left, "weapon", 30, 80),
            70
        );
        // Non-overridden part uses policy
        assert_eq!(
            cfg.resolve_draw_order(SpriteDirection::Left, "body", 20, 80),
            20
        );
    }

    #[test]
    fn default_policy_applies_to_all_directions() {
        let cfg = LayerOrderConfig {
            default_policy: LayerOrderMode::Reverse,
            direction: HashMap::new(),
        };
        assert_eq!(
            cfg.resolve_draw_order(SpriteDirection::Back, "body", 20, 80),
            60
        );
        assert_eq!(
            cfg.resolve_draw_order(SpriteDirection::FrontLeft, "body", 20, 80),
            60
        );
    }

    #[test]
    fn serde_roundtrip() {
        let mut cfg = LayerOrderConfig {
            default_policy: LayerOrderMode::Canonical,
            direction: HashMap::new(),
        };
        let mut parts = HashMap::new();
        parts.insert("weapon".to_string(), 70);
        cfg.direction.insert(
            SpriteDirection::Back,
            DirectionLayerOrder {
                policy: LayerOrderMode::Reverse,
                parts,
            },
        );
        let toml_str = toml::to_string(&cfg).unwrap();
        let parsed: LayerOrderConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.default_policy, LayerOrderMode::Canonical);
        assert_eq!(
            parsed.effective_policy(SpriteDirection::Back),
            LayerOrderMode::Reverse
        );
        assert_eq!(
            parsed.part_overrides(SpriteDirection::Back).unwrap()["weapon"],
            70
        );
    }
}
