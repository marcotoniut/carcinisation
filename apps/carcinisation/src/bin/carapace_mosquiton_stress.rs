#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value,
    clippy::type_complexity
)]
//! Real-world `carapace` stress scene using authored Mosquiton composed visuals.
//!
//! Spawns 2000 Mosquitons, drives them through deterministic depth traversal,
//! and keeps the population heavily biased toward the background. Foreground
//! reach falls off geometrically, so only one Mosquiton is ever allowed to hit
//! depth 0 while the bulk of the swarm stays in deeper layers. Most of the
//! swarm is airborne at any given moment, with per-entity liftoff / landing
//! cycles continuously mixing flying and walking states.
//!
//! Run with:
//! - `cargo run -p carcinisation --bin carapace_mosquiton_stress`
//! - `cargo run -p carcinisation --bin carapace_mosquiton_stress --features brp`
//!
//! - `Cmd+I` / `Ctrl+I` to cycle horizon profiles.
//! - `Cmd+P` / `Ctrl+P` to toggle the depth perspective grid overlay.
//! - `Cmd+O` / `Ctrl+O` to toggle entity anchor markers.
//! - `Shift+Left/Right` to pan the lateral view offset.
//!
//! This demo uses a synthetic traversal authority model and does not represent
//! the gameplay spawn pipeline. Do not use it as a reference for the stage
//! spawn-time presentation invariant.

use bevy::{
    asset::{AssetMetaCheck, AssetPlugin},
    diagnostic::LogDiagnosticsPlugin,
    prelude::*,
    window::WindowPlugin,
};
#[cfg(feature = "brp")]
use bevy_brp_extras::BrpExtrasPlugin;
use carapace::{prelude::*, set::CxSet};
use carcinisation::{
    globals::SCREEN_RESOLUTION,
    stage::{
        components::placement::{Airborne, AnchorOffsets, AuthoredDepths, Depth},
        depth_debug::DepthDebugPlugin,
        depth_scale::{DepthScaleConfig, apply_depth_fallback_scale},
        enemy::{
            composed::{
                ComposedAnimationState, ComposedEnemyVisual, ComposedResolvedParts,
                CompositionAtlasAsset, CompositionAtlasLoader, apply_composed_enemy_visuals,
                ensure_composed_enemy_parts, prepare_composed_atlas_assets,
                update_composed_enemy_visuals,
            },
            data::mosquiton::{TAG_IDLE_STAND, apply_mosquiton_animation_state},
            entity::EnemyType,
        },
        messages::ComposedAnimationCueMessage,
        projection::{ProjectionProfile, compute_visual_x, pan_lateral_view},
        resources::{ActiveProjection, DebugPanConfig, ProjectionView, StageTimeDomain},
    },
};

const MOSQUITON_COUNT: usize = 2000;
const SCREEN_SCALE: u32 = 4;
const AUTHORED_DEPTH: Depth = Depth::Three;
const SCREEN_W: f32 = SCREEN_RESOLUTION.x as f32;
const SCREEN_H: f32 = SCREEN_RESOLUTION.y as f32;
const HORIZON_FRACTIONS: [f32; 3] = [0.5, 0.15, 0.85];
/// Floor Y for depth 1 (foreground). This is `floor_base_y` in the
/// `ProjectionProfile` — `floor_y_for_depth(1)` returns this value directly
/// because `t = 1.0` at depth 1.
const FLOOR_DEPTH_1: f32 = -0.10 * SCREEN_H;
const PERSPECTIVE_BIAS: f32 = 3.0;
const AIRBORNE_DUTY_CYCLE: f32 = 0.65;
const LIFTOFF_SECS: f32 = 0.5;
const LANDING_SECS: f32 = 0.5;

/// Stress-only continuous traversal mapping.
///
/// The stress scene bins the swarm across depths 0..=9 so a tiny foreground
/// slice can reach depth 0, while the shared [`ProjectionProfile`] still owns
/// the actual floor curve for depths 1..=9 and its depth-0 extrapolation.
struct StressDepthTraversal;

impl StressDepthTraversal {
    const DEPTH_MIN: i8 = 0;
    const DEPTH_MAX: i8 = 9;
    const DEPTH_COUNT: f32 = (Self::DEPTH_MAX - Self::DEPTH_MIN + 1) as f32;
    const DEPTH_INTERVAL_COUNT: f32 = (Self::DEPTH_MAX - Self::DEPTH_MIN) as f32;

    fn wave_t(elapsed: f32, motion: &SwarmMotion) -> f32 {
        let max_t = Self::progress_for_depth(motion.max_foreground_depth);
        let oscillation = ((elapsed * motion.traverse_speed + motion.traverse_phase)
            * std::f32::consts::TAU)
            .sin()
            .abs();
        max_t * oscillation
    }

    fn depth_for_progress(progress: f32) -> Depth {
        let bin = (progress * Self::DEPTH_COUNT)
            .floor()
            .min(Self::DEPTH_COUNT - 1.0) as i8;
        let depth_i8 = (Self::DEPTH_MAX - bin).max(Self::DEPTH_MIN);
        Depth::try_from(depth_i8).unwrap_or(Depth::Nine)
    }

    fn progress_for_depth(depth: Depth) -> f32 {
        f32::from(Self::DEPTH_MAX - depth.to_i8()) / Self::DEPTH_INTERVAL_COUNT
    }
}

#[derive(Component, Clone, Copy, Debug)]
struct SwarmMotion {
    center_x: f32,
    x_amplitude: f32,
    x_angular_velocity: f32,
    x_phase: f32,
    traverse_speed: f32,
    traverse_phase: f32,
    locomotion_speed: f32,
    locomotion_phase: f32,
    flight_altitude: f32,
    max_foreground_depth: Depth,
}

#[derive(Component, Clone, Copy, Debug)]
struct SwarmLocomotion {
    state: LocomotionState,
}

#[derive(Clone, Copy, Debug)]
enum LocomotionState {
    Grounded,
    Liftoff { remaining: f32 },
    Airborne,
    Landing { remaining: f32 },
}

#[derive(Resource, Clone, Copy, Debug)]
struct StressHorizonProfile {
    index: usize,
    profile: ProjectionProfile,
}

impl Default for StressHorizonProfile {
    fn default() -> Self {
        Self {
            index: 0,
            profile: projection_profile(HORIZON_FRACTIONS[0] * SCREEN_H),
        }
    }
}

#[px_layer]
enum Layer {
    Nine,
    Eight,
    Seven,
    Six,
    Five,
    Four,
    Three,
    Two,
    #[default]
    One,
}

fn main() {
    let title = format!("Carapace Mosquiton Stress ({MOSQUITON_COUNT} / 9 layers)");

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    resolution: (
                        SCREEN_RESOLUTION.x * SCREEN_SCALE,
                        SCREEN_RESOLUTION.y * SCREEN_SCALE,
                    )
                        .into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../../assets".into(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
        CxPlugin::<Layer>::new(SCREEN_RESOLUTION, "palette/base.png"),
        LogDiagnosticsPlugin::default(),
        DepthDebugPlugin,
    ));

    #[cfg(feature = "brp")]
    if !app.is_plugin_added::<BrpExtrasPlugin>() {
        app.add_plugins(BrpExtrasPlugin);
    }

    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(DepthScaleConfig::load_or_default())
        .init_resource::<StressHorizonProfile>()
        .insert_resource(ActiveProjection(StressHorizonProfile::default().profile))
        .init_resource::<ProjectionView>()
        .init_resource::<DebugPanConfig>()
        .init_resource::<Time<StageTimeDomain>>()
        .add_message::<ComposedAnimationCueMessage>()
        .init_asset::<CompositionAtlasAsset>()
        .register_asset_loader(CompositionAtlasLoader)
        .add_systems(Startup, (setup, setup_stats_text))
        .add_systems(
            Update,
            (
                tick_stage_time,
                cycle_horizon_profile,
                pan_lateral_view,
                update_locomotion,
                animate_swarm,
                traverse_depths,
                prepare_composed_atlas_assets,
                ensure_composed_enemy_parts,
                update_composed_enemy_visuals,
                update_stats_text,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            (
                apply_depth_fallback_scale,
                apply_composed_enemy_visuals.in_set(CxSet::CompositePresentationWrites),
            ),
        )
        .run();
}

fn tick_stage_time(time: Res<Time>, mut stage_time: ResMut<Time<StageTimeDomain>>) {
    stage_time.advance_by(time.delta());
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    profile: Res<StressHorizonProfile>,
    depth_scale: Res<DepthScaleConfig>,
) {
    commands.spawn(Camera2d);

    // Overlay camera: renders gizmos on top of CxPlugin output.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        CxOverlayCamera,
    ));

    info!(
        "Spawning {MOSQUITON_COUNT} Mosquitons across depths 0-9 \
         (Cmd+I = horizon, Cmd+P = grid, Cmd+O = anchors)"
    );

    // Demo/stress path only: these entities are spawned under a synthetic
    // traversal authority model and do not serve as a gameplay reference for
    // the stage spawn-time presentation invariant.
    for index in 0..MOSQUITON_COUNT {
        let motion = motion_for(index);
        let locomotion = initial_locomotion(0.0, &motion);
        let initial_progress = StressDepthTraversal::wave_t(0.0, &motion);
        let depth = StressDepthTraversal::depth_for_progress(initial_progress);
        let initial_x = motion.center_x + (motion.x_phase).sin() * motion.x_amplitude;
        let initial_y = entity_y_for(
            initial_progress,
            depth,
            &motion,
            locomotion.state,
            None,
            profile.profile,
            &depth_scale,
        );
        let initial_tag = animation_tag_for(locomotion.state, initial_x, motion.center_x);
        let mut animation = ComposedAnimationState::new(initial_tag);
        set_mosquiton_tag(&mut animation, initial_tag);

        let mut entity = commands.spawn((
            Name::new(format!("Stress<Mosquiton#{index}>")),
            animation,
            ComposedEnemyVisual::for_enemy(&asset_server, EnemyType::Mosquiton, AUTHORED_DEPTH),
            WorldPos::from(Vec2::new(initial_x, initial_y)),
            AuthoredDepths::single(AUTHORED_DEPTH),
            depth,
            layer_for_depth(depth),
            CxAnchor::Center,
            motion,
            locomotion,
        ));

        if locomotion_is_airborne(locomotion.state) {
            entity.insert(Airborne);
        }
    }
}

/// Cycle horizon profile with `Cmd+I` / `Ctrl+I`.
fn cycle_horizon_profile(
    keys: Res<ButtonInput<KeyCode>>,
    mut profile: ResMut<StressHorizonProfile>,
    mut active_projection: ResMut<ActiveProjection>,
) {
    let modifier_held = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if !modifier_held || !keys.just_pressed(KeyCode::KeyI) {
        return;
    }

    profile.index = (profile.index + 1) % HORIZON_FRACTIONS.len();
    profile.profile.horizon_y = HORIZON_FRACTIONS[profile.index] * SCREEN_H;

    info!(
        "Stress horizon profile {} -- depth 9 floor at {:.0}% of screen height",
        profile.index,
        HORIZON_FRACTIONS[profile.index] * 100.0,
    );
    active_projection.0 = profile.profile;
}

fn animate_swarm(time: Res<Time>, mut query: Query<(&SwarmMotion, &mut WorldPos)>) {
    let t = time.elapsed_secs();

    for (motion, mut position) in &mut query {
        let x = motion.center_x
            + (t * motion.x_angular_velocity + motion.x_phase).sin() * motion.x_amplitude;
        position.0.x = x;
    }
}

fn update_locomotion(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &SwarmMotion,
        &mut SwarmLocomotion,
        &mut ComposedAnimationState,
        &WorldPos,
    )>,
) {
    let elapsed = time.elapsed_secs();
    let dt = time.delta_secs();

    for (entity, motion, mut locomotion, mut animation, position) in &mut query {
        let desired_airborne = desires_airborne(elapsed, motion);
        let previous_airborne = locomotion_is_airborne(locomotion.state);

        locomotion.state = match locomotion.state {
            LocomotionState::Grounded => {
                if desired_airborne {
                    set_mosquiton_tag(&mut animation, "liftoff");
                    LocomotionState::Liftoff {
                        remaining: LIFTOFF_SECS,
                    }
                } else {
                    let tag =
                        animation_tag_for(LocomotionState::Grounded, position.0.x, motion.center_x);
                    set_mosquiton_tag(&mut animation, tag);
                    LocomotionState::Grounded
                }
            }
            LocomotionState::Liftoff { remaining } => {
                let remaining = remaining - dt;
                if remaining <= 0.0 {
                    let tag =
                        animation_tag_for(LocomotionState::Airborne, position.0.x, motion.center_x);
                    set_mosquiton_tag(&mut animation, tag);
                    LocomotionState::Airborne
                } else {
                    LocomotionState::Liftoff { remaining }
                }
            }
            LocomotionState::Airborne => {
                if desired_airborne {
                    let tag =
                        animation_tag_for(LocomotionState::Airborne, position.0.x, motion.center_x);
                    set_mosquiton_tag(&mut animation, tag);
                    LocomotionState::Airborne
                } else {
                    set_mosquiton_tag(&mut animation, TAG_IDLE_STAND);
                    LocomotionState::Landing {
                        remaining: LANDING_SECS,
                    }
                }
            }
            LocomotionState::Landing { remaining } => {
                let remaining = remaining - dt;
                if remaining <= 0.0 {
                    let tag =
                        animation_tag_for(LocomotionState::Grounded, position.0.x, motion.center_x);
                    set_mosquiton_tag(&mut animation, tag);
                    LocomotionState::Grounded
                } else {
                    LocomotionState::Landing { remaining }
                }
            }
        };

        let airborne_now = locomotion_is_airborne(locomotion.state);
        if airborne_now && !previous_airborne {
            commands.entity(entity).insert(Airborne);
        } else if !airborne_now && previous_airborne {
            commands.entity(entity).remove::<Airborne>();
        }
    }
}

fn traverse_depths(
    time: Res<Time>,
    profile: Res<StressHorizonProfile>,
    projection_view: Res<ProjectionView>,
    depth_scale: Res<DepthScaleConfig>,
    mut query: Query<(
        &SwarmMotion,
        &SwarmLocomotion,
        &mut Depth,
        &mut Layer,
        &mut WorldPos,
        Option<&AnchorOffsets>,
    )>,
) {
    let elapsed = time.elapsed_secs();

    for (motion, locomotion, mut depth, mut layer, mut position, anchors) in &mut query {
        let progress = StressDepthTraversal::wave_t(elapsed, motion);
        let new_depth = StressDepthTraversal::depth_for_progress(progress);

        *depth = new_depth;
        *layer = layer_for_depth(new_depth);
        position.0.y = entity_y_for(
            progress,
            new_depth,
            motion,
            locomotion.state,
            anchors,
            profile.profile,
            &depth_scale,
        );
        // Stress test has no parallax system — projection-bake X from
        // the depth lane's floor Y (not the entity's transient visual Y).
        let floor_y = profile.profile.floor_y_for_progress(progress);
        position.0.x = compute_visual_x(position.0.x, floor_y, &profile.profile, &projection_view);
    }
}

/// Marker for the stats text UI node.
#[derive(Component)]
struct StatsText;

/// Rolling FPS tracker with a 60-frame window, updated every 100 ms.
#[derive(Component)]
struct FpsTracker {
    frames: Vec<f32>,
    current_fps: f32,
    current_ms: f32,
    last_update: f32,
}

impl Default for FpsTracker {
    fn default() -> Self {
        Self {
            frames: Vec::with_capacity(60),
            current_fps: 0.0,
            current_ms: 0.0,
            last_update: 0.0,
        }
    }
}

fn setup_stats_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        StatsText,
        FpsTracker::default(),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(5.0),
            top: Val::Px(5.0),
            ..default()
        },
        children![(
            TextColor(Color::srgb(0.2, 1.0, 0.2)),
            Text::new("FPS: --\nMS: --\nPTS: --"),
            TextLayout::new_with_justify(Justify::Right),
            TextFont {
                font: asset_server.load("fonts/Pixeboy.ttf"),
                font_size: 14.0,
                ..default()
            },
        )],
    ));
}

fn update_stats_text(
    time: Res<Time>,
    parts_query: Query<&ComposedResolvedParts>,
    mut parent_query: Query<(&Children, &mut FpsTracker), With<StatsText>>,
    mut text_query: Query<&mut Text>,
) {
    let elapsed = time.elapsed_secs();
    let dt = time.delta_secs();

    for (children, mut tracker) in &mut parent_query {
        if dt > 0.0 {
            tracker.frames.push(dt);
            if tracker.frames.len() > 60 {
                tracker.frames.remove(0);
            }
        }

        if elapsed - tracker.last_update > 0.1 && !tracker.frames.is_empty() {
            let avg_dt = tracker.frames.iter().sum::<f32>() / tracker.frames.len() as f32;
            tracker.current_fps = 1.0 / avg_dt;
            tracker.current_ms = avg_dt * 1000.0;
            tracker.last_update = elapsed;
        }

        let total_fragments: usize = parts_query.iter().map(|p| p.fragments().len()).sum();
        let new_text = format!(
            "FPS: {:.0}\nMS: {:.1}\nPTS: {}",
            tracker.current_fps, tracker.current_ms, total_fragments
        );

        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child)
                && text.0 != new_text
            {
                text.0.clone_from(&new_text);
            }
        }
    }
}

fn layer_for_depth(depth: Depth) -> Layer {
    match depth {
        Depth::Nine => Layer::Nine,
        Depth::Eight => Layer::Eight,
        Depth::Seven => Layer::Seven,
        Depth::Six => Layer::Six,
        Depth::Five => Layer::Five,
        Depth::Four => Layer::Four,
        Depth::Three => Layer::Three,
        Depth::Two => Layer::Two,
        Depth::One | Depth::Zero => Layer::One,
    }
}

fn depth_slot(depth: Depth) -> usize {
    match depth {
        Depth::Nine => 0,
        Depth::Eight => 1,
        Depth::Seven => 2,
        Depth::Six => 3,
        Depth::Five => 4,
        Depth::Four => 5,
        Depth::Three => 6,
        Depth::Two => 7,
        Depth::One => 8,
        Depth::Zero => 9,
    }
}

fn motion_for(index: usize) -> SwarmMotion {
    let max_foreground_depth = foreground_depth_for_rank(index);
    let lane = depth_slot(max_foreground_depth);
    let max_x = (SCREEN_W - 20.0).max(1.0) as usize;

    let base_x = 10.0 + ((index * 37 + lane * 11) % max_x) as f32;

    SwarmMotion {
        center_x: base_x,
        x_amplitude: 2.0 + ((index * 13 + lane) % 9) as f32,
        x_angular_velocity: 0.35 + lane as f32 * 0.02 + (index % 5) as f32 * 0.03,
        x_phase: ((index * 19 + lane * 7) % 360) as f32 * 0.05,
        traverse_speed: 0.08 + lane as f32 * 0.004 + (index % 11) as f32 * 0.002,
        traverse_phase: ((index * 23 + lane * 5) % 360) as f32 / 360.0,
        locomotion_speed: 1.0 / (8.0 + lane as f32 * 0.65 + (index % 7) as f32 * 0.55),
        locomotion_phase: ((index * 29 + lane * 17) % 512) as f32 / 512.0,
        flight_altitude: 8.0 + lane as f32 * 1.2 + (index % 6) as f32 * 1.5,
        max_foreground_depth,
    }
}

fn initial_locomotion(elapsed: f32, motion: &SwarmMotion) -> SwarmLocomotion {
    SwarmLocomotion {
        state: if desires_airborne(elapsed, motion) {
            LocomotionState::Airborne
        } else {
            LocomotionState::Grounded
        },
    }
}

fn desires_airborne(elapsed: f32, motion: &SwarmMotion) -> bool {
    ((elapsed * motion.locomotion_speed) + motion.locomotion_phase).fract() < AIRBORNE_DUTY_CYCLE
}

fn locomotion_is_airborne(state: LocomotionState) -> bool {
    !matches!(state, LocomotionState::Grounded)
}

fn lift_ratio(state: LocomotionState) -> f32 {
    match state {
        LocomotionState::Grounded => 0.0,
        LocomotionState::Liftoff { remaining } => 1.0 - (remaining / LIFTOFF_SECS).clamp(0.0, 1.0),
        LocomotionState::Airborne => 1.0,
        LocomotionState::Landing { remaining } => (remaining / LANDING_SECS).clamp(0.0, 1.0),
    }
}

fn projection_profile(horizon_y: f32) -> ProjectionProfile {
    ProjectionProfile {
        horizon_y,
        floor_base_y: FLOOR_DEPTH_1,
        bias_power: PERSPECTIVE_BIAS,
    }
}

fn entity_y_for(
    progress: f32,
    depth: Depth,
    motion: &SwarmMotion,
    state: LocomotionState,
    anchors: Option<&AnchorOffsets>,
    profile: ProjectionProfile,
    depth_scale: &DepthScaleConfig,
) -> f32 {
    let fallback_scale =
        depth_scale.resolve_fallback(depth, &AuthoredDepths::single(AUTHORED_DEPTH));
    let floor_y = profile.floor_y_for_progress(progress);
    let anchor_ground = anchors.map_or(0.0, |anchor| anchor.ground);
    let grounded_y = floor_y + anchor_ground * fallback_scale;
    grounded_y + motion.flight_altitude * fallback_scale * lift_ratio(state)
}

fn set_mosquiton_tag(animation: &mut ComposedAnimationState, tag: &str) {
    apply_mosquiton_animation_state(animation, tag);
}

fn animation_tag_for(state: LocomotionState, x: f32, center_x: f32) -> &'static str {
    match state {
        LocomotionState::Grounded => "walk_forward",
        LocomotionState::Liftoff { .. } => "liftoff",
        LocomotionState::Airborne => {
            if x >= center_x {
                "fly_forward"
            } else {
                "fly_backward"
            }
        }
        LocomotionState::Landing { .. } => TAG_IDLE_STAND,
    }
}

fn foreground_depth_for_rank(index: usize) -> Depth {
    match index {
        0 => Depth::Zero,
        1..=3 => Depth::One,
        4..=12 => Depth::Two,
        13..=39 => Depth::Three,
        40..=120 => Depth::Four,
        121..=363 => Depth::Five,
        364..=1092 => Depth::Six,
        1093..=1999 => Depth::Seven,
        _ => Depth::Nine,
    }
}
