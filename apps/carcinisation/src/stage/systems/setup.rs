use super::spawn::{spawn_destructible, spawn_enemy, spawn_object, spawn_pickup};
use crate::{
    components::VolumeSettings,
    game::GameProgressState,
    globals::SCREEN_RESOLUTION,
    layer::Layer,
    pixel::CxAssets,
    stage::{
        StagePlugin,
        bundles::{BackgroundBundle, SkyboxBundle},
        components::{Stage, StageEntity},
        data::{
            BandTransition, PrimitiveBandConfig, StageData, StagePrimitive, StagePrimitiveAnchor,
            StagePrimitiveFill, StagePrimitiveLayer, StagePrimitiveShape, StageSpawn,
        },
        floors::{ActiveSurfaceLayout, effective_floor_layout, evaluate_floors_at},
        messages::StageStartupEvent,
        player::messages::PlayerStartupEvent,
        projection::{ProjectionProfile, effective_projection, validate_stage_projections},
        resources::{self, ActiveProjection, StageGravity},
        ui::hud::spawn::spawn_hud,
    },
    systems::spawn::make_music_bundle,
    transitions::trigger_transition,
};
use std::time::Duration;

use activable::activate;
use bevy::{audio::PlaybackMode, prelude::*};
use carapace::prelude::{CxFilter, CxSprite, CxTypeface};

/// @trigger Builds the stage world: spawns HUD, enemies, pickups, background, and music.
///
/// When `from_checkpoint` is set, uses the stage's authored checkpoint
/// coordinates for the camera and skips initial gameplay spawns (enemies,
/// destructibles, pickups) that precede the checkpoint.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn on_stage_startup(
    trigger: On<StageStartupEvent>,
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameProgressState>>,
    mut assets_sprite: CxAssets<CxSprite>,
    _filters: CxAssets<CxFilter>,
    mut typefaces: CxAssets<CxTypeface>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    let event = trigger.event();
    let data = event.data.as_ref();
    let from_checkpoint = event.from_checkpoint;

    if let Err(e) = validate_stage_projections(data) {
        panic!("Stage '{}' failed projection validation: {e}", data.name);
    }

    if let Some(checkpoint) = &data.checkpoint {
        assert!(
            checkpoint.step_index < data.steps.len(),
            "Stage '{}' checkpoint step_index {} exceeds steps count {}",
            data.name,
            checkpoint.step_index,
            data.steps.len()
        );
    }

    // Required for the initial game-start path where no prior activate
    // has run.  On checkpoint restart this is a safe no-op (already active).
    activate::<StagePlugin>(&mut commands);

    // When resuming from checkpoint, override start_coordinates so the
    // camera system (`initialise_camera_from_stage`) picks up the
    // checkpoint position instead of the stage's original origin.
    //
    // NOTE: this mutates the resource copy.  `handle_stage_restart` reads
    // from `Res<StageData>`, so all subsequent restart triggers carry the
    // overridden coordinates.  This is safe because the only current
    // restart writer (`handle_death_screen_continue`) always sets
    // `from_checkpoint: true`, re-applying the same override.  If a
    // non-checkpoint restart path is ever added, it must source the
    // original `StageData` from the event's `Arc` rather than the resource.
    let mut effective_data = data.clone();
    if from_checkpoint && let Some(checkpoint) = &data.checkpoint {
        effective_data.start_coordinates = checkpoint.start_coordinates;
    }
    let start_x = effective_data.start_coordinates.x;
    commands.insert_resource::<StageData>(effective_data);

    // Set stage-specific gravity or use default
    let gravity = if let Some(gravity_value) = data.gravity {
        StageGravity::new(gravity_value)
    } else {
        StageGravity::standard()
    };
    commands.insert_resource(gravity);

    // Skip the start transition on checkpoint restart — it is a one-time
    // stage entry effect that should not replay on continue.
    if !from_checkpoint && let Some(request) = &data.on_start_transition_o {
        trigger_transition(&mut commands, request);
    }

    spawn_hud(&mut commands, &mut typefaces, &asset_server);

    let initial_projection = effective_projection(data, 0);
    let initial_active_projection = ActiveProjection(initial_projection);
    let initial_floor_layout = effective_floor_layout(data, 0);
    let initial_floors = evaluate_floors_at(data, Duration::ZERO);
    let depth_scale_config = crate::stage::depth_scale::DepthScaleConfig::load_or_default();

    commands.insert_resource(initial_active_projection);
    commands.insert_resource(ActiveSurfaceLayout(initial_floor_layout.clone()));
    commands.insert_resource(initial_floors.clone());
    // Lateral parallax anchor: captures camera X at stage entry.
    //
    // On checkpoint resume, this re-captures from the checkpoint's start
    // coordinates, not the stage origin. This means lateral parallax is
    // relative to entry-point camera position — motion since the player
    // began this run — which is the perceptually meaningful frame.
    //
    // Consequence: a player checkpointing into mid-stage gets a different
    // "zero" than a player who played through from the beginning. The
    // parallax response to the same camera tween will differ between those
    // playthroughs. This is intentional.
    //
    // TODO: alternative — anchor against stage_data.start_coordinates.x
    // unconditionally, making parallax independent of entry point at the
    // cost of decoupling from the player's perceived camera motion.
    let initial_projection_view = resources::ProjectionView {
        lateral_anchor_x: start_x,
        ..Default::default()
    };
    commands.insert_resource(initial_projection_view);

    for spawn in &data.spawns {
        // Skip gameplay entities when resuming from checkpoint — they
        // belong to the pre-checkpoint portion of the stage.  Objects
        // (static scenery) always spawn for visual consistency.
        if from_checkpoint && !matches!(spawn, StageSpawn::Object(_)) {
            continue;
        }

        #[cfg(debug_assertions)]
        info!("Spawning {:?}", spawn.show_type());

        match spawn {
            StageSpawn::Object(spawn) => {
                spawn_object(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Destructible(spawn) => {
                spawn_destructible(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Enemy(spawn) => {
                spawn_enemy(
                    &mut commands,
                    &asset_server,
                    Vec2::ZERO,
                    spawn,
                    &initial_floors,
                    &depth_scale_config,
                    Some(&initial_active_projection),
                    Some(&initial_projection_view),
                    None,
                );
            }
            StageSpawn::Pickup(spawn) => {
                spawn_pickup(&mut commands, &asset_server, Vec2::ZERO, spawn);
            }
        }
    }

    // Spawn generated primitive bands if configured.
    // Spawn individually authored primitives.
    for prim in &data.primitives {
        spawn_stage_primitive(&mut commands, prim);
    }

    // Spawn generated primitive bands (produces additional primitives).
    if let Some(bands) = &data.primitive_bands {
        spawn_primitive_bands(&mut commands, bands, &initial_projection);
    }

    commands
        .spawn((Stage, Name::new("Stage"), Visibility::Visible))
        .with_children(|p0| {
            if !data.background_path.is_empty() {
                p0.spawn(BackgroundBundle::new(
                    assets_sprite.load(data.background_path.clone()),
                ));
            }
            if !data.skybox.path.is_empty() {
                p0.spawn(SkyboxBundle::new(&mut assets_sprite, data.skybox.clone()));
            }
        });

    // TODO turn this into a spawn, like in cutscene, or make it a StageSpawn
    if !data.music_path.is_empty() {
        let (player, settings, system_bundle, music_tag) = make_music_bundle(
            &asset_server,
            &volume_settings,
            data.music_path.clone(),
            PlaybackMode::Loop,
        );
        commands.spawn((player, settings, system_bundle, music_tag, StageEntity));
    }

    *next_game_state = NextState::PendingIfNeq(GameProgressState::Running);
    commands.trigger(PlayerStartupEvent);
}

// ---------------------------------------------------------------------------
// Stage primitive spawning
// ---------------------------------------------------------------------------

/// Spawns a single [`StagePrimitive`] as a [`CxPrimitive`] entity.
fn spawn_stage_primitive(commands: &mut Commands, prim: &StagePrimitive) {
    use carapace::prelude::*;

    let shape = match &prim.shape {
        StagePrimitiveShape::Rect { width, height } => CxPrimitiveShape::Rect {
            size: UVec2::new(*width, *height),
        },
    };

    let fill = match &prim.fill {
        StagePrimitiveFill::Solid(idx) => CxPrimitiveFill::Solid(*idx),
        StagePrimitiveFill::Checker { a, b } => CxPrimitiveFill::Checker { a: *a, b: *b },
        StagePrimitiveFill::OrderedDither { a, b, threshold } => CxPrimitiveFill::OrderedDither {
            a: *a,
            b: *b,
            threshold: *threshold,
        },
    };

    let anchor = match prim.anchor {
        StagePrimitiveAnchor::TopLeft => CxAnchor::TopLeft,
        StagePrimitiveAnchor::TopCenter => CxAnchor::TopCenter,
        StagePrimitiveAnchor::BottomCenter => CxAnchor::BottomCenter,
        StagePrimitiveAnchor::BottomLeft => CxAnchor::BottomLeft,
        StagePrimitiveAnchor::Center => CxAnchor::Center,
    };

    let canvas = if prim.camera_space {
        CxRenderSpace::Camera
    } else {
        CxRenderSpace::World
    };

    let layer = match prim.layer {
        StagePrimitiveLayer::Skybox => Layer::Skybox,
        StagePrimitiveLayer::Background => Layer::Background,
    };

    commands.spawn((
        Name::new("StagePrimitive"),
        CxPrimitive { shape, fill },
        CxPosition::from(prim.position),
        anchor,
        canvas,
        layer,
        StageEntity,
    ));
}

// ---------------------------------------------------------------------------
// Primitive band generation
// ---------------------------------------------------------------------------

/// Spawns horizontal primitive rect bands from the RON-driven config.
///
/// Bands follow the projection floor-Y positions per depth. Each colour maps
/// to a pair of adjacent depth lanes. Transition bands (checker/dither) fill
/// the region between a solid band and the next colour.
///
/// Rects are a temporary approximation — they cannot match the perspective
/// grid's converging rays. Polygon/trapezoid primitives will be needed for
/// true projected floor geometry.
fn spawn_primitive_bands(
    commands: &mut Commands,
    config: &PrimitiveBandConfig,
    projection: &ProjectionProfile,
) {
    use carapace::prelude::*;

    struct Band {
        y_bottom: i32,
        y_top: i32,
        fill: CxPrimitiveFill,
        label: &'static str,
    }

    let band_w = (SCREEN_RESOLUTION.x as f32 * config.width_multiplier).round() as u32;
    let colors = &config.ground_colors;

    if colors.is_empty() {
        return;
    }

    // Bands cover depths 1→9, matching the perspective grid which draws
    // floor lines for depths 1-9. This gives 8 gaps. Depth 0 is excluded
    // because its floor Y is well below the visible screen.
    //
    // For N ground colours we produce 2N visual segments:
    //   checker(skybox_color, colors[0])  — 1 gap
    //   solid(colors[0])                  — K gaps
    //   checker(colors[0], colors[1])     — 1 gap
    //   solid(colors[1])                  — K gaps
    //   ...
    //   solid(colors[N-1])               — remaining gaps
    let num_colors = colors.len();
    let total_gaps = 8u32; // depths 1→9

    // Build the colour sequence: [transition, solid, transition, solid, ...]
    // Each entry is (fill, is_transition).
    let mut segment_fills: Vec<(CxPrimitiveFill, &str)> = Vec::with_capacity(num_colors * 2);

    let make_transition = |a: u8, b: u8| match config.transition {
        BandTransition::Checker => CxPrimitiveFill::Checker { a, b },
        BandTransition::OrderedDither { threshold } => {
            CxPrimitiveFill::OrderedDither { a, b, threshold }
        }
    };

    // First transition: skybox → first ground colour.
    segment_fills.push((
        make_transition(config.skybox_color, colors[0]),
        "Transition",
    ));
    for i in 0..num_colors {
        segment_fills.push((CxPrimitiveFill::Solid(colors[i]), "Solid"));
        if i + 1 < num_colors {
            segment_fills.push((make_transition(colors[i], colors[i + 1]), "Transition"));
        }
    }

    // Each segment gets 1 depth gap, except the last solid which absorbs
    // all remaining gaps (giving the lightest colour more room at the
    // horizon).
    let num_segments = segment_fills.len();
    let mut gap_cursor: u32 = 0;
    let mut bands: Vec<Band> = Vec::new();

    for (seg_idx, (fill, label)) in segment_fills.iter().enumerate() {
        if gap_cursor >= total_gaps {
            break;
        }

        let is_last = seg_idx + 1 == num_segments;
        let seg_gap_count = if is_last { total_gaps - gap_cursor } else { 1 };

        let d_start = gap_cursor;
        let d_end = gap_cursor + seg_gap_count;
        gap_cursor = d_end;

        // Offset by 1 so gap 0 maps to depth 1→2, gap 7 to depth 8→9.
        let y_bottom = projection.floor_y_for_depth(d_start as i8 + 1).round() as i32;
        let y_top = projection.floor_y_for_depth(d_end as i8 + 1).round() as i32;

        bands.push(Band {
            y_bottom,
            y_top,
            fill: fill.clone(),
            label,
        });
    }

    // floor_y_for_depth values are screen-space coordinates matching the
    // perspective debug grid. Bands use CxRenderSpace::Camera so they align
    // with the grid lines regardless of camera position.
    let screen_center_x = (SCREEN_RESOLUTION.x / 2) as i32;

    for band in &bands {
        let height = (band.y_top - band.y_bottom).max(1) as u32;
        commands.spawn((
            Name::new(format!("PrimitiveBand - {}", band.label)),
            CxPrimitive {
                shape: CxPrimitiveShape::Rect {
                    size: UVec2::new(band_w, height),
                },
                fill: band.fill.clone(),
            },
            CxPosition::from(IVec2::new(screen_center_x, band.y_bottom)),
            CxAnchor::BottomCenter,
            CxRenderSpace::Camera,
            Layer::Background,
            StageEntity,
        ));
    }

    // Skybox: hard solid rect above the horizon.
    let horizon_y = projection.horizon_y.round() as i32;
    let sky_height = SCREEN_RESOLUTION.y * 2;
    commands.spawn((
        Name::new("PrimitiveBand - Skybox"),
        CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(band_w, sky_height),
            },
            fill: CxPrimitiveFill::Solid(config.skybox_color),
        },
        CxPosition::from(IVec2::new(screen_center_x, horizon_y)),
        CxAnchor::BottomCenter,
        CxRenderSpace::Camera,
        Layer::Background,
        StageEntity,
    ));
}
