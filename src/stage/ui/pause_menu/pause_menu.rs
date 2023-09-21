use bevy::prelude::*;

#[derive(Component)]
///pause menu: self
pub struct PauseMenu;

#[derive(Component)]
///current score
pub struct ScoreText;

#[derive(Component)]
///pause menu: text
pub struct InfoText;

#[derive(Component)]
///pause menu: BG
pub struct UIBackground;