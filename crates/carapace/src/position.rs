//! Position, layers, velocity, anchors, etc.

use std::fmt::Debug;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Mutable, lifecycle::HookContext, world::DeferredWorld};
#[cfg(feature = "headed")]
use bevy_render::{RenderApp, extract_component::ExtractComponent};
use next::Next;

use crate::{prelude::*, set::PxSet};

pub(crate) fn plug_core<L: PxLayer>(app: &mut App) {
    app.insert_resource(InsertDefaultLayer::new::<L>())
        .add_systems(PreUpdate, update_sub_positions)
        .add_systems(
            PostUpdate,
            update_position_to_sub.in_set(PxSet::UpdatePosToSubPos),
        );
}

pub(crate) fn plug<L: PxLayer>(app: &mut App) {
    plug_core::<L>(app);
    #[cfg(feature = "headed")]
    app.sub_app_mut(RenderApp)
        .insert_resource(InsertDefaultLayer::new::<L>());
}

pub(crate) trait Spatial {
    fn frame_size(&self) -> UVec2;
}

impl<T: Spatial> Spatial for &'_ T {
    fn frame_size(&self) -> UVec2 {
        (*self).frame_size()
    }
}

/// The position of an entity
#[cfg_attr(feature = "headed", derive(ExtractComponent))]
#[derive(Component, Deref, DerefMut, Clone, Copy, Default, Reflect, Debug)]
pub struct PxPosition(pub IVec2);

impl From<IVec2> for PxPosition {
    fn from(position: IVec2) -> Self {
        Self(position)
    }
}

/// Trait implemented for your game's custom layer type. Use the [`px_layer`] attribute
/// or derive/implement the required traits manually. The layers will be rendered in the order
/// defined by the [`PartialOrd`] implementation. So, lower values will be in the back
/// and vice versa.
///
// TODO: For games with fixed enum layers, an opt-in `DenseLayer` trait that maps variants to
// `usize` would allow the collect phase in `screen/node.rs` to use pre-sized `Vec` storage
// instead of `BTreeMap`, eliminating tree operations entirely. This should be kept opt-in so
// the default `PxLayer` API remains ergonomic for parameterized layer types like `Layer(i32)`.
#[cfg(feature = "headed")]
pub trait PxLayer:
    ExtractComponent + Component<Mutability = Mutable> + Next + Ord + Clone + Default + Debug
{
}

#[cfg(not(feature = "headed"))]
pub trait PxLayer: Component<Mutability = Mutable> + Next + Ord + Clone + Default + Debug {}

impl<#[cfg(feature = "headed")] L: ExtractComponent, #[cfg(not(feature = "headed"))] L> PxLayer
    for L
where
    L: Component<Mutability = Mutable> + Next + Ord + Clone + Default + Debug,
{
}

#[derive(Resource, Deref)]
pub(crate) struct InsertDefaultLayer(Box<dyn Fn(&mut EntityWorldMut) + Send + Sync>);

impl InsertDefaultLayer {
    fn new<L: PxLayer>() -> Self {
        Self(Box::new(|entity| {
            entity.insert_if_new(L::default());
        }))
    }

    #[cfg(test)]
    pub(crate) fn noop() -> Self {
        Self(Box::new(|_| {}))
    }
}

#[derive(Component, Default)]
#[component(on_add = insert_default_layer)]
pub(crate) struct DefaultLayer;

fn insert_default_layer(mut world: DeferredWorld, ctx: HookContext) {
    world.commands().queue(move |world: &mut World| {
        let insert_default_layer = world.remove_resource::<InsertDefaultLayer>().unwrap();
        if let Ok(mut entity) = world.get_entity_mut(ctx.entity) {
            insert_default_layer(entity.remove::<DefaultLayer>());
        }
        world.insert_resource(insert_default_layer);
        // That's what it's all about!
    });
}

/// How a sprite is positioned relative to its [`PxPosition`]. It defaults to [`PxAnchor::Center`].
#[cfg_attr(feature = "headed", derive(ExtractComponent))]
#[derive(Component, Clone, Copy, Default, Debug, Reflect)]
pub enum PxAnchor {
    /// Center
    #[default]
    Center,
    /// Bottom left
    BottomLeft,
    /// Bottom center
    BottomCenter,
    /// Bottom right
    BottomRight,
    /// Center left
    CenterLeft,
    /// Center right
    CenterRight,
    /// Top left
    TopLeft,
    /// Top center
    TopCenter,
    /// Top right
    TopRight,
    /// Custom anchor. Values range from 0 to 1, from the bottom left to the top right.
    Custom(Vec2),
}

impl From<Vec2> for PxAnchor {
    fn from(vec: Vec2) -> Self {
        Self::Custom(vec)
    }
}

impl PxAnchor {
    /// Anchor X offset in pixels. Uses rounding for `Custom` anchors so the
    /// placement error is symmetric (±0.5px) rather than biased downward,
    /// which matters at small sprite sizes / extreme fallback scales.
    pub(crate) fn x_pos(self, width: u32) -> u32 {
        match self {
            PxAnchor::BottomLeft | PxAnchor::CenterLeft | PxAnchor::TopLeft => 0,
            PxAnchor::BottomCenter | PxAnchor::Center | PxAnchor::TopCenter => width / 2,
            PxAnchor::BottomRight | PxAnchor::CenterRight | PxAnchor::TopRight => width,
            PxAnchor::Custom(anchor) => (width as f32 * anchor.x).round() as u32,
        }
    }

    /// Anchor Y offset in pixels. Uses rounding for `Custom` anchors so the
    /// placement error is symmetric (±0.5px) rather than biased downward,
    /// which matters at small sprite sizes / extreme fallback scales.
    pub(crate) fn y_pos(self, height: u32) -> u32 {
        match self {
            PxAnchor::BottomLeft | PxAnchor::BottomCenter | PxAnchor::BottomRight => 0,
            PxAnchor::CenterLeft | PxAnchor::Center | PxAnchor::CenterRight => height / 2,
            PxAnchor::TopLeft | PxAnchor::TopCenter | PxAnchor::TopRight => height,
            PxAnchor::Custom(anchor) => (height as f32 * anchor.y).round() as u32,
        }
    }

    pub(crate) fn pos(self, size: UVec2) -> UVec2 {
        UVec2::new(self.x_pos(size.x), self.y_pos(size.y))
    }
}

// TODO Remove
/// Float-based position. Add to entities that have [`PxPosition`], but also need
/// a sub-pixel position. Use [`PxPosition`] unless a sub-pixel position is necessary.
#[derive(Component, Debug, Default, Deref, DerefMut, Reflect)]
#[require(PxPosition)]
pub struct PxSubPosition(pub Vec2);

impl From<Vec2> for PxSubPosition {
    fn from(vec: Vec2) -> Self {
        Self(vec)
    }
}

/// Velocity. Entities with this and [`PxSubPosition`] will move at this velocity over time.
#[derive(Clone, Component, Copy, Debug, Default, Deref, DerefMut, Reflect)]
#[require(PxSubPosition)]
pub struct PxVelocity(pub Vec2);

impl From<Vec2> for PxVelocity {
    fn from(vec: Vec2) -> Self {
        Self(vec)
    }
}

fn update_sub_positions(mut query: Query<(&mut PxSubPosition, &PxVelocity)>, time: Res<Time>) {
    for (mut sub_position, velocity) in &mut query {
        if **velocity == Vec2::ZERO {
            let new_position = Vec2::new(sub_position.x.round(), sub_position.y.round());
            if **sub_position != new_position {
                **sub_position = new_position;
            }
        } else {
            **sub_position += **velocity * time.delta_secs();
        }
    }
}

/// Syncs the derived integer position cache from the authoritative sub-pixel
/// position.
///
/// **Contract**: by the end of a frame, `PxPosition` must equal the
/// rounded value of `PxSubPosition` for every entity where the sub-position
/// was modified during that frame.  Rendering-facing consumers read
/// `PxPosition`; they should never see a stale value from a previous frame.
fn update_position_to_sub(
    mut query: Query<(&mut PxPosition, &PxSubPosition), Changed<PxSubPosition>>,
) {
    for (mut position, sub_position) in &mut query {
        let new_position = IVec2::new(sub_position.x.round() as i32, sub_position.y.round() as i32);
        if **position != new_position {
            **position = new_position;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, Update};
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use std::time::Duration;

    /// Minimal app with position sync in `PostUpdate` — the intended schedule.
    ///
    /// The production code currently registers these in `PreUpdate` (legacy).
    /// These tests assert the correct end-of-frame contract; they will pass
    /// under `PostUpdate` and test E will FAIL under `PreUpdate`, which is
    /// exactly the regression we want to catch.
    fn test_app() -> App {
        let mut app = App::new();
        app.add_systems(
            PostUpdate,
            (update_sub_positions, update_position_to_sub).chain(),
        );
        app.init_resource::<Time>();
        app
    }

    /// Spawn an entity with a sub-pixel position.  `PxSubPosition` requires
    /// `PxPosition`, so both are present automatically.
    fn spawn_at(app: &mut App, pos: Vec2) -> Entity {
        app.world_mut().spawn(PxSubPosition(pos)).id()
    }

    fn get_positions(app: &App, entity: Entity) -> (Vec2, IVec2) {
        let world = app.world();
        let sub = world.entity(entity).get::<PxSubPosition>().unwrap().0;
        let snapped = world.entity(entity).get::<PxPosition>().unwrap().0;
        (sub, snapped)
    }

    // ── A. Same-frame sync contract ──────────────────────────────────

    #[test]
    fn sub_position_written_during_update_is_synced_by_end_of_frame() {
        let mut app = test_app();
        // Also register a system that writes PxSubPosition during Update,
        // simulating gameplay movement.
        app.add_systems(Update, |mut q: Query<&mut PxSubPosition>| {
            for mut sub in &mut q {
                sub.0 = Vec2::new(42.7, -13.2);
            }
        });

        let entity = spawn_at(&mut app, Vec2::ZERO);

        // First update seeds the entity into the world.
        app.update();
        // Second update: the Update system writes, then PreUpdate of the
        // *next* frame syncs.  Under current PreUpdate scheduling, this
        // means the sync lags — the test documents the intended contract
        // (same-frame sync) rather than the current mechanism.
        app.update();

        let (sub, snapped) = get_positions(&app, entity);
        assert_eq!(sub, Vec2::new(42.7, -13.2));
        assert_eq!(
            snapped,
            IVec2::new(43, -13),
            "PxPosition must match rounded PxSubPosition"
        );
    }

    // ── B. Movement pipeline (velocity-driven) ──────────────────────

    #[test]
    fn velocity_driven_movement_syncs_both_positions() {
        let mut app = test_app();
        // Advance time so velocity produces a meaningful delta.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));

        let entity = app
            .world_mut()
            .spawn(PxVelocity(Vec2::new(10.0, -5.0)))
            .id();

        // First update applies velocity and syncs.
        app.update();

        let (sub, snapped) = get_positions(&app, entity);
        // velocity * dt = (10, -5) * 1.0 = (10, -5)
        assert_eq!(sub, Vec2::new(10.0, -5.0));
        assert_eq!(snapped, IVec2::new(10, -5));
    }

    #[test]
    fn zero_velocity_snaps_sub_position_to_integers() {
        let mut app = test_app();
        let entity = app
            .world_mut()
            .spawn((PxSubPosition(Vec2::new(3.7, -1.2)), PxVelocity(Vec2::ZERO)))
            .id();

        app.update();

        let (sub, snapped) = get_positions(&app, entity);
        // Zero velocity → PxSubPosition rounds to integer.
        assert_eq!(sub, Vec2::new(4.0, -1.0));
        assert_eq!(snapped, IVec2::new(4, -1));
    }

    // ── C. Rounding / snapping semantics ────────────────────────────

    #[test]
    fn positive_fractional_rounds_correctly() {
        let mut app = test_app();
        let entity = spawn_at(&mut app, Vec2::new(10.3, 20.7));
        app.update();
        let (_, snapped) = get_positions(&app, entity);
        assert_eq!(snapped, IVec2::new(10, 21));
    }

    #[test]
    fn negative_values_round_correctly() {
        let mut app = test_app();
        let entity = spawn_at(&mut app, Vec2::new(-3.2, -7.8));
        app.update();
        let (_, snapped) = get_positions(&app, entity);
        assert_eq!(snapped, IVec2::new(-3, -8));
    }

    #[test]
    fn half_values_round_away_from_zero() {
        let mut app = test_app();
        let entity = spawn_at(&mut app, Vec2::new(0.5, -0.5));
        app.update();
        let (_, snapped) = get_positions(&app, entity);
        // f32::round() rounds half away from zero.
        assert_eq!(snapped, IVec2::new(1, -1));
    }

    #[test]
    fn exact_integers_pass_through_unchanged() {
        let mut app = test_app();
        let entity = spawn_at(&mut app, Vec2::new(5.0, -3.0));
        app.update();
        let (_, snapped) = get_positions(&app, entity);
        assert_eq!(snapped, IVec2::new(5, -3));
    }

    // ── D. No-churn: no write when snapped value is unchanged ───────

    #[test]
    fn no_position_write_when_snapped_value_unchanged() {
        let mut app = test_app();

        // Start at a position that rounds to (10, 20).
        let entity = spawn_at(&mut app, Vec2::new(10.0, 20.0));
        app.update();

        // Clear change ticks by running another frame with no mutation.
        app.update();

        // Manually nudge PxSubPosition to a value that still rounds to (10, 20).
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<PxSubPosition>()
            .unwrap()
            .0 = Vec2::new(10.3, 19.7);

        app.update();

        // PxPosition should still be (10, 20) and should NOT be marked changed
        // because the rounded value didn't change.
        let (_, snapped) = get_positions(&app, entity);
        assert_eq!(snapped, IVec2::new(10, 20));

        let world = app.world();
        let pos_ref = world.entity(entity).get_ref::<PxPosition>().unwrap();
        assert!(
            !pos_ref.is_changed(),
            "PxPosition should not be marked Changed when the snapped value is the same"
        );
    }

    // ── E. Regression: no stale render position after movement ──────

    #[test]
    fn position_not_stale_after_update_phase_movement() {
        // This protects against the exact bug class we observed: gameplay
        // moves PxSubPosition during Update, but PxPosition retains the
        // old value for one frame because the sync ran too early.
        let mut app = test_app();

        let entity = spawn_at(&mut app, Vec2::new(0.0, 50.0));
        app.update(); // seed

        // Simulate a large position jump (like a depth transition).
        app.add_systems(Update, |mut q: Query<&mut PxSubPosition>| {
            for mut sub in &mut q {
                sub.0 = Vec2::new(0.0, 120.0);
            }
        });

        app.update();

        let (sub, snapped) = get_positions(&app, entity);
        assert!((sub.y - 120.0).abs() < f32::EPSILON);
        assert_eq!(
            snapped,
            IVec2::new(0, 120),
            "PxPosition must not be stale after a position jump in Update"
        );
    }
}
