use super::spawn::{spawn_destructible, spawn_enemy, spawn_object, spawn_pickup};
use crate::{
    components::VolumeSettings,
    game::GameProgressState,
    pixel::PxAssets,
    stage::{
        StagePlugin,
        bundles::{BackgroundBundle, SkyboxBundle},
        components::{Stage, StageEntity},
        data::{StageData, StageSpawn},
        messages::StageStartupEvent,
        player::messages::PlayerStartupEvent,
        projection::{effective_projection, validate_stage_projections},
        resources::StageGravity,
        ui::hud::spawn::spawn_hud,
    },
    systems::spawn::make_music_bundle,
    transitions::trigger_transition,
};
use activable::activate;
use bevy::{audio::PlaybackMode, prelude::*};
use carapace::prelude::{PxFilter, PxSprite, PxTypeface};

/// @trigger Builds the stage world: spawns HUD, enemies, pickups, background, and music.
///
/// When `from_checkpoint` is set, uses the stage's authored checkpoint
/// coordinates for the camera and skips initial gameplay spawns (enemies,
/// destructibles, pickups) that precede the checkpoint.
#[allow(clippy::too_many_arguments)]
pub fn on_stage_startup(
    trigger: On<StageStartupEvent>,
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameProgressState>>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
    mut typefaces: PxAssets<PxTypeface>,
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

    spawn_hud(
        &mut commands,
        &mut typefaces,
        &mut assets_sprite,
        &mut filters,
    );

    let initial_projection = effective_projection(data, 0);

    // Spawn default floor entities from the initial projection so the depth
    // debug overlay and falling physics have data immediately.  Steps with
    // explicit `floor_depths` will despawn and replace these.
    {
        use std::collections::HashMap;
        let default_floors: HashMap<crate::stage::components::placement::Depth, f32> = (1..=9_i8)
            .filter_map(|d| {
                crate::stage::components::placement::Depth::try_from(d)
                    .ok()
                    .map(|depth| (depth, initial_projection.floor_y_for_depth(d)))
            })
            .collect();
        crate::stage::components::placement::spawn_floor_depths(&mut commands, &default_floors);
    }

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
                    &initial_projection,
                );
            }
            StageSpawn::Pickup(spawn) => {
                spawn_pickup(&mut commands, &mut assets_sprite, Vec2::ZERO, spawn);
            }
        }
    }

    commands
        .spawn((Stage, Name::new("Stage"), Visibility::Visible))
        .with_children(|p0| {
            p0.spawn(BackgroundBundle::new(
                assets_sprite.load(data.background_path.clone()),
            ));
            p0.spawn(SkyboxBundle::new(&mut assets_sprite, data.skybox.clone()));
        });

    // TODO turn this into a spawn, like in cutscene, or make it a StageSpawn
    let (player, settings, system_bundle, music_tag) = make_music_bundle(
        &asset_server,
        &volume_settings,
        data.music_path.clone(),
        PlaybackMode::Loop,
    );

    commands.spawn((player, settings, system_bundle, music_tag, StageEntity));

    *next_game_state = NextState::PendingIfNeq(GameProgressState::Running);
    commands.trigger(PlayerStartupEvent);
}
