use bevy::prelude::*;

pub fn on_trigger_write_event<T: Event + Clone>(
    trigger: Trigger<T>,
    mut event_writer: EventWriter<T>,
) {
    let e = trigger.event();
    event_writer.send(e.clone());
}
