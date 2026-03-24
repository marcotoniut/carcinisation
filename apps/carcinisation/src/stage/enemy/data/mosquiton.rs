//! Canonical Mosquiton animation tags exported from the composed Aseprite source.
//!
//! Gameplay currently uses only the airborne subset because Mosquiton reuses the
//! mosquito behaviour model. The gallery exposes the full authored tag list.

/// Hovering idle used during normal airborne gameplay.
pub const TAG_IDLE_FLY: &str = "idle_fly";
/// Ranged-attack animation used by the reused mosquito shooting behaviour.
pub const TAG_SHOOT_FLY: &str = "shoot_fly";
/// Melee attack animation used if Mosquiton ever enters mosquito melee mode.
pub const TAG_MELEE_FLY: &str = "melee_fly";

/// Full authored tag list exposed in the gallery.
pub const GALLERY_TAGS: &[&str] = &[
    TAG_IDLE_FLY,
    TAG_SHOOT_FLY,
    TAG_MELEE_FLY,
    "death_fly",
    "liftoff",
    "idle_stand",
    "walking_forward",
    "shoot_stand",
    "melee_stand",
    "death",
];
