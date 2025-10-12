//! Lightweight helpers for working with Bevy events and triggers.

use bevy::prelude::*;

/// @trigger Copies trigger payloads into the main Bevy event queue.
pub fn on_trigger_write_event<T: Event + Clone>(
    trigger: Trigger<T>,
    mut event_writer: EventWriter<T>,
) {
    let e = trigger.event();
    event_writer.send(e.clone());
}
