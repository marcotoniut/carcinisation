//! Lightweight helpers for working with Bevy events and triggers.

use bevy::prelude::*;

/// @trigger Copies trigger payloads into the main Bevy event queue.
pub fn on_trigger_write_event<T>(trigger: On<T>, mut message_writer: MessageWriter<T>)
where
    T: Message + Event + Clone,
{
    let e = trigger.event();
    message_writer.write(e.clone());
}
