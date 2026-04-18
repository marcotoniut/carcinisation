#![allow(
    // Bevy ECS patterns.
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::type_complexity,
    // Numeric casts pervasive in coordinate math.
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    // Bevy Reflect derive generates `_field` bindings that are then used,
    // which clippy flags as `used_underscore_binding`.  #[allow] on the enum
    // does not propagate to the generated impl block.
    clippy::used_underscore_binding,
)]
//! Composed-enemy depth-traverse test.
//!
//! An enemy oscillates between depth 9 (horizon) and depth 1 (foreground),
//! validating authored depth sprite switching, fallback scaling, floor-line
//! placement, and visual coherence across the full visible depth range.
//!
//! Supports two characters:
//! - **Mosquiton** (default): walk/fly locomotion with liftoff/landing transitions
//! - **Spidey**: jump-based locomotion with idle pauses between arcs
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
//! `1` / `2` switches between Mosquiton and Spidey at runtime.
//!
//! Run with:
//! - `cargo run -p carcinisation --bin depth_traverse`
//! - `cargo run -p carcinisation --bin depth_traverse -- --character spidey`

use bevy::{asset::AssetMetaCheck, prelude::*};
#[cfg(feature = "brp")]
use bevy_brp_extras::BrpExtrasPlugin;
use carapace::{animation::PxAnimationPlugin, prelude::*};
use carcinisation::{
    globals::SCREEN_RESOLUTION,
    stage::{
        components::placement::{Airborne, AnchorOffsets, AuthoredDepths, Depth, Floor},
        depth_debug::{DepthDebugOverlay, DepthDebugPlugin},
        depth_scale::{DepthScaleConfig, apply_depth_fallback_scale},
        enemy::{
            composed::{
                ComposedAnimationPlaybackDebug, ComposedAnimationPlaybackDebugEnabled,
                ComposedAnimationState, ComposedEnemyVisual, CompositionAtlasAsset,
                CompositionAtlasLoader, apply_composed_enemy_visuals, ensure_composed_enemy_parts,
                prepare_composed_atlas_assets, update_composed_enemy_visuals,
            },
            entity::EnemyType,
        },
        messages::ComposedAnimationCueMessage,
        projection::ProjectionProfile,
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
const DEPTH_INTERVAL_COUNT: f32 = (DEPTH_MAX - DEPTH_MIN) as f32;

const VIEWPORT_SCALE: f32 = 4.0;

/// Duration of the idle pause at each oscillation endpoint before reversing.
const ENDPOINT_PAUSE_SECS: f32 = 2.0;

/// Short pause before/after liftoff and landing transitions.
const TRANSITION_PAUSE_SECS: f32 = 0.5;

/// How long the liftoff/landing animation pose is held.
const TRANSITION_ANIM_SECS: f32 = 0.5;

/// Number of passes (half-trips) before switching between walk and fly modes.
/// 3 = forward, backward, forward -- then transition.
const PASSES_BEFORE_SWITCH: u32 = 3;

/// How far the body centre rises above its grounded height during flight
/// (in carapace pixels).  Added to `ground − air` to yield the flight
/// altitude at which the air anchor hovers above the floor.  Scaled by
/// depth-fallback scale at runtime.
const FLY_HEIGHT_OFFSET: f32 = 24.0;

/// Idle pause between spidey jumps.
const SPIDEY_JUMP_IDLE_SECS: f32 = 1.0;

/// Duration of the spidey jump arc animation.
const SPIDEY_JUMP_ARC_SECS: f32 = 0.8;

/// Peak height of the spidey jump arc at depth scale 1.0.
///
/// Scaled by depth so far-depth jumps do not read taller than foreground jumps.
const SPIDEY_JUMP_ARC_HEIGHT: f32 = 80.0;

/// Duration of the landing freeze frame after a spidey jump.
const SPIDEY_LANDING_SECS: f32 = 0.3;

// --- Resources ---

/// Active horizon profile.  Wraps a [`ProjectionProfile`] and an index into
/// [`HORIZON_FRACTIONS`] for cycling via `Ctrl+O`.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct HorizonProfile {
    index: usize,
    #[reflect(ignore)]
    profile: ProjectionProfile,
}

impl Default for HorizonProfile {
    fn default() -> Self {
        Self {
            index: 0,
            profile: ProjectionProfile {
                horizon_y: HORIZON_FRACTIONS[0] * SCREEN_H,
                floor_base_y: FLOOR_DEPTH_1,
                bias_power: 3.0,
            },
        }
    }
}

impl HorizonProfile {
    fn floor_y_for_depth(&self, d: i8) -> f32 {
        self.profile.floor_y_for_depth(d)
    }
}

/// Which character is currently displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Resource, Reflect)]
#[reflect(Resource)]
enum SelectedCharacter {
    #[default]
    Mosquiton,
    Spidey,
}

impl SelectedCharacter {
    fn enemy_type(self) -> EnemyType {
        match self {
            Self::Mosquiton => EnemyType::Mosquiton,
            Self::Spidey => EnemyType::Spidey,
        }
    }

    fn authored_depth(self) -> Depth {
        match self {
            Self::Mosquiton | Self::Spidey => Depth::Three,
        }
    }

    fn initial_animation(self) -> &'static str {
        match self {
            Self::Mosquiton => "walk_forward",
            Self::Spidey => "idle",
        }
    }
}

// --- Components ---

#[derive(Component, Reflect)]
#[reflect(Component)]
struct DepthWalker;

/// Oscillation state machine for the depth walk.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct WalkProgress {
    t: f32,
    direction: f32,
    phase: WalkPhase,
    /// Whether we're currently in flying mode (mosquiton only).
    airborne: bool,
    /// Counts completed passes (half-trips). Each endpoint hit increments by 1.
    half_trips: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
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
    /// Spidey: idle pause between jumps.
    SpideyIdle { remaining: f32 },
    /// Spidey: mid-jump arc.
    SpideyJumping {
        elapsed: f32,
        start_depth: Depth,
        target_depth: Depth,
        start_t: f32,
        target_t: f32,
        start_y: f32,
        target_y: f32,
    },
    /// Spidey: landing freeze frame after jump.
    SpideyLanding { remaining: f32 },
}

/// Marker for Floor entities managed by this example (so we can update them).
#[derive(Component, Reflect)]
#[reflect(Component)]
struct DepthFloorLine;

#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
struct DepthTraverseScaleOverride {
    /// Extra multiplier stacked above the discrete depth fallback scale while
    /// Spidey is visually between depth steps.
    applied: Vec2,
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
struct DepthTraverseDebugState {
    selected_character: String,
    phase: String,
    current_depth: i8,
    target_depth: i8,
    jump_elapsed_secs: f32,
    jump_progress: f32,
    current_floor_y: f32,
    current_y: f32,
    target_y: f32,
    current_scale: f32,
    active_animation_tag: String,
    active_animation_source_frame: usize,
    active_animation_frame_index: usize,
    holding_last_frame: bool,
}

impl Default for DepthTraverseDebugState {
    fn default() -> Self {
        Self {
            selected_character: String::new(),
            phase: String::new(),
            current_depth: DEPTH_MAX,
            target_depth: DEPTH_MAX,
            jump_elapsed_secs: 0.0,
            jump_progress: 0.0,
            current_floor_y: 0.0,
            current_y: 0.0,
            target_y: 0.0,
            current_scale: 1.0,
            active_animation_tag: String::new(),
            active_animation_source_frame: 0,
            active_animation_frame_index: 0,
            holding_last_frame: false,
        }
    }
}

// --- Layer (local -- the game's Layer is crate-private) ---

#[px_layer]
enum Layer {
    #[default]
    Back,
    Front,
}

// --- Perspective mapping (delegates to shared ProjectionProfile) ---

/// Map a normalised `t` in `[0, 1]` (0 = horizon, 1 = foreground) to floor Y.
///
/// Kept as a local helper for the continuous-t walk interpolation.  Delegates
/// to the same cubic bias as [`ProjectionProfile`] with `bias_power = 3.0`.
fn floor_y_from_t(t: f32, floor_depth_9: f32) -> f32 {
    let biased = t * t * t; // cubic, matches ProjectionProfile default bias
    floor_depth_9 + biased * (FLOOR_DEPTH_1 - floor_depth_9)
}

fn depth_to_t(depth: Depth) -> f32 {
    f32::from(DEPTH_MAX - depth.to_i8()) / DEPTH_INTERVAL_COUNT
}

fn adjacent_depth(current: Depth, direction: f32) -> Depth {
    if direction > 0.0 {
        current - 1
    } else {
        current + 1
    }
}

// --- CLI parsing ---

fn parse_character_from_args() -> SelectedCharacter {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if arg == "--character"
            && let Some(value) = args.get(i + 1)
        {
            return match value.to_lowercase().as_str() {
                "spidey" | "spider" => SelectedCharacter::Spidey,
                "mosquiton" => SelectedCharacter::Mosquiton,
                _ => {
                    eprintln!("Unknown character '{value}', defaulting to mosquiton");
                    SelectedCharacter::Mosquiton
                }
            };
        }
    }
    SelectedCharacter::Mosquiton
}

// --- Entry point ---

fn main() {
    let initial_character = parse_character_from_args();
    let title = match initial_character {
        SelectedCharacter::Mosquiton => "Depth Traverse - Mosquiton",
        SelectedCharacter::Spidey => "Depth Traverse - Spidey",
    };

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: title.into(),
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
    .register_type::<ComposedAnimationPlaybackDebug>()
    .register_type::<ComposedAnimationPlaybackDebugEnabled>()
    .register_type::<Airborne>()
    .register_type::<AnchorOffsets>()
    .register_type::<DepthFloorLine>()
    .register_type::<DepthTraverseScaleOverride>()
    .register_type::<DepthTraverseDebugState>()
    .register_type::<DepthWalker>()
    .register_type::<HorizonProfile>()
    .register_type::<SelectedCharacter>()
    .register_type::<WalkPhase>()
    .register_type::<WalkProgress>()
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(DepthScaleConfig::load_or_default())
    .init_resource::<HorizonProfile>()
    .insert_resource(initial_character)
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
                prepare_composed_atlas_assets,
                ensure_composed_enemy_parts,
                cycle_horizon_profile,
                switch_character,
                advance_walk,
                apply_spidey_jump_scale_override,
                update_composed_enemy_visuals,
                update_depth_traverse_debug_state,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            (apply_depth_fallback_scale, apply_composed_enemy_visuals),
        )
        .run();
}

// --- Systems ---

/// Tick the stage time domain so composed animations advance.
fn tick_stage_time(time: Res<Time>, mut stage_time: ResMut<Time<StageTimeDomain>>) {
    stage_time.advance_by(time.delta());
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    profile: Res<HorizonProfile>,
    selected: Res<SelectedCharacter>,
) {
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
        commands.spawn((DepthFloorLine, depth, Floor(profile.floor_y_for_depth(d))));
    }

    spawn_walker(&mut commands, &asset_server, &profile, *selected);
}

fn spawn_walker(
    commands: &mut Commands,
    asset_server: &AssetServer,
    profile: &HorizonProfile,
    character: SelectedCharacter,
) {
    let initial_depth = Depth::Nine;
    let initial_phase = match character {
        SelectedCharacter::Mosquiton => WalkPhase::Moving,
        SelectedCharacter::Spidey => WalkPhase::SpideyIdle {
            remaining: SPIDEY_JUMP_IDLE_SECS,
        },
    };

    commands.spawn((
        DepthWalker,
        Name::new("DepthTraverseWalker"),
        ComposedAnimationState::new(character.initial_animation()),
        ComposedAnimationPlaybackDebugEnabled,
        ComposedEnemyVisual::for_enemy(
            asset_server,
            character.enemy_type(),
            character.authored_depth(),
        ),
        DepthTraverseDebugState::default(),
        PxSubPosition::from(Vec2::new(CENTER_X, profile.profile.horizon_y)),
        initial_depth,
        AuthoredDepths::single(character.authored_depth()),
        Layer::Front,
        PxAnchor::Center,
        WalkProgress {
            t: 0.0,
            direction: 1.0,
            phase: initial_phase,
            airborne: false,
            half_trips: 0,
        },
    ));
}

/// Switch character with keyboard `1` (Mosquiton) or `2` (Spidey).
fn switch_character(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    profile: Res<HorizonProfile>,
    mut selected: ResMut<SelectedCharacter>,
    walker_query: Query<Entity, With<DepthWalker>>,
) {
    let new_character = if keys.just_pressed(KeyCode::Digit1) {
        Some(SelectedCharacter::Mosquiton)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(SelectedCharacter::Spidey)
    } else {
        None
    };

    let Some(new_character) = new_character else {
        return;
    };

    if *selected == new_character {
        return;
    }

    info!("Switching to {:?}", new_character);
    *selected = new_character;

    // Despawn existing walker.
    for entity in &walker_query {
        commands.entity(entity).despawn();
    }

    spawn_walker(&mut commands, &asset_server, &profile, new_character);
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
    profile.profile.horizon_y = HORIZON_FRACTIONS[profile.index] * SCREEN_H;

    info!(
        "Horizon profile {} -- depth 9 floor at {:.0}% of screen height",
        profile.index,
        HORIZON_FRACTIONS[profile.index] * 100.0,
    );

    for (depth, mut floor) in &mut floor_query {
        floor.0 = profile.floor_y_for_depth(depth.to_i8());
    }
}

#[allow(clippy::type_complexity)]
/// Advance the oscillation state machine.
///
/// Walk progress `t` in `[0, 1]` advances linearly over time (0 = depth 9 / horizon,
/// 1 = depth 1 / foreground). The screen-space floor position is derived from `t`
/// through [`floor_y_from_t`] which applies the perspective bias.
///
/// Mosquiton: after every [`PASSES_BEFORE_SWITCH`] passes, transitions
/// between walking and flying modes via liftoff/landing animations.
///
/// Spidey: alternates between idle poses and jump arcs, advancing one
/// depth level per jump.
fn advance_walk(
    mut commands: Commands,
    time: Res<Time>,
    profile: Res<HorizonProfile>,
    depth_scale: Res<DepthScaleConfig>,
    selected: Res<SelectedCharacter>,
    mut query: Query<
        (
            Entity,
            &mut WalkProgress,
            &mut Depth,
            &mut PxSubPosition,
            &mut ComposedAnimationState,
            Option<&AnchorOffsets>,
            Has<Airborne>,
        ),
        With<DepthWalker>,
    >,
) {
    let dt = time.delta_secs();

    for (entity, mut progress, mut depth, mut pos, mut anim, anchors, is_airborne) in &mut query {
        if *selected == SelectedCharacter::Mosquiton {
            advance_mosquiton(&mut progress, &mut anim, dt);

            // AnchorOffsets is inserted by update_composed_enemy_visuals
            // once the atlas loads.  Until then, skip positioning.
            let Some(anchors) = anchors else {
                continue;
            };

            let t = progress.t;
            let bin = (t * DEPTH_COUNT).floor().min(DEPTH_COUNT - 1.0) as i8;
            let new_depth_i8 = (DEPTH_MAX - bin).max(DEPTH_MIN);
            let new_depth = Depth::try_from(new_depth_i8).unwrap_or(Depth::Three);

            if *depth != new_depth {
                *depth = new_depth;
            }

            // Sync the Airborne marker with the state machine.
            //
            // Uses deferred commands, so Has<Airborne> lags progress.airborne
            // by one frame.  This is safe: placement (below) reads
            // progress.airborne directly, so entity_y is always correct.
            // The only consumer of the marker is depth_debug's green line,
            // which shows the old anchor for one transition frame — a
            // cosmetic-only artefact, not a gameplay or positioning issue.
            if progress.airborne && !is_airborne {
                commands.entity(entity).insert(Airborne);
            } else if !progress.airborne && is_airborne {
                commands.entity(entity).remove::<Airborne>();
            }

            let floor_y = floor_y_from_t(t, profile.profile.horizon_y);
            let fallback_scale = depth_scale.resolve_fallback(
                new_depth,
                &AuthoredDepths::single(SelectedCharacter::Mosquiton.authored_depth()),
            );

            // State-based anchor selection.
            //
            // NOTE: both branches are still floor-relative — the airborne
            // reference is derived from floor_y, not from an independent
            // target.  This is correct for the depth_traverse demo where
            // the entity tracks the floor at every depth.
            if progress.airborne {
                // Airborne: air anchor (body centre) at flight altitude.
                let body_height_grounded = anchors.ground - anchors.air;
                let flight_altitude = body_height_grounded + FLY_HEIGHT_OFFSET;
                let airborne_ref = floor_y + flight_altitude * fallback_scale;
                pos.0 = Vec2::new(CENTER_X, airborne_ref + anchors.air * fallback_scale);
            } else {
                // Grounded: ground anchor sits on the floor.
                pos.0 = Vec2::new(CENTER_X, floor_y + anchors.ground * fallback_scale);
            }
        } else {
            advance_spidey(
                &mut progress,
                &mut anim,
                &profile,
                &depth_scale,
                &mut pos,
                &mut depth,
                dt,
            );
        }
    }
}

/// Mosquiton walk/fly state machine.
#[allow(clippy::too_many_lines)]
fn advance_mosquiton(progress: &mut WalkProgress, anim: &mut ComposedAnimationState, dt: f32) {
    match progress.phase {
        WalkPhase::Idle { remaining } => {
            let remaining = remaining - dt;
            if remaining <= 0.0 {
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
                    progress.direction = -progress.direction;
                    progress.phase = WalkPhase::Moving;
                    let move_tag = if progress.airborne {
                        if progress.direction > 0.0 {
                            "fly_forward"
                        } else {
                            "fly_backward"
                        }
                    } else {
                        "walk_forward"
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
                    anim.requested_tag = "idle_stand".into();
                    progress.phase = WalkPhase::Landing {
                        remaining: TRANSITION_ANIM_SECS,
                    };
                } else {
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
                progress.direction = -progress.direction;
                progress.phase = WalkPhase::Moving;
                let move_tag = if progress.airborne {
                    if progress.direction > 0.0 {
                        "fly_forward"
                    } else {
                        "fly_backward"
                    }
                } else {
                    "walk_forward"
                };
                anim.requested_tag = move_tag.into();
            } else {
                progress.phase = WalkPhase::PostTransition { remaining };
            }
        }

        WalkPhase::Moving => {
            let speed = 2.0 / PERIOD_SECS;
            progress.t += progress.direction * speed * dt;

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

        // Spidey-only phases are unreachable for mosquiton.
        WalkPhase::SpideyIdle { .. }
        | WalkPhase::SpideyJumping { .. }
        | WalkPhase::SpideyLanding { .. } => {}
    }
}

/// Spidey jump-based state machine.
///
/// Spidey idles, then jumps to the next depth level, landing on the floor
/// line. The jump arc uses a simple parabolic interpolation.
fn advance_spidey(
    progress: &mut WalkProgress,
    anim: &mut ComposedAnimationState,
    profile: &HorizonProfile,
    depth_scale: &DepthScaleConfig,
    pos: &mut PxSubPosition,
    depth: &mut Depth,
    dt: f32,
) {
    match progress.phase {
        WalkPhase::SpideyIdle { remaining } => {
            let remaining = remaining - dt;
            if remaining <= 0.0 {
                if (progress.direction > 0.0 && *depth == Depth::One)
                    || (progress.direction < 0.0 && *depth == Depth::Nine)
                {
                    progress.half_trips += 1;
                    progress.direction = -progress.direction;
                }

                let start_depth = *depth;
                let target_depth = adjacent_depth(start_depth, progress.direction);
                let start_t = depth_to_t(start_depth);
                let target_t = depth_to_t(target_depth);
                let start_y = profile.floor_y_for_depth(start_depth.to_i8());
                let target_y = profile.floor_y_for_depth(target_depth.to_i8());

                anim.requested_tag = "jump".into();
                anim.set_hold_last_frame(true);
                progress.phase = WalkPhase::SpideyJumping {
                    elapsed: 0.0,
                    start_depth,
                    target_depth,
                    start_t,
                    target_t,
                    start_y,
                    target_y,
                };
            } else {
                progress.phase = WalkPhase::SpideyIdle { remaining };
            }
        }

        WalkPhase::SpideyJumping {
            elapsed,
            start_depth: _,
            target_depth,
            start_t,
            target_t,
            start_y,
            target_y,
        } => {
            let elapsed = elapsed + dt;
            if elapsed >= SPIDEY_JUMP_ARC_SECS {
                *depth = target_depth;
                progress.t = target_t;
                pos.0 = Vec2::new(CENTER_X, target_y);
                anim.requested_tag = "landing".into();
                anim.set_hold_last_frame(false);

                progress.phase = WalkPhase::SpideyLanding {
                    remaining: SPIDEY_LANDING_SECS,
                };
            } else {
                let frac = elapsed / SPIDEY_JUMP_ARC_SECS;
                if frac >= 0.5 && anim.requested_tag != "landing" {
                    anim.requested_tag = "landing".into();
                    anim.set_hold_last_frame(false);
                }

                let travel_t = start_t + (target_t - start_t) * frac;
                let y_linear = start_y + (target_y - start_y) * frac;
                let scale = spidey_jump_arc_scale(depth_scale, *depth, target_depth, frac);
                let arc = 4.0 * SPIDEY_JUMP_ARC_HEIGHT * scale * frac * (1.0 - frac);
                pos.0 = Vec2::new(CENTER_X, y_linear + arc);
                progress.t = travel_t;

                progress.phase = WalkPhase::SpideyJumping {
                    elapsed,
                    start_depth: *depth,
                    target_depth,
                    start_t,
                    target_t,
                    start_y,
                    target_y,
                };
            }
        }

        WalkPhase::SpideyLanding { remaining } => {
            let remaining = remaining - dt;
            if remaining <= 0.0 {
                anim.requested_tag = "idle".into();
                anim.set_hold_last_frame(false);
                progress.phase = WalkPhase::SpideyIdle {
                    remaining: SPIDEY_JUMP_IDLE_SECS,
                };
            } else {
                progress.phase = WalkPhase::SpideyLanding { remaining };
            }
        }

        // Mosquiton-only phases are unreachable for spidey.
        WalkPhase::Moving
        | WalkPhase::Idle { .. }
        | WalkPhase::PreTransition { .. }
        | WalkPhase::Liftoff { .. }
        | WalkPhase::Landing { .. }
        | WalkPhase::PostTransition { .. } => {}
    }
}

fn spidey_jump_arc_scale(
    depth_scale: &DepthScaleConfig,
    start_depth: Depth,
    target_depth: Depth,
    frac: f32,
) -> f32 {
    let start_scale = depth_scale.scale_for(start_depth).unwrap_or(1.0);
    let target_scale = depth_scale.scale_for(target_depth).unwrap_or(start_scale);
    start_scale + (target_scale - start_scale) * frac.clamp(0.0, 1.0)
}

fn spidey_depth_ratio(
    depth_scale: &DepthScaleConfig,
    authored_depths: &AuthoredDepths,
    depth: Depth,
) -> f32 {
    depth_scale.resolve_fallback(depth, authored_depths)
}

fn spidey_interpolated_depth_ratio(
    depth_scale: &DepthScaleConfig,
    authored_depths: &AuthoredDepths,
    start_depth: Depth,
    target_depth: Depth,
    frac: f32,
) -> f32 {
    let start = spidey_depth_ratio(depth_scale, authored_depths, start_depth);
    let target = spidey_depth_ratio(depth_scale, authored_depths, target_depth);
    start + (target - start) * frac.clamp(0.0, 1.0)
}

#[allow(clippy::type_complexity)]
fn apply_spidey_jump_scale_override(
    mut commands: Commands,
    selected: Res<SelectedCharacter>,
    depth_scale: Res<DepthScaleConfig>,
    mut query: Query<
        (
            Entity,
            &Depth,
            &AuthoredDepths,
            &WalkProgress,
            Option<&mut PxPresentationTransform>,
            Option<&DepthTraverseScaleOverride>,
        ),
        With<DepthWalker>,
    >,
) {
    if *selected != SelectedCharacter::Spidey {
        return;
    }

    for (entity, depth, authored_depths, progress, presentation, previous_override) in &mut query {
        let discrete_ratio = spidey_depth_ratio(&depth_scale, authored_depths, *depth);
        let desired_ratio = match progress.phase {
            WalkPhase::SpideyJumping {
                elapsed,
                start_depth,
                target_depth,
                ..
            } => spidey_interpolated_depth_ratio(
                &depth_scale,
                authored_depths,
                start_depth,
                target_depth,
                elapsed / SPIDEY_JUMP_ARC_SECS,
            ),
            _ => spidey_depth_ratio(&depth_scale, authored_depths, *depth),
        };

        let desired_override = Vec2::splat(desired_ratio / discrete_ratio.max(f32::EPSILON));
        let previous_scale =
            previous_override.map_or(Vec2::ONE, |override_scale| override_scale.applied);

        if let Some(mut presentation) = presentation {
            let sign_x = presentation.scale.x.signum();
            let sign_y = presentation.scale.y.signum();
            let base_x = presentation.scale.x.abs() / previous_scale.x;
            let base_y = presentation.scale.y.abs() / previous_scale.y;

            presentation.scale = Vec2::new(
                sign_x * base_x * desired_override.x,
                sign_y * base_y * desired_override.y,
            );
        } else if (desired_override - Vec2::ONE).length_squared() >= f32::EPSILON {
            commands.entity(entity).insert(PxPresentationTransform {
                scale: desired_override,
                ..Default::default()
            });
        }

        if (desired_override - Vec2::ONE).length_squared() < f32::EPSILON {
            commands
                .entity(entity)
                .remove::<DepthTraverseScaleOverride>();
        } else {
            commands.entity(entity).insert(DepthTraverseScaleOverride {
                applied: desired_override,
            });
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_lines)]
fn update_depth_traverse_debug_state(
    selected: Res<SelectedCharacter>,
    profile: Res<HorizonProfile>,
    depth_scale: Res<DepthScaleConfig>,
    mut query: Query<
        (
            &Depth,
            &PxSubPosition,
            &WalkProgress,
            &ComposedAnimationState,
            Option<&ComposedAnimationPlaybackDebug>,
            &mut DepthTraverseDebugState,
        ),
        With<DepthWalker>,
    >,
) {
    for (depth, pos, progress, anim, playback, mut debug) in &mut query {
        let (phase, target_depth, jump_elapsed_secs, jump_progress, target_y) = match progress.phase
        {
            WalkPhase::Moving => (
                "moving".to_string(),
                depth.to_i8(),
                0.0,
                0.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
            WalkPhase::Idle { .. } => (
                "idle".to_string(),
                depth.to_i8(),
                0.0,
                0.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
            WalkPhase::PreTransition { .. } => (
                "pre_transition".to_string(),
                depth.to_i8(),
                0.0,
                0.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
            WalkPhase::Liftoff { .. } => (
                "liftoff".to_string(),
                depth.to_i8(),
                0.0,
                0.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
            WalkPhase::Landing { .. } => (
                "landing".to_string(),
                depth.to_i8(),
                0.0,
                0.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
            WalkPhase::PostTransition { .. } => (
                "post_transition".to_string(),
                depth.to_i8(),
                0.0,
                0.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
            WalkPhase::SpideyIdle { .. } => (
                "spidey_idle".to_string(),
                depth.to_i8(),
                0.0,
                0.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
            WalkPhase::SpideyJumping {
                elapsed,
                target_depth,
                target_y,
                ..
            } => {
                let progress = (elapsed / SPIDEY_JUMP_ARC_SECS).clamp(0.0, 1.0);
                let phase = if progress < 0.5 {
                    "spidey_jump_ascent"
                } else {
                    "spidey_jump_descent"
                };
                (
                    phase.to_string(),
                    target_depth.to_i8(),
                    elapsed,
                    progress,
                    target_y,
                )
            }
            WalkPhase::SpideyLanding { .. } => (
                "spidey_landing_hold".to_string(),
                depth.to_i8(),
                SPIDEY_JUMP_ARC_SECS,
                1.0,
                profile.floor_y_for_depth(depth.to_i8()),
            ),
        };

        let current_floor_y = profile.floor_y_for_depth(depth.to_i8());
        let authored_depths = AuthoredDepths::single(selected.authored_depth());
        let current_scale = match progress.phase {
            WalkPhase::SpideyJumping {
                elapsed,
                start_depth,
                target_depth,
                ..
            } if *selected == SelectedCharacter::Spidey => spidey_interpolated_depth_ratio(
                &depth_scale,
                &authored_depths,
                start_depth,
                target_depth,
                elapsed / SPIDEY_JUMP_ARC_SECS,
            ),
            _ => spidey_depth_ratio(&depth_scale, &authored_depths, *depth),
        };
        let (active_animation_source_frame, active_animation_frame_index, holding_last_frame) =
            playback
                .and_then(|state| state.tracks.last())
                .map_or((0, 0, false), |track| {
                    (track.source_frame, track.frame_index, track.hold_last_frame)
                });

        *debug = DepthTraverseDebugState {
            selected_character: match *selected {
                SelectedCharacter::Mosquiton => "mosquiton".to_string(),
                SelectedCharacter::Spidey => "spidey".to_string(),
            },
            phase,
            current_depth: depth.to_i8(),
            target_depth,
            jump_elapsed_secs,
            jump_progress,
            current_floor_y,
            current_y: pos.y,
            target_y,
            current_scale,
            active_animation_tag: anim.requested_tag.clone(),
            active_animation_source_frame,
            active_animation_frame_index,
            holding_last_frame,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spidey_depth_traverse_uses_eight_distinct_frontward_steps() {
        let mut depth = Depth::Nine;
        let mut visited = Vec::new();

        for _ in 0..8 {
            depth = adjacent_depth(depth, 1.0);
            visited.push(depth.to_i8());
        }

        assert_eq!(visited, vec![8, 7, 6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn frontmost_depths_are_visibly_distinct() {
        let profile = ProjectionProfile {
            horizon_y: HORIZON_FRACTIONS[0] * SCREEN_H,
            floor_base_y: FLOOR_DEPTH_1,
            bias_power: 3.0,
        };
        let depth_two_y = profile.floor_y_for_depth(2);
        let depth_one_y = profile.floor_y_for_depth(1);

        assert!(
            (depth_one_y - depth_two_y).abs() > 20.0,
            "expected a clear front-depth separation, got {}",
            (depth_one_y - depth_two_y).abs()
        );
    }

    #[test]
    fn spidey_jump_arc_scales_larger_toward_foreground() {
        let depth_scale = DepthScaleConfig::default();

        let far = spidey_jump_arc_scale(&depth_scale, Depth::Nine, Depth::Eight, 0.5);
        let near = spidey_jump_arc_scale(&depth_scale, Depth::Two, Depth::One, 0.5);

        assert!(
            near > far,
            "expected foreground jump scale to exceed far-depth scale, got near={near} far={far}"
        );
    }

    #[test]
    fn spidey_scale_interpolates_between_depths_during_jump() {
        let depth_scale = DepthScaleConfig::default();
        let authored = AuthoredDepths::single(Depth::Three);

        let start = spidey_depth_ratio(&depth_scale, &authored, Depth::Two);
        let target = spidey_depth_ratio(&depth_scale, &authored, Depth::One);
        let midpoint =
            spidey_interpolated_depth_ratio(&depth_scale, &authored, Depth::Two, Depth::One, 0.5);

        assert!(midpoint > start);
        assert!(midpoint < target);
    }
}
