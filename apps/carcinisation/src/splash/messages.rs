//! Splash screen lifecycle triggers.

use bevy::prelude::*;

#[derive(Event)]
/// Fired to start the boot splash screen.
pub struct SplashStartupEvent;
