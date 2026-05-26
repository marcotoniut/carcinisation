//! Semantic animation resolution for directional sprites.
//!
//! [`DirectionalAnimationSet`] provides a type-safe lookup layer over raw
//! directional tag names. Systems request an action + direction and get back
//! the resolved tag, source direction, and flip state — without string
//! construction or mirror policy logic leaking into gameplay code.
//!
//! # Status
//!
//! This module is fully implemented and tested but not yet consumed by runtime
//! systems. The FPS billboard loader and ORS composed animation system still
//! resolve directions via ad-hoc paths. Integration is planned for when
//! enemies are migrated to the shared directional billboard pipeline.
//!
//! # Usage
//!
//! ```ignore
//! let set = DirectionalAnimationSet::from_composed_atlas(&atlas, &config);
//! let resolved = set.resolve("idle_fly", SpriteDirection::Right).unwrap();
//! // resolved.tag == "frontleft_idle_fly"  (mirrored from left)
//! // resolved.flip_x == true
//! ```

use std::collections::{HashMap, HashSet};

use crate::direction::{ParsedDirectionalTag, SpriteDirection, parse_directional_tag};
use crate::directional_config::{DirectionalConfig, MirrorPolicy};
use crate::layer_order::LayerOrderMode;

/// Result of resolving a directional animation tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTag {
    /// The full tag name to look up in the atlas (e.g. `"frontleft_idle_fly"`).
    pub tag: String,
    /// The physical source direction used for this resolution.
    pub source_direction: SpriteDirection,
    /// Whether the sprite should be horizontally flipped.
    pub flip_x: bool,
    /// The layer ordering mode to apply for this direction.
    ///
    /// Uses the *viewing* direction (what the observer sees), not the physical
    /// source direction. For mirrored directions, this means
    /// `BackRight` uses `BackRight`'s policy even though sprites come from
    /// `BackLeft`. Configure `[directional.layer_order.direction]` for virtual
    /// directions if they need different ordering than their physical source.
    pub layer_order_mode: LayerOrderMode,
}

/// Semantic animation set built from a composed atlas and directional config.
///
/// Provides `resolve(action, direction)` for type-safe tag lookup with
/// mirror fallback and layer order policy.
#[derive(Clone, Debug)]
pub struct DirectionalAnimationSet {
    /// Available actions → set of authored directions.
    actions: HashMap<String, HashSet<SpriteDirection>>,
    /// Entity-level directional config.
    config: DirectionalConfig,
}

/// Error returned when a direction cannot be resolved for an action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveError {
    /// The requested action does not exist in the animation set.
    UnknownAction(String),
    /// The direction cannot be resolved (no match, no mirror, no fallback).
    DirectionUnavailable {
        action: String,
        direction: SpriteDirection,
        policy: MirrorPolicy,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAction(action) => write!(f, "unknown animation action '{action}'"),
            Self::DirectionUnavailable {
                action,
                direction,
                policy,
            } => write!(
                f,
                "direction {direction} unavailable for action '{action}' (policy: {policy:?})"
            ),
        }
    }
}

impl DirectionalAnimationSet {
    /// Build an animation set from a list of tag names and a directional config.
    ///
    /// Tags that don't match the directional format are silently skipped.
    #[must_use]
    pub fn from_tags(tags: &[&str], config: DirectionalConfig) -> Self {
        let mut actions: HashMap<String, HashSet<SpriteDirection>> = HashMap::new();
        for &tag in tags {
            if tag.starts_with('_') {
                continue;
            }
            if let Some(ParsedDirectionalTag { direction, action }) = parse_directional_tag(tag) {
                actions.entry(action).or_default().insert(direction);
            }
        }
        Self { actions, config }
    }

    /// Build an animation set from tag name strings (owned).
    #[must_use]
    pub fn from_tag_strings(tags: &[String], config: DirectionalConfig) -> Self {
        let refs: Vec<&str> = tags.iter().map(String::as_str).collect();
        Self::from_tags(&refs, config)
    }

    /// All available action names.
    pub fn actions(&self) -> impl Iterator<Item = &str> {
        self.actions.keys().map(String::as_str)
    }

    /// Directions available for a specific action.
    #[must_use]
    pub fn directions_for(&self, action: &str) -> Option<&HashSet<SpriteDirection>> {
        self.actions.get(action)
    }

    /// The directional config backing this set.
    #[must_use]
    pub const fn config(&self) -> &DirectionalConfig {
        &self.config
    }

    /// Resolve a tag for the given action and direction.
    ///
    /// Resolution order:
    /// 1. Exact direction match in authored tags
    /// 2. Declared mirror fallback (if `MirrorAllowed`)
    /// 3. Front fallback (if `FrontOnly`)
    /// 4. `ResolveError::DirectionUnavailable`
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAction` if the action doesn't exist,
    /// or `ResolveError::DirectionUnavailable` if no resolution path exists.
    pub fn resolve(
        &self,
        action: &str,
        direction: SpriteDirection,
    ) -> Result<ResolvedTag, ResolveError> {
        let dirs = self
            .actions
            .get(action)
            .ok_or_else(|| ResolveError::UnknownAction(action.to_string()))?;

        let policy = self.config.effective_mirror_policy(action);

        match policy {
            MirrorPolicy::FrontOnly => {
                let default_dir = self.config.default_direction;
                if dirs.contains(&default_dir) {
                    Ok(ResolvedTag {
                        tag: default_dir.tag_name(action),
                        source_direction: default_dir,
                        flip_x: false,
                        layer_order_mode: self.config.layer_order.effective_policy(default_dir),
                    })
                } else {
                    Err(ResolveError::DirectionUnavailable {
                        action: action.to_string(),
                        direction,
                        policy,
                    })
                }
            }
            MirrorPolicy::ExplicitRequired => {
                if dirs.contains(&direction) {
                    Ok(ResolvedTag {
                        tag: direction.tag_name(action),
                        source_direction: direction,
                        flip_x: false,
                        layer_order_mode: self.config.layer_order.effective_policy(direction),
                    })
                } else {
                    Err(ResolveError::DirectionUnavailable {
                        action: action.to_string(),
                        direction,
                        policy,
                    })
                }
            }
            MirrorPolicy::MirrorAllowed => {
                // Try exact match first.
                if dirs.contains(&direction) {
                    return Ok(ResolvedTag {
                        tag: direction.tag_name(action),
                        source_direction: direction,
                        flip_x: false,
                        layer_order_mode: self.config.layer_order.effective_policy(direction),
                    });
                }
                // Try declared mirror mapping.
                if let Some(&source) = self.config.mirrors.get(&direction)
                    && dirs.contains(&source)
                {
                    return Ok(ResolvedTag {
                        tag: source.tag_name(action),
                        source_direction: source,
                        flip_x: true,
                        layer_order_mode: self.config.layer_order.effective_policy(direction),
                    });
                }
                Err(ResolveError::DirectionUnavailable {
                    action: action.to_string(),
                    direction,
                    policy,
                })
            }
        }
    }

    /// Convenience: resolve for the default direction (typically Front).
    ///
    /// Used by ORS which always renders front-facing.
    ///
    /// # Errors
    ///
    /// Same as [`Self::resolve`].
    pub fn resolve_default(&self, action: &str) -> Result<ResolvedTag, ResolveError> {
        self.resolve(action, self.config.default_direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::directional_config::default_mirrors;
    use crate::layer_order::LayerOrderConfig;

    fn standard_tags() -> Vec<&'static str> {
        vec![
            "front_idle_fly",
            "frontleft_idle_fly",
            "left_idle_fly",
            "backleft_idle_fly",
            "back_idle_fly",
            "front_shoot_fly",
            "frontleft_shoot_fly",
            "left_shoot_fly",
            "backleft_shoot_fly",
            "back_shoot_fly",
        ]
    }

    fn standard_config() -> DirectionalConfig {
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
    fn resolve_front_exact() {
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), standard_config());
        let resolved = set.resolve("idle_fly", SpriteDirection::Front).unwrap();
        assert_eq!(resolved.tag, "front_idle_fly");
        assert_eq!(resolved.source_direction, SpriteDirection::Front);
        assert!(!resolved.flip_x);
    }

    #[test]
    fn resolve_mirrored_right() {
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), standard_config());
        let resolved = set.resolve("idle_fly", SpriteDirection::Right).unwrap();
        assert_eq!(resolved.tag, "left_idle_fly");
        assert_eq!(resolved.source_direction, SpriteDirection::Left);
        assert!(resolved.flip_x);
    }

    #[test]
    fn resolve_frontright_mirrors_frontleft() {
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), standard_config());
        let resolved = set
            .resolve("shoot_fly", SpriteDirection::FrontRight)
            .unwrap();
        assert_eq!(resolved.tag, "frontleft_shoot_fly");
        assert!(resolved.flip_x);
    }

    #[test]
    fn resolve_default_uses_front() {
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), standard_config());
        let resolved = set.resolve_default("idle_fly").unwrap();
        assert_eq!(resolved.tag, "front_idle_fly");
        assert!(!resolved.flip_x);
    }

    #[test]
    fn unknown_action_returns_error() {
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), standard_config());
        let err = set
            .resolve("nonexistent", SpriteDirection::Front)
            .unwrap_err();
        assert!(matches!(err, ResolveError::UnknownAction(_)));
    }

    #[test]
    fn explicit_required_rejects_virtual() {
        let mut config = standard_config();
        config.mirror_policy = MirrorPolicy::ExplicitRequired;
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), config);
        let err = set.resolve("idle_fly", SpriteDirection::Right).unwrap_err();
        assert!(matches!(err, ResolveError::DirectionUnavailable { .. }));
    }

    #[test]
    fn front_only_always_returns_front() {
        let mut config = standard_config();
        config.mirror_policy = MirrorPolicy::FrontOnly;
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), config);
        let resolved = set.resolve("idle_fly", SpriteDirection::BackLeft).unwrap();
        assert_eq!(resolved.tag, "front_idle_fly");
        assert!(!resolved.flip_x);
    }

    #[test]
    fn per_action_override_restricts_mirroring() {
        let mut config = standard_config();
        config.animation_overrides.insert(
            "shoot_fly".to_string(),
            crate::directional_config::AnimationDirectionalOverride {
                mirror_policy: Some(MirrorPolicy::ExplicitRequired),
            },
        );
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), config);
        // idle_fly still mirrors
        assert!(set.resolve("idle_fly", SpriteDirection::Right).is_ok());
        // shoot_fly can't mirror
        assert!(set.resolve("shoot_fly", SpriteDirection::Right).is_err());
    }

    #[test]
    fn skips_dev_prefixed_tags() {
        let tags = vec!["_dev_test", "front_idle"];
        let set = DirectionalAnimationSet::from_tags(&tags, standard_config());
        assert!(set.directions_for("dev_test").is_none());
        assert!(set.directions_for("idle").is_some());
    }

    #[test]
    fn actions_lists_all() {
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), standard_config());
        let mut actions: Vec<&str> = set.actions().collect();
        actions.sort_unstable();
        assert_eq!(actions, vec!["idle_fly", "shoot_fly"]);
    }

    #[test]
    fn layer_order_mode_propagated() {
        use crate::layer_order::{DirectionLayerOrder, LayerOrderMode};
        let mut config = standard_config();
        config.layer_order.direction.insert(
            SpriteDirection::Back,
            DirectionLayerOrder {
                policy: LayerOrderMode::Reverse,
                parts: HashMap::new(),
            },
        );
        let set = DirectionalAnimationSet::from_tags(&standard_tags(), config);
        let front = set.resolve("idle_fly", SpriteDirection::Front).unwrap();
        assert_eq!(front.layer_order_mode, LayerOrderMode::Canonical);
        let back = set.resolve("idle_fly", SpriteDirection::Back).unwrap();
        assert_eq!(back.layer_order_mode, LayerOrderMode::Reverse);
    }
}
