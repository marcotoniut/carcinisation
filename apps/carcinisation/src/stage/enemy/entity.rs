use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
pub enum EnemyType {
    #[default]
    Mosquito,
    Spidey,
    Tardigrade,
    Marauder,
    Spidomonsta,
    Kyle,
}

impl EnemyType {
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }

    pub fn show_type(&self) -> String {
        format!("Enemy<{:?}>", self)
    }

    /// Returns the sprite base name for this enemy type
    pub fn sprite_base_name(&self) -> &'static str {
        match self {
            EnemyType::Mosquito => "mosquito",
            EnemyType::Spidey => "spidey",
            EnemyType::Tardigrade => "tardigrade",
            EnemyType::Marauder => "marauder",
            EnemyType::Spidomonsta => "spidomonsta",
            EnemyType::Kyle => "kyle",
        }
    }
}
