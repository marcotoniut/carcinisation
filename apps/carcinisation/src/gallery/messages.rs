//! Gallery event types.

use bevy::prelude::*;

/// Triggers gallery scene initialisation.
#[derive(Event, Message, Debug)]
pub struct GalleryStartupEvent;
