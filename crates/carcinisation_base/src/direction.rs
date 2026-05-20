//! Shared directional sprite model for Carcinisation.
//!
//! Defines [`SpriteDirection`] — the canonical set of 8 virtual directions
//! used for directional billboard rendering — plus helpers for parsing
//! directional tag names from Aseprite tag conventions.
//!
//! # Tag naming convention
//!
//! Aseprite tags follow `{direction}_{action}` or `{direction}_{action}_{stance}`:
//!
//! ```text
//! front_idle_stand
//! frontleft_walk_forward
//! back_death
//! ```
//!
//! Five physical directions are authored in Aseprite (front, frontleft, left,
//! backleft, back). Three right-side virtual directions are derived at runtime
//! via horizontal mirroring, governed by per-entity [`MirrorPolicy`].

use std::fmt;

use serde::{Deserialize, Serialize};

/// Number of physical (atlas-backed) directions authored in Aseprite.
pub const NUM_PHYSICAL_DIRECTIONS: usize = 5;

/// The 8 virtual directions a sprite can face, relative to the camera/player.
///
/// - `Front` means facing the camera.
/// - `Back` means facing away.
/// - Left/right follow screen-space conventions.
///
/// Five directions (Front through Back) are physically authored in Aseprite.
/// The three right-side directions are derived via horizontal mirroring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpriteDirection {
    #[serde(rename = "front")]
    Front,
    #[serde(rename = "frontleft")]
    FrontLeft,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "backleft")]
    BackLeft,
    #[serde(rename = "back")]
    Back,
    #[serde(rename = "backright")]
    BackRight,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "frontright")]
    FrontRight,
}

impl SpriteDirection {
    /// All 8 directions in clockwise order starting from front.
    pub const ALL: [Self; 8] = [
        Self::Front,
        Self::FrontLeft,
        Self::Left,
        Self::BackLeft,
        Self::Back,
        Self::BackRight,
        Self::Right,
        Self::FrontRight,
    ];

    /// The 5 physically authored directions.
    pub const PHYSICAL: [Self; NUM_PHYSICAL_DIRECTIONS] = [
        Self::Front,
        Self::FrontLeft,
        Self::Left,
        Self::BackLeft,
        Self::Back,
    ];

    /// Physical direction index (0–4) for atlas lookup.
    ///
    /// Virtual (mirrored) directions return the index of their physical source.
    #[must_use]
    pub const fn physical_index(self) -> usize {
        match self {
            Self::Front => 0,
            Self::FrontLeft | Self::FrontRight => 1,
            Self::Left | Self::Right => 2,
            Self::BackLeft | Self::BackRight => 3,
            Self::Back => 4,
        }
    }

    /// Whether this direction requires a horizontal flip to derive from its
    /// physical source direction.
    #[must_use]
    pub const fn requires_flip(self) -> bool {
        matches!(self, Self::FrontRight | Self::Right | Self::BackRight)
    }

    /// Whether this is one of the 5 physically authored directions.
    #[must_use]
    pub const fn is_physical(self) -> bool {
        !self.requires_flip()
    }

    /// The physical source direction used for atlas lookup.
    #[must_use]
    pub const fn physical_source(self) -> Self {
        match self {
            Self::FrontRight => Self::FrontLeft,
            Self::Right => Self::Left,
            Self::BackRight => Self::BackLeft,
            other => other,
        }
    }

    /// The mirrored counterpart of this direction, if one exists.
    ///
    /// Physical left-side directions return their right-side mirror and vice
    /// versa. Front and Back have no mirror (they are symmetric).
    #[must_use]
    pub const fn mirror(self) -> Option<Self> {
        match self {
            Self::FrontLeft => Some(Self::FrontRight),
            Self::Left => Some(Self::Right),
            Self::BackLeft => Some(Self::BackRight),
            Self::FrontRight => Some(Self::FrontLeft),
            Self::Right => Some(Self::Left),
            Self::BackRight => Some(Self::BackLeft),
            Self::Front | Self::Back => None,
        }
    }

    /// Tag prefix string used in Aseprite tag naming convention.
    #[must_use]
    pub const fn tag_prefix(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::FrontLeft => "frontleft",
            Self::Left => "left",
            Self::BackLeft => "backleft",
            Self::Back => "back",
            Self::BackRight => "backright",
            Self::Right => "right",
            Self::FrontRight => "frontright",
        }
    }

    /// Parse a direction from its tag prefix string.
    #[must_use]
    pub fn from_tag_prefix(s: &str) -> Option<Self> {
        match s {
            "front" => Some(Self::Front),
            "frontleft" => Some(Self::FrontLeft),
            "left" => Some(Self::Left),
            "backleft" => Some(Self::BackLeft),
            "back" => Some(Self::Back),
            "backright" => Some(Self::BackRight),
            "right" => Some(Self::Right),
            "frontright" => Some(Self::FrontRight),
            _ => None,
        }
    }

    /// Construct the full tag name for a given action.
    ///
    /// ```text
    /// SpriteDirection::FrontLeft.tag_name("idle_fly") => "frontleft_idle_fly"
    /// ```
    #[must_use]
    pub fn tag_name(self, action: &str) -> String {
        format!("{}_{action}", self.tag_prefix())
    }
}

impl fmt::Display for SpriteDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag_prefix())
    }
}

/// Tag-name prefix → direction, sorted longest-first to prevent ambiguous
/// prefix matches (e.g. "frontleft" must be tried before "front").
const DIRECTION_TAG_MAP: [(SpriteDirection, &str); 8] = [
    (SpriteDirection::FrontLeft, "frontleft"),
    (SpriteDirection::FrontRight, "frontright"),
    (SpriteDirection::BackLeft, "backleft"),
    (SpriteDirection::BackRight, "backright"),
    (SpriteDirection::Front, "front"),
    (SpriteDirection::Right, "right"),
    (SpriteDirection::Left, "left"),
    (SpriteDirection::Back, "back"),
];

/// Result of parsing a directional Aseprite tag name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDirectionalTag {
    /// The direction extracted from the tag prefix.
    pub direction: SpriteDirection,
    /// The action/animation name after the direction prefix.
    pub action: String,
}

/// Parse a directional tag name into its direction and action components.
///
/// Expects format `{direction}_{action}` where direction is one of the 8
/// recognised prefixes (5 physical + 3 right-side virtual).
///
/// Returns `None` for tags that don't match the directional format.
///
/// # Examples
///
/// ```
/// # use carcinisation_base::direction::parse_directional_tag;
/// let parsed = parse_directional_tag("front_idle_stand").unwrap();
/// assert_eq!(parsed.action, "idle_stand");
///
/// let parsed = parse_directional_tag("frontleft_idle").unwrap();
/// assert_eq!(parsed.action, "idle");
///
/// assert!(parse_directional_tag("idle_stand").is_none());
/// ```
#[must_use]
pub fn parse_directional_tag(tag_name: &str) -> Option<ParsedDirectionalTag> {
    for &(direction, prefix) in &DIRECTION_TAG_MAP {
        if let Some(rest) = tag_name.strip_prefix(prefix)
            && let Some(action) = rest.strip_prefix('_')
            && !action.is_empty()
        {
            return Some(ParsedDirectionalTag {
                direction,
                action: action.to_string(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_front_idle_stand() {
        let parsed = parse_directional_tag("front_idle_stand").unwrap();
        assert_eq!(parsed.direction, SpriteDirection::Front);
        assert_eq!(parsed.action, "idle_stand");
    }

    #[test]
    fn parse_frontleft_before_front() {
        // "frontleft_idle" must NOT be parsed as front + "left_idle"
        let parsed = parse_directional_tag("frontleft_idle").unwrap();
        assert_eq!(parsed.direction, SpriteDirection::FrontLeft);
        assert_eq!(parsed.action, "idle");
    }

    #[test]
    fn parse_backleft_before_back() {
        let parsed = parse_directional_tag("backleft_shoot").unwrap();
        assert_eq!(parsed.direction, SpriteDirection::BackLeft);
        assert_eq!(parsed.action, "shoot");
    }

    #[test]
    fn parse_right_not_confused_with_backright() {
        let parsed = parse_directional_tag("right_idle").unwrap();
        assert_eq!(parsed.direction, SpriteDirection::Right);
        assert_eq!(parsed.action, "idle");
    }

    #[test]
    fn parse_backright() {
        let parsed = parse_directional_tag("backright_walk").unwrap();
        assert_eq!(parsed.direction, SpriteDirection::BackRight);
        assert_eq!(parsed.action, "walk");
    }

    #[test]
    fn parse_frontright_before_front() {
        let parsed = parse_directional_tag("frontright_melee").unwrap();
        assert_eq!(parsed.direction, SpriteDirection::FrontRight);
        assert_eq!(parsed.action, "melee");
    }

    #[test]
    fn parse_multi_segment_action() {
        let parsed = parse_directional_tag("back_idle_fly").unwrap();
        assert_eq!(parsed.direction, SpriteDirection::Back);
        assert_eq!(parsed.action, "idle_fly");
    }

    #[test]
    fn parse_non_directional_returns_none() {
        assert!(parse_directional_tag("idle_stand").is_none());
        assert!(parse_directional_tag("shoot_fly").is_none());
    }

    #[test]
    fn parse_bare_direction_returns_none() {
        assert!(parse_directional_tag("front").is_none());
        assert!(parse_directional_tag("front_").is_none());
    }

    #[test]
    fn physical_index_consistency() {
        assert_eq!(SpriteDirection::Front.physical_index(), 0);
        assert_eq!(SpriteDirection::FrontLeft.physical_index(), 1);
        assert_eq!(SpriteDirection::FrontRight.physical_index(), 1);
        assert_eq!(SpriteDirection::Left.physical_index(), 2);
        assert_eq!(SpriteDirection::Right.physical_index(), 2);
        assert_eq!(SpriteDirection::BackLeft.physical_index(), 3);
        assert_eq!(SpriteDirection::BackRight.physical_index(), 3);
        assert_eq!(SpriteDirection::Back.physical_index(), 4);
    }

    #[test]
    fn flip_only_for_virtual_directions() {
        for dir in SpriteDirection::PHYSICAL {
            assert!(!dir.requires_flip(), "{dir:?} should not flip");
        }
        assert!(SpriteDirection::FrontRight.requires_flip());
        assert!(SpriteDirection::Right.requires_flip());
        assert!(SpriteDirection::BackRight.requires_flip());
    }

    #[test]
    fn mirror_pairs() {
        assert_eq!(
            SpriteDirection::FrontLeft.mirror(),
            Some(SpriteDirection::FrontRight)
        );
        assert_eq!(
            SpriteDirection::FrontRight.mirror(),
            Some(SpriteDirection::FrontLeft)
        );
        assert_eq!(SpriteDirection::Front.mirror(), None);
        assert_eq!(SpriteDirection::Back.mirror(), None);
    }

    #[test]
    fn tag_name_roundtrip() {
        let tag = SpriteDirection::FrontLeft.tag_name("idle_fly");
        assert_eq!(tag, "frontleft_idle_fly");
        let parsed = parse_directional_tag(&tag).unwrap();
        assert_eq!(parsed.direction, SpriteDirection::FrontLeft);
        assert_eq!(parsed.action, "idle_fly");
    }

    #[test]
    fn from_tag_prefix_roundtrip() {
        for dir in SpriteDirection::ALL {
            let prefix = dir.tag_prefix();
            let parsed = SpriteDirection::from_tag_prefix(prefix).unwrap();
            assert_eq!(parsed, dir);
        }
    }

    #[test]
    fn tag_name_produces_expected_physical_tags() {
        assert_eq!(
            SpriteDirection::Front.tag_name("idle_fly"),
            "front_idle_fly"
        );
        assert_eq!(
            SpriteDirection::BackLeft.tag_name("shoot"),
            "backleft_shoot"
        );
    }
}
