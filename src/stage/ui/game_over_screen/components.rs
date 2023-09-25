use bevy::prelude::*;

#[derive(Component)]
pub struct GameOverScreen {}

#[derive(Component)]
///current score
pub struct FinalScoreText;

#[derive(Component)]
///current score
pub struct InfoText;

#[derive(Component)]
///pause menu: text
pub struct LivesText;

#[derive(Component)]
///pause menu: BG
pub struct UIBackground;
