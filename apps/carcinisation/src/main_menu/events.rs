//! Events exchanged by the main menu state machine.

use bevy::prelude::*;

#[derive(Event, Message)]
/// Fired to initialise the main menu scene.
pub struct MainMenuStartupEvent;

#[derive(Event, Message)]
/// Fired to tear down the main menu scene.
pub struct MainMenuShutdownEvent;
