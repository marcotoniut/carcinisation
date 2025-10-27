use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "derive-ts")]
use ts_rs::TS;

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
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
}
