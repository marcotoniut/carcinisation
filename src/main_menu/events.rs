use bevy::prelude::*;

use super::MainMenuScreen;

#[derive(Event)]
pub struct MainMenuStartupEvent;

#[derive(Event)]
pub struct MainMenuShutdownEvent;

#[derive(Event)]
pub struct ChangeMainMenuScreenEvent(pub MainMenuScreen);
