use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::stage::components::placement::Depth;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
pub enum EnemyType {
    #[default]
    Mosquito,
    Mosquiton,
    Spidey,
    Tardigrade,
    Marauder,
    Spidomonsta,
    Kyle,
}

impl EnemyType {
    #[must_use]
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }

    #[must_use]
    pub fn show_type(&self) -> String {
        format!("Enemy<{self:?}>")
    }

    /// Returns the sprite base name for this enemy type
    #[must_use]
    pub const fn sprite_base_name(&self) -> &'static str {
        match self {
            Self::Mosquito => "mosquito",
            Self::Mosquiton => "mosquiton",
            Self::Spidey => "spidey",
            Self::Tardigrade => "tardigrade",
            Self::Marauder => "marauder",
            Self::Spidomonsta => "spidomonsta",
            Self::Kyle => "kyle",
        }
    }

    /// Returns the base authored depth for composed-animation enemy types.
    ///
    /// Assets are authored at a single canonical depth; other depths use
    /// fallback scaling via [`DepthScaleConfig`]. Returns `None` for
    /// non-composed enemy types (e.g. regular Mosquito uses per-depth sprites).
    #[must_use]
    pub const fn composed_authored_depth(&self) -> Option<Depth> {
        match self {
            Self::Mosquiton | Self::Spidey => Some(Depth::Three),
            _ => None,
        }
    }
}
