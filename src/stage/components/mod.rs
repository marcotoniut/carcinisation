pub mod damage;
pub mod interactive;
pub mod placement;

use bevy::prelude::*;

use super::data::{ContainerSpawn, StageSpawn};

// TODO should go in UI
#[derive(Clone, Component, Debug)]
pub struct StageClearedText {}

#[derive(Clone, Component, Debug)]
pub struct SpawnDrop {
    pub contains: ContainerSpawn,
    pub entity: Entity,
}

#[derive(Component)]
pub struct Stage;
