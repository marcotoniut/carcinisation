use bevy::prelude::*;
use bevy::reflect::Reflect;
use leafwing_input_manager::Actionlike;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum CutsceneInput {
    Skip,
}
