use bevy::prelude::*;

/// Triggers persistence of the current scene path.
#[derive(Event, Message)]
pub struct WriteRecentFilePathEvent;
