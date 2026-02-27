//! `carapace`'s UI system. The building blocks of UI are here, but they are all just pieces.
//! For example, there is a [`PxTextField`] component, but if you spawn it on its own, the text
//! field won't have a background, and you won't even be able to type in it. Instead, you should
//! make your own helper functions that compose UI components together. For a text field, you could
//! use a [`PxStack`] with a white [`PxRect`] background and a [`PxTextField`], and add an observer
//! on [`PxRect`] that sets [`InputFocus`] to the text field.
//!
//! For more information, browse this module and see the `ui` example.

mod input;
mod layout;
mod widgets;

#[cfg(feature = "headed")]
use bevy_ecs::schedule::common_conditions::on_message;
#[cfg(feature = "headed")]
use bevy_input::{InputSystems, keyboard::KeyboardInput};
#[cfg(feature = "headed")]
use bevy_input_focus::InputFocus;

use crate::{prelude::*, set::PxSet};

pub use input::{PxCaret, PxKeyField, PxKeyFieldUpdate, PxTextField, PxTextFieldUpdate};
pub use widgets::{
    PxGrid, PxGridRow, PxGridRows, PxMargin, PxMinSize, PxRow, PxRowSlot, PxScroll, PxStack,
    PxUiRoot,
};

pub(crate) fn plug<L: PxLayer>(app: &mut App) {
    #[cfg(feature = "headed")]
    app.add_systems(
        PreUpdate,
        (
            input::update_key_fields.run_if(resource_exists::<InputFocus>),
            input::update_text_fields
                .run_if(resource_exists::<InputFocus>)
                .run_if(on_message::<KeyboardInput>),
            input::scroll,
        )
            .after(InputSystems),
    )
    .add_systems(
        PostUpdate,
        (
            input::update_key_field_focus,
            input::update_text_field_focus.before(input::caret_blink),
        )
            .run_if(resource_exists::<InputFocus>),
    );
    app.add_systems(
        PostUpdate,
        (
            input::caret_blink,
            layout::layout::<L>.before(PxSet::Picking),
        )
            .chain(),
    );
}
