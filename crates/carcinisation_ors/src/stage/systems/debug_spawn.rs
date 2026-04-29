//! Debug-only enemy spawner for diagnostic stages.
//!
//! - `1` → Mosquiton (idle, despawns previous debug enemy)
//! - `2` → Tardigrade (idle, despawns previous debug enemy)
//! - `3` → Spidey (idle, despawns previous debug enemy)
//! - `Shift+1..9` → relocate the current debug enemy to that depth

use bevy::prelude::*;
use carapace::prelude::WorldPos;
use carcinisation_core::components::DespawnMark;

use crate::stage::{
    components::placement::Depth,
    data::EnemySpawn,
    depth_scale::DepthScaleConfig,
    enemy::data::steps::EnemyStep,
    floors::ActiveFloors,
    resources::{ActiveProjection, ProjectionView},
    systems::spawn::spawn_enemy,
};
use carcinisation_core::globals::SCREEN_RESOLUTION;

/// Marker for debug-spawned enemies so they can be despawned on type switch.
#[derive(Component)]
pub struct DebugSpawnedEnemy;

/// Spawns/replaces debug enemies on keypress. Shift+digit relocates depth.
#[allow(clippy::too_many_arguments)]
pub fn debug_keyboard_spawn_enemies(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    floors: Res<ActiveFloors>,
    depth_scale: Res<DepthScaleConfig>,
    projection: Res<ActiveProjection>,
    projection_view: Res<ProjectionView>,
    debug_enemy_query: Query<Entity, With<DebugSpawnedEnemy>>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    let center_x = SCREEN_RESOLUTION.x as f32 / 2.0;

    // Shift+digit → relocate existing debug enemy to that depth.
    if shift {
        let target_depth = digit_to_depth(&keys);
        if let Some(depth) = target_depth {
            let new_y = projection.0.floor_y_for_depth(depth.to_i8());
            for entity in &debug_enemy_query {
                commands
                    .entity(entity)
                    .insert((WorldPos::from(Vec2::new(center_x, new_y)), depth));

                info!("Debug relocate → depth {depth:?}, y={new_y:.1}");
            }
        }
        return;
    }

    // Digit without shift → spawn new enemy type (despawn previous).
    // ground_offset: distance from floor line to entity WorldPos.
    //   Origin-mode composed (Mosquiton): ground_anchor_y from atlas metadata
    //   BottomOrigin composed (Spidey): 0 (entity IS the ground contact)
    //   Simple sprite (Tardigrade): small offset for visual centering
    let (mut spawn, ground_offset) = if keys.just_pressed(KeyCode::Digit1) {
        (
            EnemySpawn::mosquiton_base()
                .with_steps_vec(vec![EnemyStep::idle_base().with_duration(99999.).into()]),
            49.0, // Origin mode: ground_anchor_y from atlas
        )
    } else if keys.just_pressed(KeyCode::Digit2) {
        (
            EnemySpawn::spidey_base(0.5, Vec2::ZERO)
                .with_steps_vec(vec![EnemyStep::idle_base().with_duration(99999.).into()]),
            0.0, // BottomOrigin: entity IS the ground contact
        )
    } else if keys.just_pressed(KeyCode::Digit3) {
        (
            EnemySpawn::tardigrade_base()
                .with_depth(Depth::Six) // tardigrade only has sprites for depths 6-8
                .with_steps_vec(vec![EnemyStep::idle_base().with_duration(99999.).into()]),
            10.0, // Simple sprite: approximate visual center offset
        )
    } else {
        return;
    };

    // Despawn any existing debug enemy.
    for entity in &debug_enemy_query {
        commands.entity(entity).insert(DespawnMark);
    }

    // floor_y_for_depth values match world Y when camera is at origin.
    // Add the per-type ground offset so the entity appears grounded.
    let floor_y = projection.0.floor_y_for_depth(spawn.depth.to_i8());
    spawn = spawn.with_coordinates(Vec2::new(center_x, floor_y + ground_offset));

    info!(
        "Debug spawn: {:?} at depth {:?}",
        spawn.enemy_type, spawn.depth
    );

    let entity = spawn_enemy(
        &mut commands,
        &asset_server,
        Vec2::ZERO,
        &spawn,
        &floors,
        &depth_scale,
        Some(&projection),
        Some(&projection_view),
        None,
    );

    commands.entity(entity).insert(DebugSpawnedEnemy);
}

fn digit_to_depth(keys: &ButtonInput<KeyCode>) -> Option<Depth> {
    if keys.just_pressed(KeyCode::Digit1) {
        Some(Depth::One)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(Depth::Two)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(Depth::Three)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(Depth::Four)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(Depth::Five)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(Depth::Six)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(Depth::Seven)
    } else if keys.just_pressed(KeyCode::Digit8) {
        Some(Depth::Eight)
    } else if keys.just_pressed(KeyCode::Digit9) {
        Some(Depth::Nine)
    } else {
        None
    }
}
