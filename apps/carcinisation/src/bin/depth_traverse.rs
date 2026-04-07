//! Mosquiton depth-traverse test.
//!
//! Mosquiton oscillates between depth 9 (horizon) and depth 1 (foreground),
//! validating authored depth sprite switching, fallback scaling, floor-line
//! placement, and visual coherence across the full visible depth range.
//!
//! Depth floor lines are drawn by the shared [`DepthDebugPlugin`] overlay
//! (enabled by default here; toggle with `Ctrl+L` / `Cmd+L`).
//!
//! A [`PxOverlayCamera`] is used so Bevy gizmos render on top of `PxPlugin`'s
//! pixel-art output.
//!
//! `Ctrl+O` / `Cmd+O` cycles through three horizon profiles (depth 9 floor
//! at 50%, 15%, or 85% of screen height).
//!
//! Run with: `cargo run -p carcinisation --bin depth_traverse`

use bevy::{asset::AssetMetaCheck, prelude::*};
#[cfg(feature = "brp")]
use bevy_brp_extras::BrpExtrasPlugin;
use carapace::{animation::PxAnimationPlugin, prelude::*};
use carcinisation::{
    globals::SCREEN_RESOLUTION,
    stage::{
        components::placement::{AuthoredDepths, Depth, Floor},
        depth_debug::{DepthDebugOverlay, DepthDebugPlugin},
        depth_scale::{DepthScaleConfig, apply_depth_fallback_scale},
        enemy::{
            composed::{
                ComposedAnimationState, ComposedEnemyVisual, CompositionAtlasAsset,
                CompositionAtlasLoader, ensure_composed_enemy_parts, prepare_composed_atlas_assets,
                update_composed_enemy_visuals,
            },
            entity::EnemyType,
        },
        messages::ComposedAnimationCueMessage,
        resources::StageTimeDomain,
    },
};

// --- Constants ---

#[allow(clippy::cast_precision_loss)] // 160 fits in f32 exactly.
const SCREEN_W: f32 = SCREEN_RESOLUTION.x as f32;
#[allow(clippy::cast_precision_loss)] // 144 fits in f32 exactly.
const SCREEN_H: f32 = SCREEN_RESOLUTION.y as f32;

/// Depth 1 floor line is fixed across all profiles: -10% of screen height.
const FLOOR_DEPTH_1: f32 = -0.1 * SCREEN_H;

/// Depth 9 floor line fractions for each horizon profile.
const HORIZON_FRACTIONS: [f32; 3] = [0.5, 0.15, 0.85];

const CENTER_X: f32 = SCREEN_W / 2.0;
const PERIOD_SECS: f32 = 14.0;

const DEPTH_MIN: i8 = 1;
const DEPTH_MAX: i8 = 9;
const DEPTH_COUNT: f32 = (DEPTH_MAX - DEPTH_MIN + 1) as f32;

const VIEWPORT_SCALE: f32 = 4.0;

/// Duration of the idle pause at each oscillation endpoint before reversing.
const ENDPOINT_PAUSE_SECS: f32 = 2.0;

/// Short pause before/after liftoff and landing transitions.
const TRANSITION_PAUSE_SECS: f32 = 0.5;

/// How long the liftoff/landing animation pose is held.
const TRANSITION_ANIM_SECS: f32 = 0.5;

/// Number of passes (half-trips) before switching between walk and fly modes.
/// 3 = forward, backward, forward — then transition.
const PASSES_BEFORE_SWITCH: u32 = 3;

/// Vertical offset (in carapace pixels) the sprite rises during flight.
/// Scaled by depth scale at runtime for consistent visual gap.
const FLY_HEIGHT_OFFSET: f32 = 24.0;

// --- Resources ---

/// Active horizon profile. Stores the depth-9 floor Y and drives all floor
/// mapping through [`floor_y_from_t`] and [`floor_y_for_depth`].
#[derive(Resource)]
struct HorizonProfile {
    index: usize,
    floor_depth_9: f32,
}

impl Default for HorizonProfile {
    fn default() -> Self {
        Self {
            index: 0,
            floor_depth_9: HORIZON_FRACTIONS[0] * SCREEN_H,
        }
    }
}

// --- Components ---

#[derive(Component)]
struct DepthWalker;

/// Oscillation state machine for the depth walk.
#[derive(Component)]
struct WalkProgress {
    t: f32,
    direction: f32,
    phase: WalkPhase,
    /// Whether we're currently in flying mode.
    airborne: bool,
    /// Counts completed passes (half-trips). Each endpoint hit increments by 1.
    half_trips: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WalkPhase {
    /// Moving between endpoints (walking or flying depending on `airborne`).
    Moving,
    /// Paused at an endpoint; `remaining` counts down to zero before reversing.
    Idle { remaining: f32 },
    /// Pre-transition pause before liftoff/landing.
    PreTransition { remaining: f32 },
    /// Playing the liftoff animation (rising).
    Liftoff { remaining: f32 },
    /// Playing the landing animation (descending).
    /// TODO: use a dedicated landing animation when one is authored.
    Landing { remaining: f32 },
    /// Post-transition pause after liftoff/landing, before resuming movement.
    PostTransition { remaining: f32 },
}

/// Marker for Floor entities managed by this example (so we can update them).
#[derive(Component)]
struct DepthFloorLine;

// --- Layer (local — the game's Layer is crate-private) ---

#[px_layer]
enum Layer {
    #[default]
    Back,
    Front,
}

// --- Perspective mapping ---

/// Cubic ease-in perspective bias applied to the normalised depth parameter
/// before interpolating screen-space floor positions.
///
/// `t` is the linear normalised depth where `0.0` = depth 9 (horizon) and
/// `1.0` = depth 1 (foreground). The output `t³` strongly compresses distant
/// depths (small `t`) and aggressively spreads near depths (large `t`),
/// giving a pronounced perspective-like floor layout while keeping the walk
/// speed linear in time.
fn perspective_bias(t: f32) -> f32 {
    t * t * t
}

/// Map a normalised `t ∈ [0, 1]` (0 = horizon, 1 = foreground) to a
/// screen-space floor Y through the perspective bias, using the given
/// depth-9 floor position.
fn floor_y_from_t(t: f32, floor_depth_9: f32) -> f32 {
    let biased = perspective_bias(t);
    floor_depth_9 + biased * (FLOOR_DEPTH_1 - floor_depth_9)
}

/// Compute the floor Y position for a discrete depth value.
fn floor_y_for_depth(d: i8, floor_depth_9: f32) -> f32 {
    let t = f32::from(DEPTH_MAX - d) / f32::from(DEPTH_MAX - DEPTH_MIN);
    floor_y_from_t(t, floor_depth_9)
}

// --- Entry point ---

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Mosquiton Depth Traverse".into(),
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    resolution: (
                        SCREEN_W as u32 * VIEWPORT_SCALE as u32,
                        SCREEN_H as u32 * VIEWPORT_SCALE as u32,
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
        PxPlugin::<Layer>::new(SCREEN_RESOLUTION, "palette/base.png"),
        PxAnimationPlugin,
        DepthDebugPlugin,
    ))
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(DepthScaleConfig::load_or_default())
    .init_resource::<HorizonProfile>()
    // Start with the depth overlay enabled for this example.
    .insert_resource(DepthDebugOverlay::new(true));
    // BRP support for runtime inspection (carapace/brp_extras adds BrpExtrasPlugin
    // when the feature is active, so only add it if it hasn't been registered yet).
    #[cfg(feature = "brp")]
    if !app.is_plugin_added::<BrpExtrasPlugin>() {
        app.add_plugins(BrpExtrasPlugin);
    }
    // Resources needed by the composed animation pipeline.
    app.init_resource::<Time<StageTimeDomain>>()
        .add_message::<ComposedAnimationCueMessage>()
        // Composed animation asset pipeline.
        .init_asset::<CompositionAtlasAsset>()
        .register_asset_loader(CompositionAtlasLoader)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                tick_stage_time,
                (
                    prepare_composed_atlas_assets,
                    ensure_composed_enemy_parts,
                    update_composed_enemy_visuals,
                )
                    .chain(),
                cycle_horizon_profile,
                advance_walk,
                apply_depth_fallback_scale,
            ),
        )
        .run();
}

// --- Systems ---

/// Tick the stage time domain so composed animations advance.
fn tick_stage_time(time: Res<Time>, mut stage_time: ResMut<Time<StageTimeDomain>>) {
    stage_time.advance_by(time.delta());
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, profile: Res<HorizonProfile>) {
    // Primary camera for PxPlugin rendering.
    commands.spawn(Camera2d);

    // Overlay camera: renders gizmos on top of PxPlugin output.
    // PxOverlayCamera tells PxRenderNode to skip this camera.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        PxOverlayCamera,
    ));

    // Spawn Floor entities for depths 1..=9 so the shared DepthDebugPlugin
    // can draw them.
    for d in DEPTH_MIN..=DEPTH_MAX {
        let depth = Depth::try_from(d).unwrap();
        commands.spawn((
            DepthFloorLine,
            depth,
            Floor(floor_y_for_depth(d, profile.floor_depth_9)),
        ));
    }

    let initial_depth = Depth::Nine;

    commands.spawn((
        DepthWalker,
        ComposedAnimationState::new("walking_forward"),
        ComposedEnemyVisual::for_enemy(&asset_server, EnemyType::Mosquiton, Depth::Three),
        PxSubPosition::from(Vec2::new(CENTER_X, profile.floor_depth_9)),
        initial_depth,
        AuthoredDepths::single(Depth::Three),
        Layer::Front,
        PxAnchor::Center,
        WalkProgress {
            t: 0.0,
            direction: 1.0,
            phase: WalkPhase::Moving,
            airborne: false,
            half_trips: 0,
        },
    ));
}

/// Cycle horizon profile with `Ctrl+O` (`Cmd+O` on macOS).
///
/// When the profile changes, all [`DepthFloorLine`] entities are updated to
/// reflect the new depth-9 floor position.
fn cycle_horizon_profile(
    keys: Res<ButtonInput<KeyCode>>,
    mut profile: ResMut<HorizonProfile>,
    mut floor_query: Query<(&Depth, &mut Floor), With<DepthFloorLine>>,
) {
    let modifier_held = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if !modifier_held || !keys.just_pressed(KeyCode::KeyO) {
        return;
    }

    profile.index = (profile.index + 1) % HORIZON_FRACTIONS.len();
    profile.floor_depth_9 = HORIZON_FRACTIONS[profile.index] * SCREEN_H;

    info!(
        "Horizon profile {} — depth 9 floor at {:.0}% of screen height",
        profile.index,
        HORIZON_FRACTIONS[profile.index] * 100.0,
    );

    for (depth, mut floor) in &mut floor_query {
        floor.0 = floor_y_for_depth(depth.to_i8(), profile.floor_depth_9);
    }
}

/// Advance the oscillation state machine.
///
/// Walk progress `t ∈ [0, 1]` advances linearly over time (0 = depth 9 / horizon,
/// 1 = depth 1 / foreground). The screen-space floor position is derived from `t`
/// through [`floor_y_from_t`] which applies the perspective bias.
///
/// After every [`PASSES_BEFORE_SWITCH`] passes the mosquiton transitions
/// between walking and flying modes via liftoff/landing animations with short
/// pauses before and after the transition.
fn advance_walk(
    time: Res<Time>,
    profile: Res<HorizonProfile>,
    depth_scale: Res<DepthScaleConfig>,
    mut query: Query<
        (
            &mut WalkProgress,
            &mut Depth,
            &mut PxSubPosition,
            &mut ComposedAnimationState,
        ),
        With<DepthWalker>,
    >,
) {
    let dt = time.delta_secs();

    for (mut progress, mut depth, mut pos, mut anim) in &mut query {
        match progress.phase {
            WalkPhase::Idle { remaining } => {
                let remaining = remaining - dt;
                if remaining <= 0.0 {
                    // Check if we should transition between walk/fly modes.
                    if progress.half_trips >= PASSES_BEFORE_SWITCH {
                        progress.half_trips = 0;
                        let idle_tag = if progress.airborne {
                            "idle_fly"
                        } else {
                            "idle_stand"
                        };
                        anim.requested_tag = idle_tag.into();
                        progress.phase = WalkPhase::PreTransition {
                            remaining: TRANSITION_PAUSE_SECS,
                        };
                    } else {
                        // Normal endpoint: reverse direction and resume.
                        progress.direction = -progress.direction;
                        progress.phase = WalkPhase::Moving;
                        let move_tag = if progress.airborne {
                            if progress.direction > 0.0 {
                                "flying_forward"
                            } else {
                                "flying_backwards"
                            }
                        } else {
                            "walking_forward"
                        };
                        anim.requested_tag = move_tag.into();
                    }
                } else {
                    progress.phase = WalkPhase::Idle { remaining };
                }
            }

            WalkPhase::PreTransition { remaining } => {
                let remaining = remaining - dt;
                if remaining <= 0.0 {
                    if progress.airborne {
                        // Start landing — play idle_stand as transition pose.
                        // TODO: use a dedicated landing animation when authored.
                        anim.requested_tag = "idle_stand".into();
                        progress.phase = WalkPhase::Landing {
                            remaining: TRANSITION_ANIM_SECS,
                        };
                    } else {
                        // Start liftoff. Wing flap is handled by metadata
                        // part_overrides on flying animations.
                        anim.requested_tag = "liftoff".into();
                        progress.phase = WalkPhase::Liftoff {
                            remaining: TRANSITION_ANIM_SECS,
                        };
                    }
                } else {
                    progress.phase = WalkPhase::PreTransition { remaining };
                }
            }

            WalkPhase::Liftoff { remaining } => {
                let remaining = remaining - dt;
                if remaining <= 0.0 {
                    progress.airborne = true;
                    anim.requested_tag = "idle_fly".into();
                    progress.phase = WalkPhase::PostTransition {
                        remaining: TRANSITION_PAUSE_SECS,
                    };
                } else {
                    progress.phase = WalkPhase::Liftoff { remaining };
                }
            }

            WalkPhase::Landing { remaining } => {
                let remaining = remaining - dt;
                if remaining <= 0.0 {
                    progress.airborne = false;
                    anim.requested_tag = "idle_stand".into();
                    progress.phase = WalkPhase::PostTransition {
                        remaining: TRANSITION_PAUSE_SECS,
                    };
                } else {
                    progress.phase = WalkPhase::Landing { remaining };
                }
            }

            WalkPhase::PostTransition { remaining } => {
                let remaining = remaining - dt;
                if remaining <= 0.0 {
                    // Resume movement in current direction.
                    progress.direction = -progress.direction;
                    progress.phase = WalkPhase::Moving;
                    let move_tag = if progress.airborne {
                        if progress.direction > 0.0 {
                            "flying_forward"
                        } else {
                            "flying_backwards"
                        }
                    } else {
                        "walking_forward"
                    };
                    anim.requested_tag = move_tag.into();
                } else {
                    progress.phase = WalkPhase::PostTransition { remaining };
                }
            }

            WalkPhase::Moving => {
                let speed = 2.0 / PERIOD_SECS;
                progress.t += progress.direction * speed * dt;

                // Clamp and enter idle at endpoints.
                if progress.t >= 1.0 {
                    progress.t = 1.0;
                    progress.half_trips += 1;
                    let idle_tag = if progress.airborne {
                        "idle_fly"
                    } else {
                        "idle_stand"
                    };
                    progress.phase = WalkPhase::Idle {
                        remaining: ENDPOINT_PAUSE_SECS,
                    };
                    anim.requested_tag = idle_tag.into();
                } else if progress.t <= 0.0 {
                    progress.t = 0.0;
                    progress.half_trips += 1;
                    let idle_tag = if progress.airborne {
                        "idle_fly"
                    } else {
                        "idle_stand"
                    };
                    progress.phase = WalkPhase::Idle {
                        remaining: ENDPOINT_PAUSE_SECS,
                    };
                    anim.requested_tag = idle_tag.into();
                }
            }
        }

        let t = progress.t;

        let bin = (t * DEPTH_COUNT).floor().min(DEPTH_COUNT - 1.0) as i8;
        let new_depth_i8 = (DEPTH_MAX - bin).max(DEPTH_MIN);
        let new_depth = Depth::try_from(new_depth_i8).unwrap_or(Depth::Three);

        if *depth != new_depth {
            *depth = new_depth;
        }

        let floor_y = floor_y_from_t(t, profile.floor_depth_9);
        let y_offset = if progress.airborne {
            // Scale fly height by depth scale ratio so the visual gap
            // stays proportional to the sprite size at every depth.
            let scale = depth_scale.scale_for(new_depth).unwrap_or(1.0);
            FLY_HEIGHT_OFFSET * scale
        } else {
            0.0
        };
        pos.0 = Vec2::new(CENTER_X, floor_y + y_offset);
    }
}
