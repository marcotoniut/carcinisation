//! Sprite lateral parallax and presentation offset composition.
//!
//! Entities closer to the camera (lower Y / foreground) shift more than distant
//! ones (higher Y / horizon) when the camera pans laterally.  Gameplay positions
//! ([`WorldPos`] / [`CxPosition`]) are never modified — the shift lives in
//! [`CxPresentationTransform`] offset fields.
//!
//! # Components
//!
//! - [`ParallaxOffset`] — per-entity parallax contribution (**collision-affecting**),
//!   recomputed every frame by [`update_parallax_offset`].
//! - [`NoParallax`] — opt-out marker.
//!
//! # Composition model
//!
//! [`compose_presentation_offsets`] is the **sole writer** to the composed offset
//! fields on [`CxPresentationTransform`].  It produces two offsets:
//!
//! - `collision_offset` = sum of collision-affecting contributors (parallax, future knockback).
//! - `visual_offset` = `collision_offset` + visual-only contributors (future hit-flash, cosmetic animation).
//!
//! Rendering reads `visual_offset`. Collision state reads `collision_offset`.
//!
//! To add a new contributor: add a new offset component, include it as
//! `Option<&NewOffset>` in the compose query, and add its value to the
//! correct accumulator based on its category.

use std::time::Duration;

use bevy::prelude::*;
use carapace::prelude::{CxPresentationTransform, WorldPos};

use super::{
    components::placement::Depth,
    data::{StageData, StageStep},
    projection::{projection_weight, tween_duration, walk_steps_at_elapsed},
    resources::{ActiveProjection, ProjectionView, StageTimeDomain},
};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// The currently effective parallax attenuation multiplier.
///
/// `0.0` = no parallax, `1.0` = full parallax.  Updated each frame by
/// [`update_active_parallax_attenuation`] from stage data + elapsed time.
#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct ActiveParallaxAttenuation(pub f32);

impl Default for ActiveParallaxAttenuation {
    fn default() -> Self {
        Self(1.0)
    }
}

// ---------------------------------------------------------------------------
// Step evaluation (mirrors projection.rs pattern)
// ---------------------------------------------------------------------------

/// Resolve the effective parallax attenuation at a given step index.
///
/// Walks backwards from `step_index` to find the nearest step with an
/// override.  Falls back to `stage_data.parallax_attenuation`, then to `1.0`.
#[must_use]
pub fn effective_parallax_attenuation(stage_data: &StageData, step_index: usize) -> f32 {
    if stage_data.steps.is_empty() {
        return stage_data.parallax_attenuation.unwrap_or(1.0);
    }
    for i in (0..=step_index.min(stage_data.steps.len() - 1)).rev() {
        let att = match &stage_data.steps[i] {
            StageStep::Tween(s) => s.parallax_attenuation,
            StageStep::Stop(s) => s.parallax_attenuation,
            StageStep::Cinematic(_) => None,
        };
        if let Some(a) = att {
            return a;
        }
    }
    stage_data.parallax_attenuation.unwrap_or(1.0)
}

/// Minimum tween duration (in seconds) below which interpolation is skipped.
const MIN_INTERPOLATION_DURATION_SECS: f32 = 0.01;

/// Evaluate the interpolated parallax attenuation at a given elapsed time.
///
/// During tween steps, linearly interpolates between the previous and current
/// effective values.  During stop/cinematic steps, holds the current value.
#[must_use]
pub fn evaluate_parallax_attenuation_at(stage_data: &StageData, elapsed: Duration) -> f32 {
    let info = walk_steps_at_elapsed(stage_data, elapsed);
    let curr = effective_parallax_attenuation(stage_data, info.step_index);

    if info.tween_progress < 1.0
        && let Some(StageStep::Tween(tween)) = stage_data.steps.get(info.step_index)
    {
        let dur = tween_duration(info.step_start_position, tween);
        if dur.as_secs_f32() >= MIN_INTERPOLATION_DURATION_SECS {
            let prev = if info.step_index > 0 {
                effective_parallax_attenuation(stage_data, info.step_index - 1)
            } else {
                stage_data.parallax_attenuation.unwrap_or(1.0)
            };
            return prev + (curr - prev) * info.tween_progress;
        }
    }

    curr
}

/// Keeps [`ActiveParallaxAttenuation`] in sync with the current step.
pub fn update_active_parallax_attenuation(
    stage_data: Option<Res<StageData>>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut active: ResMut<ActiveParallaxAttenuation>,
) {
    let Some(stage_data) = stage_data else {
        return;
    };
    active.0 = evaluate_parallax_attenuation_at(&stage_data, stage_time.elapsed());
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Per-entity parallax offset contribution (**collision-affecting**).
///
/// Parallax represents actual spatial displacement the player perceives.
/// It contributes to both `CxPresentationTransform.visual_offset` and
/// `CxPresentationTransform.collision_offset` so hitboxes align with
/// visible sprite positions during lateral camera motion.
///
/// Recomputed every frame by [`update_parallax_offset`].  The value is
/// consumed by [`compose_presentation_offsets`] and must not be read for
/// gameplay logic.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct ParallaxOffset(pub Vec2);

/// Opt-out marker: entities with this component skip parallax displacement
/// entirely. They are excluded from [`update_parallax_offset`] via a
/// `Without<NoParallax>` query filter, so their [`ParallaxOffset`] (if any)
/// stays at zero and their visual position is unaffected by lateral camera
/// movement.
///
/// Use for entities whose world-space X already accounts for projection (e.g.
/// the `depth_traverse` demo) or that should remain fixed regardless of camera
/// pan.
#[derive(Component, Debug, Clone, Copy)]
pub struct NoParallax;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Computes the parallax offset for each entity based on its depth lane
/// and the current lateral view offset.
///
/// # Weight source
///
/// For entities with a [`Depth`] component (stage entities on a depth lane),
/// the weight comes from the projection profile's floor Y for that lane.
/// This is stable — jumping, falling, hovering, or any other vertical motion
/// does not change the lateral parallax.
///
/// Entities without `Depth` fall back to `WorldPos.y`. This path
/// exists for non-stage entities that don't participate in the depth-lane
/// model (e.g. UI sprites, particles). It is not the normal game path.
pub fn update_parallax_offset(
    projection: Option<Res<ActiveProjection>>,
    view: Option<Res<ProjectionView>>,
    attenuation: Option<Res<ActiveParallaxAttenuation>>,
    mut query: Query<(&WorldPos, &mut ParallaxOffset, Option<&Depth>), Without<NoParallax>>,
) {
    let (Some(projection), Some(view)) = (projection, view) else {
        return;
    };
    let profile = &projection.0;
    let lateral = view.lateral_view_offset;
    let att = attenuation.map_or(1.0, |a| a.0);
    if lateral.abs() < f32::EPSILON || att.abs() < f32::EPSILON {
        for (_, mut parallax, _) in &mut query {
            if parallax.0 != Vec2::ZERO {
                parallax.0 = Vec2::ZERO;
            }
        }
        return;
    }

    for (sub_pos, mut parallax, depth) in &mut query {
        // Depth lane → stable floor Y; no Depth → transient entity Y.
        let reference_y = depth.map_or(sub_pos.0.y, |d| profile.floor_y_for_depth(d.to_i8()));
        let weight = projection_weight(profile, reference_y).clamp(0.0, 1.0);
        let new_offset = Vec2::new(-lateral * weight * att, 0.0);
        if parallax.0 != new_offset {
            parallax.0 = new_offset;
        }
    }
}

/// Composes all offset contributors into `CxPresentationTransform` offset fields.
///
/// Produces both composed offsets in one pass:
/// - `collision_offset` = sum of collision-affecting contributors (currently: parallax)
/// - `visual_offset` = `collision_offset` + sum of visual-only contributors (currently: none)
///
/// When a new contributor lands:
/// - Collision-affecting: add to `collision_offset` accumulator.
/// - Visual-only: add to `visual_only` accumulator.
///
/// `visual_offset` is always the superset: `collision_offset + visual_only`.
///
/// Writer discipline: only writes when the composed value actually changed,
/// so `Changed<CxPresentationTransform>` fires only on real changes.
pub fn compose_presentation_offsets(
    mut query: Query<(&ParallaxOffset, &mut CxPresentationTransform)>,
) {
    for (parallax, mut pt) in &mut query {
        // Collision-affecting contributors.
        let new_collision_offset = parallax.0;

        // Visual-only contributors (none yet).
        let visual_only = Vec2::ZERO;

        let new_visual_offset = new_collision_offset + visual_only;

        if pt.collision_offset != new_collision_offset {
            pt.collision_offset = new_collision_offset;
        }
        if pt.visual_offset != new_visual_offset {
            pt.visual_offset = new_visual_offset;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::projection::ProjectionProfile;

    fn test_profile() -> ProjectionProfile {
        ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.insert_resource(ActiveProjection(test_profile()));
        app.insert_resource(ProjectionView {
            lateral_view_offset: 50.0,
            lateral_anchor_x: 0.0,
        });
        app.add_systems(
            Update,
            (update_parallax_offset, compose_presentation_offsets).chain(),
        );
        app
    }

    // --- update_parallax_offset ---

    #[test]
    fn parallax_zero_at_horizon() {
        let mut app = make_app();
        let profile = test_profile();
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, profile.horizon_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let offset = app.world().entity(entity).get::<ParallaxOffset>().unwrap();
        assert!(
            offset.0.x.abs() < 1e-3,
            "at horizon, parallax should be ~0, got {}",
            offset.0.x
        );
    }

    #[test]
    fn parallax_full_at_foreground() {
        let mut app = make_app();
        let profile = test_profile();
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, profile.floor_base_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let offset = app.world().entity(entity).get::<ParallaxOffset>().unwrap();
        // weight=1.0, lateral=50.0 → offset.x = -50.0
        assert!(
            (offset.0.x - (-50.0)).abs() < 1e-3,
            "at foreground, parallax should be -50.0, got {}",
            offset.0.x
        );
    }

    #[test]
    fn parallax_intermediate_at_mid_y() {
        let mut app = make_app();
        let profile = test_profile();
        let mid_y = (profile.horizon_y + profile.floor_base_y) * 0.5; // 50.0
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, mid_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let offset = app.world().entity(entity).get::<ParallaxOffset>().unwrap();
        // weight = (50 - 100) / (0 - 100) = 0.5, lateral=50 → offset.x = -25.0
        assert!(
            (offset.0.x - (-25.0)).abs() < 1e-3,
            "at mid Y, parallax should be -25.0, got {}",
            offset.0.x
        );
    }

    // --- compose_presentation_offsets ---

    #[test]
    fn compose_overwrites_not_accumulates() {
        let mut app = make_app();
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, 0.0)), // floor_base_y → weight=1.0
                ParallaxOffset(Vec2::new(5.0, 0.0)),
                CxPresentationTransform {
                    visual_offset: Vec2::new(99.0, 99.0), // stale value to be overwritten
                    collision_offset: Vec2::new(99.0, 99.0),
                    ..Default::default()
                },
            ))
            .id();

        app.update();

        let pt = app
            .world()
            .entity(entity)
            .get::<CxPresentationTransform>()
            .unwrap();
        // Compose writes parallax.0, NOT stale + parallax.
        // After update_parallax_offset runs, parallax.0 = Vec2::new(-50.0, 0.0).
        // Compose then writes both offset fields = parallax.0.
        assert!(
            (pt.visual_offset.x - (-50.0)).abs() < 1e-3,
            "compose should overwrite, not accumulate; got {}",
            pt.visual_offset.x
        );
        assert!(
            pt.visual_offset.y.abs() < 1e-3,
            "visual_offset.y should be 0.0, got {}",
            pt.visual_offset.y
        );
        assert_eq!(
            pt.collision_offset, pt.visual_offset,
            "parallax is collision-affecting: both offsets should be equal",
        );
    }

    // --- Invariant: WorldPos unchanged ---

    #[test]
    fn parallax_does_not_modify_gameplay_position() {
        let mut app = make_app();
        let initial_pos = Vec2::new(42.0, 50.0);
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(initial_pos),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let pos = app.world().entity(entity).get::<WorldPos>().unwrap();
        assert_eq!(
            pos.0, initial_pos,
            "WorldPos must not be modified by parallax systems"
        );
    }

    // --- NoParallax opt-out ---

    #[test]
    fn no_parallax_marker_skips_entity() {
        let mut app = make_app();
        let normal = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, 0.0)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();
        let excluded = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, 0.0)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
                NoParallax,
            ))
            .id();

        app.update();

        let normal_offset = app.world().entity(normal).get::<ParallaxOffset>().unwrap();
        assert!(
            normal_offset.0.x.abs() > 1.0,
            "normal entity should have parallax offset"
        );

        let excluded_offset = app
            .world()
            .entity(excluded)
            .get::<ParallaxOffset>()
            .unwrap();
        assert_eq!(
            excluded_offset.0,
            Vec2::ZERO,
            "NoParallax entity should keep zero offset"
        );
    }

    // --- Zero lateral offset ---

    #[test]
    fn zero_lateral_offset_zeros_all_parallax() {
        let mut app = App::new();
        app.insert_resource(ActiveProjection(test_profile()));
        app.insert_resource(ProjectionView {
            lateral_view_offset: 0.0,
            lateral_anchor_x: 0.0,
        });
        app.add_systems(
            Update,
            (update_parallax_offset, compose_presentation_offsets).chain(),
        );

        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, 0.0)),
                ParallaxOffset(Vec2::new(99.0, 0.0)), // stale non-zero
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let offset = app.world().entity(entity).get::<ParallaxOffset>().unwrap();
        assert_eq!(offset.0, Vec2::ZERO, "zero lateral should zero parallax");
    }

    // --- Depth-lane stability ---

    /// Two entities at the same Depth but different Y (one on floor, one
    /// airborne) must produce identical parallax offsets. Vertical motion
    /// within a lane must not affect lateral parallax.
    #[test]
    fn same_depth_different_y_produces_same_parallax() {
        let mut app = make_app();
        let profile = test_profile();
        let floor_y = profile.floor_y_for_depth(Depth::Five.to_i8());

        let grounded = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, floor_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
                Depth::Five,
            ))
            .id();

        // Airborne: 40px above the floor (simulating a jump or fall).
        let airborne = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, floor_y + 40.0)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
                Depth::Five,
            ))
            .id();

        app.update();

        let grounded_offset = app
            .world()
            .entity(grounded)
            .get::<ParallaxOffset>()
            .unwrap()
            .0;
        let airborne_offset = app
            .world()
            .entity(airborne)
            .get::<ParallaxOffset>()
            .unwrap()
            .0;
        assert_eq!(
            grounded_offset, airborne_offset,
            "same depth lane must produce same parallax regardless of Y \
             (grounded={grounded_offset:?}, airborne={airborne_offset:?})",
        );
    }

    /// Entity without Depth falls back to WorldPos.y for the weight.
    #[test]
    fn no_depth_falls_back_to_y() {
        let mut app = make_app();
        let profile = test_profile();

        // At foreground Y (floor_base_y = 0.0), weight = 1.0
        let at_floor = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, profile.floor_base_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
                // No Depth component
            ))
            .id();

        // At horizon Y, weight ≈ 0.0
        let at_horizon = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, profile.horizon_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let floor_offset = app
            .world()
            .entity(at_floor)
            .get::<ParallaxOffset>()
            .unwrap()
            .0;
        let horizon_offset = app
            .world()
            .entity(at_horizon)
            .get::<ParallaxOffset>()
            .unwrap()
            .0;

        // Without Depth, different Y → different weight → different offset.
        assert!(
            (floor_offset.x - (-50.0)).abs() < 1e-3,
            "at foreground Y without Depth, should use Y-based weight=1.0; got {}",
            floor_offset.x
        );
        assert!(
            horizon_offset.x.abs() < 1e-3,
            "at horizon Y without Depth, should use Y-based weight≈0; got {}",
            horizon_offset.x
        );
    }

    // --- Attenuation ---

    #[test]
    fn attenuation_halves_parallax() {
        let mut app = make_app();
        app.insert_resource(ActiveParallaxAttenuation(0.5));
        let profile = test_profile();
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, profile.floor_base_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let offset = app.world().entity(entity).get::<ParallaxOffset>().unwrap();
        // weight=1.0, lateral=50.0, att=0.5 → offset.x = -25.0
        assert!(
            (offset.0.x - (-25.0)).abs() < 1e-3,
            "half attenuation should halve parallax, got {}",
            offset.0.x
        );
    }

    #[test]
    fn attenuation_zero_disables_parallax() {
        let mut app = make_app();
        app.insert_resource(ActiveParallaxAttenuation(0.0));
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, 0.0)),
                ParallaxOffset(Vec2::new(99.0, 0.0)), // stale
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let offset = app.world().entity(entity).get::<ParallaxOffset>().unwrap();
        assert_eq!(
            offset.0,
            Vec2::ZERO,
            "zero attenuation should zero parallax"
        );
    }

    #[test]
    fn no_attenuation_resource_defaults_to_full() {
        // make_app() does NOT insert ActiveParallaxAttenuation.
        let mut app = make_app();
        let profile = test_profile();
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, profile.floor_base_y)),
                ParallaxOffset::default(),
                CxPresentationTransform::default(),
            ))
            .id();

        app.update();

        let offset = app.world().entity(entity).get::<ParallaxOffset>().unwrap();
        // No resource → att=1.0, weight=1.0, lateral=50.0 → -50.0
        assert!(
            (offset.0.x - (-50.0)).abs() < 1e-3,
            "without resource, should use full parallax, got {}",
            offset.0.x
        );
    }

    // --- Evaluation functions ---

    use crate::stage::components::{StopStageStep, TweenStageStep};
    use crate::stage::data::{SkyboxData, StageData, StageStep};

    fn make_stage(steps: Vec<StageStep>) -> StageData {
        StageData {
            name: "test".into(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: vec![],
            steps,
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
            checkpoint: None,
            parallax_attenuation: None,
        }
    }

    fn att_tween(x: f32, speed: f32, att: Option<f32>) -> StageStep {
        StageStep::Tween(TweenStageStep {
            coordinates: Vec2::new(x, 0.0),
            base_speed: speed,
            spawns: vec![],
            surfaces: None,
            projection: None,
            parallax_attenuation: att,
        })
    }

    #[test]
    fn effective_attenuation_defaults_to_one() {
        let stage = make_stage(vec![att_tween(100.0, 1.0, None)]);
        assert!((effective_parallax_attenuation(&stage, 0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn effective_attenuation_reads_step_value() {
        let stage = make_stage(vec![att_tween(100.0, 1.0, Some(0.3))]);
        assert!((effective_parallax_attenuation(&stage, 0) - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn effective_attenuation_sticky_carry_forward() {
        let stage = make_stage(vec![
            att_tween(100.0, 1.0, Some(0.5)),
            att_tween(200.0, 1.0, None), // inherits
        ]);
        assert!((effective_parallax_attenuation(&stage, 1) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn effective_attenuation_later_override_wins() {
        let stage = make_stage(vec![
            att_tween(100.0, 1.0, Some(0.5)),
            att_tween(200.0, 1.0, Some(0.8)),
        ]);
        assert!((effective_parallax_attenuation(&stage, 0) - 0.5).abs() < f32::EPSILON);
        assert!((effective_parallax_attenuation(&stage, 1) - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn effective_attenuation_uses_stage_default() {
        let mut stage = make_stage(vec![att_tween(100.0, 1.0, None)]);
        stage.parallax_attenuation = Some(0.7);
        assert!((effective_parallax_attenuation(&stage, 0) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn evaluate_attenuation_interpolates_mid_tween() {
        // Step 0: att=0.2, tween 0→100 at speed 1 = 100s.
        // Step 1: att=0.8, tween 100→200 at speed 1 = 100s.
        let stage = make_stage(vec![
            att_tween(100.0, 1.0, Some(0.2)),
            att_tween(200.0, 1.0, Some(0.8)),
        ]);
        // At 150s: mid step 1, t=0.5. prev=0.2, curr=0.8 → 0.5
        let att = evaluate_parallax_attenuation_at(&stage, Duration::from_secs(150));
        assert!(
            (att - 0.5).abs() < 0.01,
            "mid-tween should interpolate, got {att}"
        );
    }

    #[test]
    fn evaluate_attenuation_holds_during_stop() {
        let stage = make_stage(vec![
            att_tween(100.0, 1.0, Some(0.4)),
            StageStep::Stop(
                StopStageStep::new()
                    .with_max_duration(10.0)
                    .with_parallax_attenuation(0.6),
            ),
        ]);
        // 105s: 100s tween + 5s into stop.
        let att = evaluate_parallax_attenuation_at(&stage, Duration::from_secs(105));
        assert!(
            (att - 0.6).abs() < f32::EPSILON,
            "stop step should hold value, got {att}"
        );
    }
}
