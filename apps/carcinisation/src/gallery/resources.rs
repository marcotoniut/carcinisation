//! Gallery state and character selection model.

use crate::stage::components::placement::Depth;
use bevy::prelude::*;
use strum::IntoEnumIterator;

/// All characters viewable in the gallery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GalleryCharacter {
    #[default]
    Mosquito,
    Mosquiton,
    Tardigrade,
    Spidey,
    Marauder,
    Spidomonsta,
    Kyle,
}

impl GalleryCharacter {
    #[must_use]
    pub const fn all() -> &'static [GalleryCharacter] {
        &[
            GalleryCharacter::Mosquito,
            GalleryCharacter::Mosquiton,
            GalleryCharacter::Tardigrade,
            GalleryCharacter::Spidey,
            GalleryCharacter::Marauder,
            GalleryCharacter::Spidomonsta,
            GalleryCharacter::Kyle,
        ]
    }

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Mosquito => "Mosquito",
            Self::Mosquiton => "Mosquiton",
            Self::Tardigrade => "Tardigrade",
            Self::Spidey => "Spidey",
            Self::Marauder => "Marauder (N/A)",
            Self::Spidomonsta => "Spidomonsta (N/A)",
            Self::Kyle => "Kyle (N/A)",
        }
    }

    #[must_use]
    pub fn available_animations(self) -> Vec<String> {
        match self {
            Self::Mosquito => vec![
                "idle".into(),
                "fly".into(),
                "melee_attack".into(),
                "death".into(),
            ],
            Self::Mosquiton => vec!["idle_stand".into(), "shoot_stand".into()],
            Self::Tardigrade => vec![
                "idle".into(),
                "attack".into(),
                "sucking".into(),
                "death".into(),
            ],
            Self::Spidey => vec!["idle".into(), "death".into()],
            Self::Marauder | Self::Spidomonsta | Self::Kyle => vec![],
        }
    }

    /// The default depth for this character's assets.
    #[must_use]
    pub const fn default_depth(self) -> Depth {
        match self {
            Self::Mosquiton => Depth::Three,
            Self::Tardigrade => Depth::Six,
            Self::Mosquito | Self::Spidey | Self::Marauder | Self::Spidomonsta | Self::Kyle => {
                Depth::Five
            }
        }
    }

    /// Returns all depths that have assets for this character.
    #[must_use]
    pub fn available_depths(self) -> Vec<Depth> {
        match self {
            Self::Mosquito => vec![
                Depth::Three,
                Depth::Four,
                Depth::Five,
                Depth::Six,
                Depth::Seven,
                Depth::Eight,
            ],
            Self::Mosquiton => vec![Depth::Three],
            Self::Tardigrade => vec![Depth::Six, Depth::Seven, Depth::Eight],
            Self::Spidey => vec![
                Depth::Two,
                Depth::Three,
                Depth::Four,
                Depth::Five,
                Depth::Six,
                Depth::Seven,
            ],
            Self::Marauder | Self::Spidomonsta | Self::Kyle => vec![],
        }
    }

    /// Whether this character has assets that can be displayed.
    #[must_use]
    pub const fn has_assets(self) -> bool {
        matches!(
            self,
            Self::Mosquito | Self::Mosquiton | Self::Tardigrade | Self::Spidey
        )
    }
}

/// Persistent gallery UI state: tracks current selection and previous values for change detection.
#[derive(Resource, Debug)]
pub struct GalleryState {
    pub selected_character: GalleryCharacter,
    pub selected_animation: String,
    pub selected_depth: Depth,
    pub prev_character: Option<GalleryCharacter>,
    pub prev_animation: Option<String>,
    pub prev_depth: Option<Depth>,
    /// Whether the current animation is paused.
    pub paused: bool,
    /// Frame index to display when paused (0-based).
    pub selected_frame: usize,
    /// Total frame count of the current animation (updated by the apply systems).
    pub frame_count: usize,
}

impl Default for GalleryState {
    fn default() -> Self {
        let character = GalleryCharacter::Mosquito;
        let animation = character
            .available_animations()
            .into_iter()
            .next()
            .unwrap_or_default();

        Self {
            selected_character: character,
            selected_animation: animation,
            selected_depth: character.default_depth(),
            prev_character: None,
            prev_animation: None,
            prev_depth: None,
            paused: false,
            selected_frame: 0,
            frame_count: 0,
        }
    }
}

/// Returns all depth values (0–9) for the slider.
#[must_use]
pub fn all_depths() -> Vec<Depth> {
    Depth::iter().collect()
}
