//! Systems managing the lifecycle and movement of letterbox bars.

use super::components::{LetterboxBottom, LetterboxEntity, LetterboxTop};
use super::messages::LetterboxMoveEvent;
use super::resources::LetterboxTimeDomain;
#[cfg(debug_assertions)]
use crate::debug::plugin::{debug_print_shutdown, debug_print_startup};
use crate::globals::SCREEN_RESOLUTION_F32;
use crate::globals::mark_for_despawn_by_query;
use crate::{globals::SCREEN_RESOLUTION, layer::Layer};
use bevy::prelude::*;
use carapace::prelude::*;
use carapace::primitive::{CxPrimitive, CxPrimitiveFill, CxPrimitiveShape};
use carcinisation_cutscene::layer::CutsceneLayer;
use cween::linear::components::{LinearTweenBundle, TargetingValueY};

const DEBUG_MODULE: &str = "Letterbox";

/// @system Spawns the top/bottom letterbox entities when entering the active state.
pub fn on_letterbox_startup(mut commands: Commands) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    commands.spawn((
        Name::new("LetterboxTop"),
        LetterboxEntity,
        LetterboxTop,
        CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(SCREEN_RESOLUTION.x, SCREEN_RESOLUTION.y),
            },
            fill: CxPrimitiveFill::Solid(1),
        },
        CxAnchor::BottomLeft,
        CxRenderSpace::Camera,
        CxPosition::from(IVec2::new(0, SCREEN_RESOLUTION.y as i32)),
        Layer::Cutscene(CutsceneLayer::Letterbox),
        WorldPos(Vec2::new(0., SCREEN_RESOLUTION_F32.y)),
    ));

    commands.spawn((
        Name::new("LetterboxBottom"),
        LetterboxEntity,
        LetterboxBottom,
        CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(SCREEN_RESOLUTION.x, SCREEN_RESOLUTION.y),
            },
            fill: CxPrimitiveFill::Solid(1),
        },
        CxAnchor::TopLeft,
        CxRenderSpace::Camera,
        CxPosition::from(IVec2::new(0, 0)),
        Layer::Cutscene(CutsceneLayer::Letterbox),
        WorldPos(Vec2::ZERO),
    ));
}

/// @system Cleans up letterbox entities when leaving the active state.
pub fn on_letterbox_shutdown(
    mut commands: Commands,
    entity_query: Query<Entity, With<LetterboxEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    mark_for_despawn_by_query(&mut commands, &entity_query);
}

/// @trigger Applies movement instructions to letterbox entities.
///
/// When target is 0.0 (fully hidden), also sets `Visibility::Hidden` so
/// bars are invisible even if the tween hasn't completed.  Any non-zero
/// target restores `Visibility::Visible`.
pub fn on_move(
    trigger: On<LetterboxMoveEvent>,
    mut commands: Commands,
    top_query: Query<(Entity, &WorldPos), With<LetterboxTop>>,
    bottom_query: Query<(Entity, &WorldPos), With<LetterboxBottom>>,
) {
    let e = trigger.event();
    let visibility = if e.target == 0.0 {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };

    for xs in top_query.iter() {
        let target = SCREEN_RESOLUTION_F32.y - e.target;
        insert_linear_movement(&mut commands, xs, target, e.speed);
        commands.entity(xs.0).insert(visibility);
    }

    for xs in bottom_query.iter() {
        insert_linear_movement(&mut commands, xs, e.target, e.speed);
        commands.entity(xs.0).insert(visibility);
    }
}

/// Inserts linear movement towards `target`, preserving direction.
pub fn insert_linear_movement(
    commands: &mut Commands,
    (entity, position): (Entity, &WorldPos),
    target: f32,
    speed: f32,
) {
    let speed = speed * (target - position.y).signum();
    commands
        .entity(entity)
        // TODO review why this was removed
        // .remove::<LinearValueRemovalBundle<LetterboxTimeDomain, TargetingValueY>>()
        .insert(
            LinearTweenBundle::<LetterboxTimeDomain, TargetingValueY>::new(
                position.y, target, speed,
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn minimal letterbox entities with the markers and components
    /// that `on_move` queries require.
    fn spawn_test_letterboxes(world: &mut World) {
        world.spawn((
            LetterboxEntity,
            LetterboxTop,
            WorldPos(Vec2::new(0., 144.)),
            Visibility::Visible,
        ));
        world.spawn((
            LetterboxEntity,
            LetterboxBottom,
            WorldPos(Vec2::ZERO),
            Visibility::Visible,
        ));
    }

    #[test]
    fn hide_sets_visibility_hidden() {
        let mut app = App::new();
        app.add_observer(on_move);

        spawn_test_letterboxes(app.world_mut());
        app.world_mut().flush();

        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::hide());
        app.world_mut().flush();

        for vis in app
            .world_mut()
            .query_filtered::<&Visibility, With<LetterboxEntity>>()
            .iter(app.world())
        {
            assert_eq!(*vis, Visibility::Hidden);
        }
    }

    #[test]
    fn show_restores_visibility_visible() {
        let mut app = App::new();
        app.add_observer(on_move);

        spawn_test_letterboxes(app.world_mut());
        // Start hidden
        for entity in app
            .world_mut()
            .query_filtered::<Entity, With<LetterboxEntity>>()
            .iter(app.world())
            .collect::<Vec<_>>()
        {
            app.world_mut()
                .entity_mut(entity)
                .insert(Visibility::Hidden);
        }
        app.world_mut().flush();

        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::show());
        app.world_mut().flush();

        for vis in app
            .world_mut()
            .query_filtered::<&Visibility, With<LetterboxEntity>>()
            .iter(app.world())
        {
            assert_eq!(*vis, Visibility::Visible);
        }
    }

    #[test]
    fn hide_show_cycle_is_deterministic() {
        let mut app = App::new();
        app.add_observer(on_move);

        spawn_test_letterboxes(app.world_mut());
        app.world_mut().flush();

        // hide
        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::hide());
        app.world_mut().flush();

        for vis in app
            .world_mut()
            .query_filtered::<&Visibility, With<LetterboxEntity>>()
            .iter(app.world())
        {
            assert_eq!(*vis, Visibility::Hidden);
        }

        // show
        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::show());
        app.world_mut().flush();

        for vis in app
            .world_mut()
            .query_filtered::<&Visibility, With<LetterboxEntity>>()
            .iter(app.world())
        {
            assert_eq!(*vis, Visibility::Visible);
        }

        // hide again
        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::hide());
        app.world_mut().flush();

        for vis in app
            .world_mut()
            .query_filtered::<&Visibility, With<LetterboxEntity>>()
            .iter(app.world())
        {
            assert_eq!(*vis, Visibility::Hidden);
        }
    }

    #[test]
    fn entities_survive_simulated_cutscene_shutdown() {
        let mut app = App::new();
        app.add_observer(on_move);

        spawn_test_letterboxes(app.world_mut());
        app.world_mut().flush();

        // Simulate cutscene: show bars
        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::show());
        app.world_mut().flush();

        // Simulate cutscene shutdown: hide bars (no despawn)
        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::hide());
        app.world_mut().flush();

        // Entities still exist
        let count = app
            .world_mut()
            .query_filtered::<(), With<LetterboxEntity>>()
            .iter(app.world())
            .count();
        assert_eq!(
            count, 2,
            "letterbox entities must survive cutscene shutdown"
        );

        // Can show again for next cutscene
        app.world_mut()
            .commands()
            .trigger(LetterboxMoveEvent::show());
        app.world_mut().flush();

        for vis in app
            .world_mut()
            .query_filtered::<&Visibility, With<LetterboxEntity>>()
            .iter(app.world())
        {
            assert_eq!(*vis, Visibility::Visible);
        }
    }
}
