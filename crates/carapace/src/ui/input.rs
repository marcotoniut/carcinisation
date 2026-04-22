use std::time::Duration;

use bevy_ecs::system::SystemId;
#[cfg(feature = "headed")]
use bevy_input::{
    ButtonState,
    keyboard::{Key, KeyboardInput, NativeKey},
    mouse::MouseWheel,
};
#[cfg(feature = "headed")]
use bevy_input_focus::InputFocus;

use crate::{blink::CxBlink, prelude::*};

use super::widgets::CxScroll;

// TODO Should be modular
#[cfg(feature = "headed")]
pub(crate) fn scroll(mut scrolls: Query<&mut CxScroll>, mut wheels: MessageReader<MouseWheel>) {
    for wheel in wheels.read() {
        for mut scroll in &mut scrolls {
            scroll.scroll = scroll
                .scroll
                .saturating_add_signed(-wheel.y as i32)
                .min(scroll.max_scroll);
        }
    }
}

/// Field that captures a single key and renders its label.
#[derive(Component, Reflect)]
#[require(CxText)]
#[reflect(from_reflect = false)]
pub struct CxKeyField {
    /// Placeholder/caret character when focused.
    pub caret: char,
    /// System that creates the text label
    ///
    /// Ideally, this would accept a Bevy `Key`, but there doesn't seem to be a way to convert a
    /// winit `PhysicalKey` to a winit `Key`, so it wouldn't be possible to run this when building
    /// the UI builder (ie in the insertion path) or update all the text if the keyboard layout
    /// changes.
    #[reflect(ignore)]
    pub key_to_str: SystemId<In<KeyCode>, String>,
    /// Last displayed value when unfocused.
    pub cached_text: String,
}

#[cfg(feature = "headed")]
pub(crate) fn update_key_field_focus(
    mut prev_focus: Local<Option<Entity>>,
    mut fields: Query<(&CxKeyField, &mut CxText, &mut Visibility, Entity)>,
    focus: Res<InputFocus>,
    mut cmd: Commands,
) {
    let focus = focus.get();

    if *prev_focus == focus {
        return;
    }

    if let Some(prev_focus) = *prev_focus
        && let Ok((field, mut text, mut visibility, id)) = fields.get_mut(prev_focus)
    {
        text.value = field.cached_text.clone();
        *visibility = Visibility::Inherited;
        cmd.entity(id).remove::<CxBlink>();
    }

    if let Some(focus) = focus
        && let Ok((field, mut text, _, id)) = fields.get_mut(focus)
    {
        text.value.clear();
        text.value.push(field.caret);
        cmd.entity(id)
            .try_insert(CxBlink::new(Duration::from_millis(500)));
    }

    *prev_focus = focus;
}

/// Emitted when a [`CxKeyField`] captures a key press.
#[derive(EntityEvent)]
pub struct CxKeyFieldUpdate {
    /// Target field entity.
    pub entity: Entity,
    /// Captured key.
    pub key: KeyCode,
}

// TODO Should be modular
#[cfg(feature = "headed")]
pub(crate) fn update_key_fields(
    mut fields: Query<Entity, With<CxKeyField>>,
    mut focus: ResMut<InputFocus>,
    mut keys: MessageReader<KeyboardInput>,
    mut cmd: Commands,
) {
    let mut keys = keys.read();
    let key = keys.find(|key| matches!(key.state, ButtonState::Pressed));
    keys.last();
    let Some(key) = key else {
        return;
    };

    let Some(focus_id) = focus.get() else {
        return;
    };

    let Ok(field_id) = fields.get_mut(focus_id) else {
        return;
    };

    let key = key.key_code;

    cmd.queue(move |world: &mut World| {
        let Some(field) = world.get::<CxKeyField>(field_id) else {
            return;
        };

        let key = match world.run_system_with(field.key_to_str, key) {
            Ok(key) => key,
            Err(err) => {
                error!("couldn't get text for pressed key for key field: {err}");
                return;
            }
        };

        if let Some(mut field) = world.get_mut::<CxKeyField>(field_id) {
            field.cached_text.clone_from(&key);
        }

        if let Some(mut text) = world.get_mut::<CxText>(field_id) {
            text.value = key;
        }
    });

    cmd.trigger(CxKeyFieldUpdate {
        entity: field_id,
        key,
    });

    focus.clear();
}

/// Caret blink state for text fields.
#[derive(Reflect)]
pub struct CxCaret {
    /// Whether the caret is currently visible.
    pub state: bool,
    /// `CxBlink` timer.
    pub timer: Timer,
}

impl Default for CxCaret {
    fn default() -> Self {
        Self {
            state: true,
            timer: Timer::new(Duration::from_millis(500), TimerMode::Repeating),
        }
    }
}

/// Editable text field with an optional blinking caret.
#[derive(Component, Reflect)]
#[require(CxText)]
pub struct CxTextField {
    /// Cached text without the caret character.
    pub cached_text: String,
    /// Character used as the caret.
    pub caret_char: char,
    /// Active caret state if focused.
    pub caret: Option<CxCaret>,
}

#[cfg(feature = "headed")]
pub(crate) fn update_text_field_focus(
    mut prev_focus: Local<Option<Entity>>,
    mut fields: Query<(&mut CxTextField, &mut CxText)>,
    focus: Res<InputFocus>,
) {
    let focus = focus.get();

    if *prev_focus == focus {
        return;
    }

    if let Some(prev_focus) = *prev_focus
        && let Ok((mut field, mut text)) = fields.get_mut(prev_focus)
    {
        text.value = field.cached_text.clone();
        field.caret = None;
    }

    if let Some(focus) = focus
        && let Ok((mut field, mut text)) = fields.get_mut(focus)
    {
        field.cached_text.clone_from(&text.value);
        text.value.push(field.caret_char);
        field.caret = Some(default());
    }

    *prev_focus = focus;
}

pub(crate) fn caret_blink(mut fields: Query<(&mut CxTextField, &mut CxText)>, time: Res<Time>) {
    for (mut field, mut text) in &mut fields {
        let Some(ref mut caret) = field.caret else {
            continue;
        };

        caret.timer.tick(time.delta());

        if caret.timer.just_finished() {
            caret.state ^= true;
            let state = caret.state;

            text.value.clone_from(&field.cached_text);

            if state {
                text.value.push(field.caret_char);
            }
        }
    }
}

/// Emitted when a [`CxTextField`] changes its text.
#[derive(EntityEvent)]
pub struct CxTextFieldUpdate {
    /// Target field entity.
    pub entity: Entity,
    /// Updated text content.
    pub text: String,
}

// TODO Should be modular
#[cfg(feature = "headed")]
pub(crate) fn update_text_fields(
    mut fields: Query<(&mut CxTextField, &mut CxText)>,
    focus: Res<InputFocus>,
    mut keys: MessageReader<KeyboardInput>,
    mut cmd: Commands,
) {
    let Some(focus_id) = focus.get() else {
        keys.read().for_each(drop);
        return;
    };

    let Ok((mut field, mut text)) = fields.get_mut(focus_id) else {
        keys.read().for_each(drop);
        return;
    };

    let mut changed = false;
    for key in keys.read() {
        if !matches!(key.state, ButtonState::Pressed) {
            continue;
        }
        match key.logical_key {
            Key::Character(ref characters) | Key::Unidentified(NativeKey::Web(ref characters)) => {
                for character in characters.chars() {
                    field.cached_text.push(character);
                    changed = true;
                }
            }
            Key::Space => {
                field.cached_text.push(' ');
                changed = true;
            }
            Key::Backspace => {
                changed |= field.cached_text.pop().is_some();
            }
            _ => (),
        }
    }
    if !changed {
        return;
    }

    text.value.clone_from(&field.cached_text);
    text.value.push(field.caret_char);
    field.caret = Some(default());

    cmd.trigger(CxTextFieldUpdate {
        entity: focus_id,
        text: field.cached_text.clone(),
    });
}

#[cfg(all(test, feature = "headed"))]
mod tests {
    use super::*;
    use bevy_ecs::{message::Messages, schedule::Schedule};

    fn test_world() -> World {
        let mut world = World::new();
        world.init_resource::<InputFocus>();
        world.init_resource::<Messages<KeyboardInput>>();
        // `CxText` requires `DefaultLayer`; this no-op keeps setup focused on text-input behavior.
        world.insert_resource(crate::position::InsertDefaultLayer::noop());
        world
    }

    fn spawn_text_field(world: &mut World) -> Entity {
        world
            .spawn((
                CxTextField {
                    cached_text: String::new(),
                    caret_char: '|',
                    caret: None,
                },
                CxText::default(),
            ))
            .id()
    }

    fn key_input_char(character: &str) -> KeyboardInput {
        KeyboardInput {
            key_code: bevy_input::keyboard::KeyCode::KeyA,
            logical_key: Key::Character(character.into()),
            state: ButtonState::Pressed,
            text: Some(character.into()),
            repeat: false,
            window: Entity::from_raw_u32(1).unwrap(),
        }
    }

    #[test]
    fn text_input_does_not_replay_after_focus_is_restored() {
        let mut world = test_world();
        let text_field = spawn_text_field(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems(update_text_fields);

        world
            .resource_mut::<Messages<KeyboardInput>>()
            .write(key_input_char("a"));
        schedule.run(&mut world);

        world.resource_mut::<InputFocus>().set(text_field);
        schedule.run(&mut world);

        let field = world.get::<CxTextField>(text_field).unwrap();
        assert!(field.cached_text.is_empty());
    }

    #[test]
    fn text_input_does_not_replay_after_invalid_focus_entity() {
        let mut world = test_world();
        let text_field = spawn_text_field(&mut world);
        let invalid_focus = world.spawn_empty().id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_text_fields);

        world.resource_mut::<InputFocus>().set(invalid_focus);
        world
            .resource_mut::<Messages<KeyboardInput>>()
            .write(key_input_char("a"));
        schedule.run(&mut world);

        world.resource_mut::<InputFocus>().set(text_field);
        schedule.run(&mut world);

        let field = world.get::<CxTextField>(text_field).unwrap();
        assert!(field.cached_text.is_empty());
    }
}
