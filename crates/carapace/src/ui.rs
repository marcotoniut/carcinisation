//! `carapace`'s UI system. The building blocks of UI are here, but they are all just pieces.
//! For example, there is a [`CxTextField`] component, but if you spawn it on its own, the text
//! field won't have a background, and you won't even be able to type in it. Instead, you should
//! make your own helper functions that compose UI components together. For a text field, you could
//! use a [`CxStack`] with a white [`CxFilterRect`] background and a [`CxTextField`], and add an observer
//! on [`CxFilterRect`] that sets [`InputFocus`] to the text field.
//!
//! For more information, browse this module and see the `ui` example.

mod input;
mod layout;
mod widgets;

use bevy_ecs::schedule::common_conditions::any_with_component;
#[cfg(feature = "headed")]
use bevy_ecs::schedule::common_conditions::on_message;
#[cfg(feature = "headed")]
use bevy_input::{InputSystems, keyboard::KeyboardInput, mouse::MouseWheel};
#[cfg(feature = "headed")]
use bevy_input_focus::InputFocus;

use crate::{prelude::*, set::CxSet};

pub use input::{CxCaret, CxKeyField, CxKeyFieldUpdate, CxTextField, CxTextFieldUpdate};
pub use widgets::{
    CxGrid, CxGridRow, CxGridTracks, CxMargin, CxMinSize, CxRow, CxRowSlot, CxScroll, CxStack,
    CxUiRoot,
};

pub(crate) fn plug<L: CxLayer>(app: &mut App) {
    #[cfg(feature = "headed")]
    app.add_systems(
        PreUpdate,
        (
            input::update_key_fields
                .run_if(resource_exists::<InputFocus>)
                .run_if(on_message::<KeyboardInput>),
            input::update_text_fields
                .run_if(resource_exists::<InputFocus>)
                .run_if(on_message::<KeyboardInput>),
            input::scroll.run_if(on_message::<MouseWheel>),
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
            input::caret_blink.run_if(any_with_component::<CxTextField>),
            layout::layout::<L>
                .before(CxSet::Picking)
                .run_if(layout::layout_needs_recompute),
        )
            .chain(),
    );
}

#[cfg(test)]
mod tests {
    use std::{panic::AssertUnwindSafe, time::Duration};

    use bevy_ecs::{schedule::Schedule, schedule::common_conditions::any_with_component};
    use bevy_time::{Time, Timer, TimerMode};

    use super::*;

    #[cfg(feature = "headed")]
    #[derive(
        bevy_render::extract_component::ExtractComponent,
        Component,
        next::Next,
        Ord,
        PartialOrd,
        Eq,
        PartialEq,
        Clone,
        Default,
        Debug,
    )]
    #[next(path = next::Next)]
    enum TestLayer {
        #[default]
        Ui,
    }

    #[derive(Resource, Default)]
    struct LayoutRuns(u32);

    fn count_layout_runs(mut runs: ResMut<LayoutRuns>) {
        runs.0 += 1;
    }

    fn setup_layout_world() -> World {
        let mut world = World::new();
        world.insert_resource(LayoutRuns::default());
        world.insert_resource(crate::position::InsertDefaultLayer::noop());
        world.insert_resource(Assets::<CxTypeface>::default());
        world.insert_resource(Assets::<CxSpriteAsset>::default());
        world
    }

    fn setup_layout_schedules() -> (Schedule, Schedule) {
        let mut layout_schedule = Schedule::default();
        layout_schedule.add_systems(count_layout_runs.run_if(layout::layout_needs_recompute));

        let mut caret_schedule = Schedule::default();
        caret_schedule.add_systems(input::caret_blink);
        (layout_schedule, caret_schedule)
    }

    #[cfg(feature = "headed")]
    #[test]
    fn post_update_ui_chain_skips_without_ui_entities() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                input::caret_blink.run_if(any_with_component::<CxTextField>),
                layout::layout::<TestLayer>
                    .before(CxSet::Picking)
                    .run_if(layout::layout_needs_recompute),
            )
                .chain(),
        );

        // Regression guard: this should not panic even with no `Time`/`CxScreen` resources because
        // both systems are skipped when there are no matching UI entities.
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| schedule.run(&mut world)));
        assert!(
            result.is_ok(),
            "post-update UI chain should skip safely when there are no UI entities"
        );
    }

    #[test]
    fn caret_blink_runs_when_text_field_exists() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        world.insert_resource(crate::position::InsertDefaultLayer::noop());

        let entity = world
            .spawn((
                CxTextField {
                    cached_text: "abc".to_string(),
                    caret_char: '|',
                    caret: Some(CxCaret {
                        state: true,
                        timer: Timer::new(Duration::from_millis(1), TimerMode::Repeating),
                    }),
                },
                CxText {
                    value: "abc|".to_string(),
                    typeface: default(),
                    line_breaks: Vec::new(),
                },
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(input::caret_blink.run_if(any_with_component::<CxTextField>));
        schedule.run(&mut world);

        let field = world.get::<CxTextField>(entity).unwrap();
        let text = world.get::<CxText>(entity).unwrap();
        assert!(!field.caret.as_ref().unwrap().state);
        assert_eq!(text.value, "abc");
    }

    #[test]
    fn layout_run_condition_skips_after_initial_unchanged_frame() {
        let mut world = setup_layout_world();

        let root = world.spawn(CxUiRoot).id();
        world.flush();

        let mut schedule = Schedule::default();
        schedule.add_systems(count_layout_runs.run_if(layout::layout_needs_recompute));

        schedule.run(&mut world);
        assert_eq!(world.resource::<LayoutRuns>().0, 1);

        world.clear_trackers();
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<LayoutRuns>().0,
            1,
            "layout should not re-run when tracked UI inputs have not changed"
        );

        world.entity_mut(root).insert(CxMinSize(UVec2::splat(8)));
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<LayoutRuns>().0,
            2,
            "layout should re-run after tracked UI component changes"
        );

        world.clear_trackers();
        world.entity_mut(root).insert(CxText {
            value: "hello".into(),
            typeface: default(),
            line_breaks: Vec::new(),
        });
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<LayoutRuns>().0,
            3,
            "layout should re-run when text content changes"
        );
    }

    #[test]
    fn layout_run_condition_ignores_caret_timer_only_updates() {
        let mut world = setup_layout_world();
        world.init_resource::<Time>();

        world.spawn(CxUiRoot);
        let field = world
            .spawn((
                CxTextField {
                    cached_text: "abc".to_string(),
                    caret_char: '|',
                    caret: Some(CxCaret {
                        state: true,
                        timer: Timer::new(Duration::from_millis(500), TimerMode::Repeating),
                    }),
                },
                CxText {
                    value: "abc|".to_string(),
                    typeface: default(),
                    line_breaks: Vec::new(),
                },
            ))
            .id();
        world.flush();

        let (mut layout_schedule, mut caret_schedule) = setup_layout_schedules();

        layout_schedule.run(&mut world);
        assert_eq!(world.resource::<LayoutRuns>().0, 1);

        world.clear_trackers();
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(100));
        caret_schedule.run(&mut world);

        let text = world.get::<CxText>(field).unwrap();
        assert_eq!(
            text.value, "abc|",
            "caret timer tick should not mutate text"
        );

        layout_schedule.run(&mut world);
        assert_eq!(
            world.resource::<LayoutRuns>().0,
            1,
            "layout should skip when only caret timer state changes"
        );
    }

    #[test]
    fn layout_run_condition_tracks_caret_text_changes_not_timer_ticks() {
        let mut world = setup_layout_world();
        world.init_resource::<Time>();

        world.spawn(CxUiRoot);
        world.spawn((
            CxTextField {
                cached_text: "abc".to_string(),
                caret_char: '|',
                caret: Some(CxCaret {
                    state: true,
                    timer: Timer::new(Duration::from_millis(500), TimerMode::Repeating),
                }),
            },
            CxText {
                value: "abc|".to_string(),
                typeface: default(),
                line_breaks: Vec::new(),
            },
        ));
        world.flush();

        let (mut layout_schedule, mut caret_schedule) = setup_layout_schedules();

        layout_schedule.run(&mut world);
        world.clear_trackers();

        for _ in 0..60 {
            world
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(100));
            caret_schedule.run(&mut world);
            layout_schedule.run(&mut world);
            world.clear_trackers();
        }

        assert_eq!(
            world.resource::<LayoutRuns>().0,
            13,
            "layout should run on initial frame and each 500ms caret text flip, not every tick"
        );
    }
}
