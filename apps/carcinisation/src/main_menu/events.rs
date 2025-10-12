//! Events exchanged by the main menu state machine.

use bevy::prelude::*;

use super::MainMenuScreen;

#[derive(Event)]
/// Fired to initialise the main menu scene.
pub struct MainMenuStartupEvent;

#[derive(Event)]
/// Fired to tear down the main menu scene.
pub struct MainMenuShutdownEvent;

#[derive(Event)]
/// Switches to a different menu screen.
pub struct ChangeMainMenuScreenTrigger(pub MainMenuScreen);
