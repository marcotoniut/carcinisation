use bevy::prelude::*;

#[derive(Component)]
pub struct ClearedScreen {}

#[derive(Component)]
///current score
pub struct ScoreText;

#[derive(Component)]
///current score
pub struct InfoText;

#[derive(Component)]
///pause menu: text
pub struct LivesText;

#[derive(Component)]
///pause menu: BG
pub struct UIBackground;
