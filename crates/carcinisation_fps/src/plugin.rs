//! Bevy plugin that encapsulates the first-person raycaster systems.
//!
//! The plugin is generic over the game's layer type. The caller provides
//! a [`FpConfig`] that specifies which layer the FP view renders into
//! and the path to the map RON file.

use std::marker::PhantomData;

use bevy::{ecs::system::SystemParam, prelude::*};
use carapace::prelude::*;

/// System set for FP plugin systems. External input systems should run
/// `.before(FpSystems)` so the FP plugin reads updated state.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FpSystems;

use crate::{
    billboard::{
        Billboard, billboards_from_enemies_indexed, billboards_from_mosquitons,
        billboards_from_projectile_impacts, billboards_from_projectiles, make_death_sprite,
        make_enemy_sprite, make_pillar_sprite,
    },
    camera::FpCamera,
    collision::try_move,
    data::{FpEntityKind, FpMapData},
    enemy::{
        FpEnemy, FpProjectile, FpProjectileImpact, hitscan, hitscan_projectiles, tick_enemies,
        tick_projectile_impacts, tick_projectiles,
    },
    map::FpMap,
    mosquiton::{
        FpBloodShotBillboardSprites, FpMosquiton, FpMosquitonBillboardSprites, FpMosquitonConfig,
        hitscan_mosquitons, make_blood_shot_billboard_sprites, make_mosquiton_billboard_sprites,
        tick_mosquitons,
    },
    render::{FpPalette, draw_crosshair, draw_overlay_tint, render_fp_scene},
};

const QUICK_TURN_DURATION_SECS: f32 = 0.2;
const QUICK_TURN_RADIANS: f32 = std::f32::consts::PI;
const QUICK_TURN_GRACE_WINDOW_SECS: f32 = 0.08;
const DEATH_TURN_DURATION_SECS: f32 = 0.45;
const DEATH_RED_MAX_DENSITY: f32 = 0.85;

/// Configuration for the FP plugin.
#[derive(Resource, Clone)]
pub struct FpConfig {
    /// RON map file contents (pre-loaded string).
    pub map_ron: String,
    /// The layer value the FP sprite entity should render into.
    /// This is stored as a closure that produces the layer component.
    pub screen_width: u32,
    pub screen_height: u32,
    pub move_speed: f32,
    pub turn_speed: f32,
    pub hitscan_damage: u32,
    pub player_max_health: u32,
}

impl Default for FpConfig {
    fn default() -> Self {
        Self {
            map_ron: String::new(),
            screen_width: 160,
            screen_height: 144,
            move_speed: 2.0,
            turn_speed: 2.0,
            hitscan_damage: 15,
            player_max_health: 100,
        }
    }
}

// --- Resources ---

#[derive(Resource)]
pub struct FpSpriteHandle(pub Handle<CxSpriteAsset>);

#[derive(Resource)]
pub struct FpWallTextures(pub Vec<CxImage>);

#[derive(Resource)]
pub struct FpCameraRes(pub FpCamera);

#[derive(Resource)]
pub struct FpMapRes(pub FpMap);

#[derive(Resource)]
pub struct FpPaletteRes(pub FpPalette);

#[derive(Resource)]
pub struct FpStaticBillboards(pub Vec<Billboard>);

#[derive(Resource)]
pub struct FpEnemies(pub Vec<FpEnemy>);

#[derive(Resource)]
pub struct FpEnemySpriteIndices(pub Vec<usize>);

#[derive(Resource)]
pub struct FpSpritePairs(pub Vec<(CxImage, CxImage)>);

#[derive(Resource)]
pub struct FpProjectiles(pub Vec<FpProjectile>);

#[derive(Resource)]
pub struct FpProjectileImpacts(pub Vec<FpProjectileImpact>);

#[derive(Resource)]
pub struct FpBloodShotSprites(pub FpBloodShotBillboardSprites);

#[derive(Resource)]
pub struct FpMosquitons(pub Vec<FpMosquiton>);

#[derive(Resource)]
pub struct FpMosquitonSprites(pub FpMosquitonBillboardSprites);

#[derive(SystemParam)]
struct FpRenderSources<'w> {
    static_bbs: Res<'w, FpStaticBillboards>,
    enemies: Res<'w, FpEnemies>,
    indices: Res<'w, FpEnemySpriteIndices>,
    pairs: Res<'w, FpSpritePairs>,
    projectiles: Res<'w, FpProjectiles>,
    impacts: Res<'w, FpProjectileImpacts>,
    blood_shot_sprites: Res<'w, FpBloodShotSprites>,
    mosquitons: Res<'w, FpMosquitons>,
    mosquiton_sprites: Res<'w, FpMosquitonSprites>,
}

#[derive(Resource)]
pub struct FpPlayerHealth(pub u32);

#[derive(Resource)]
pub struct FpPlayerDead(pub bool);

/// Marker resource indicating FP mode is active.
#[derive(Resource)]
pub struct FpActive;

/// Resolved FP player intent. Integration layers can build this from any input source.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FpPlayerIntent {
    pub move_delta: Vec2,
    pub turn_delta: f32,
    pub shoot_pressed: bool,
    pub quick_turn_pressed: bool,
}

/// Debouncer for the Back+B quick-turn chord.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct FpQuickTurnDebounce {
    phase: FpQuickTurnChordPhase,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum FpQuickTurnChordPhase {
    #[default]
    Idle,
    GraceWindow {
        since: f32,
    },
    Consumed,
    BlockedUntilRelease,
}

/// Runtime state for a smooth 180-degree left quick-turn.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct FpQuickTurnState {
    remaining_radians: f32,
}

/// Runtime state for death camera facing and red fade.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct FpDeathViewState {
    active: bool,
    elapsed: f32,
    start_angle: f32,
    target_angle: f32,
}

/// Resolve Back+B/Down+Shift quick-turn as a near-simultaneous chord.
///
/// Mirrors ORS melee chord handling with a short grace window: slightly
/// imprecise presses count, but deliberate backpedal → strafe input does not.
#[must_use]
pub fn resolve_quick_turn_pressed(
    back_pressed: bool,
    b_pressed: bool,
    back_just_pressed: bool,
    b_just_pressed: bool,
    now_secs: f32,
    debounce: &mut FpQuickTurnDebounce,
) -> bool {
    let chord_pressed = back_pressed && b_pressed;
    if !back_pressed && !b_pressed {
        debounce.phase = FpQuickTurnChordPhase::Idle;
        return false;
    }

    match debounce.phase {
        FpQuickTurnChordPhase::Idle => {
            if back_just_pressed && b_just_pressed {
                debounce.phase = FpQuickTurnChordPhase::Consumed;
                true
            } else if back_just_pressed || b_just_pressed {
                debounce.phase = FpQuickTurnChordPhase::GraceWindow { since: now_secs };
                false
            } else {
                debounce.phase = FpQuickTurnChordPhase::BlockedUntilRelease;
                false
            }
        }
        FpQuickTurnChordPhase::GraceWindow { since } => {
            if chord_pressed && now_secs - since <= QUICK_TURN_GRACE_WINDOW_SECS {
                debounce.phase = FpQuickTurnChordPhase::Consumed;
                true
            } else if now_secs - since > QUICK_TURN_GRACE_WINDOW_SECS {
                debounce.phase = FpQuickTurnChordPhase::BlockedUntilRelease;
                false
            } else {
                false
            }
        }
        FpQuickTurnChordPhase::Consumed | FpQuickTurnChordPhase::BlockedUntilRelease => false,
    }
}

/// Start a smooth 180-degree quick-turn to the left.
pub fn request_quick_turn(state: &mut FpQuickTurnState) {
    if state.remaining_radians <= 0.0 {
        state.remaining_radians = QUICK_TURN_RADIANS;
    }
}

/// Advance the active quick-turn animation by `dt` seconds.
pub fn tick_quick_turn(camera: &mut FpCamera, state: &mut FpQuickTurnState, dt: f32) {
    if state.remaining_radians <= 0.0 {
        return;
    }

    let step = (QUICK_TURN_RADIANS / QUICK_TURN_DURATION_SECS * dt)
        .min(state.remaining_radians)
        .max(0.0);
    camera.angle = (camera.angle + step).rem_euclid(std::f32::consts::TAU);
    state.remaining_radians -= step;
}

/// Start the death view: rotate toward the source that killed the player.
pub fn request_death_view(state: &mut FpDeathViewState, camera: &FpCamera, killer_position: Vec2) {
    if state.active {
        return;
    }

    let to_killer = killer_position - camera.position;
    if to_killer.length_squared() <= f32::EPSILON {
        state.target_angle = camera.angle;
    } else {
        state.target_angle = to_killer
            .y
            .atan2(to_killer.x)
            .rem_euclid(std::f32::consts::TAU);
    }
    state.start_angle = camera.angle;
    state.elapsed = 0.0;
    state.active = true;
}

/// Advance the death camera turn and red-fade timer.
pub fn tick_death_view(camera: &mut FpCamera, state: &mut FpDeathViewState, dt: f32) {
    if !state.active {
        return;
    }

    state.elapsed = (state.elapsed + dt).min(DEATH_TURN_DURATION_SECS);
    let t = (state.elapsed / DEATH_TURN_DURATION_SECS).clamp(0.0, 1.0);
    let delta = signed_angle_delta(state.start_angle, state.target_angle);
    camera.angle = (state.start_angle + delta * t).rem_euclid(std::f32::consts::TAU);
}

#[must_use]
pub fn death_red_density(state: &FpDeathViewState) -> f32 {
    if !state.active {
        return 0.0;
    }
    let t = (state.elapsed / DEATH_TURN_DURATION_SECS).clamp(0.0, 1.0);
    (t * DEATH_RED_MAX_DENSITY).clamp(0.0, 1.0)
}

fn signed_angle_delta(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

// --- Plugin ---

/// First-person raycaster plugin.
///
/// Generic over `L: CxLayer` so it works with any game's layer enum.
/// Insert [`FpConfig`] before adding this plugin, or the setup system
/// will panic.
pub struct FpPlugin<L: CxLayer> {
    _l: PhantomData<L>,
}

impl<L: CxLayer> Default for FpPlugin<L> {
    fn default() -> Self {
        Self { _l: PhantomData }
    }
}

impl<L: CxLayer> FpPlugin<L> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L: CxLayer + Default> Plugin for FpPlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_fp::<L>);
        app.init_resource::<FpShootRequest>();
        app.init_resource::<FpQuickTurnState>();
        app.init_resource::<FpDeathViewState>();
        app.add_systems(
            Update,
            (
                apply_quick_turn_animation,
                tick_enemy_ai,
                apply_death_view,
                tick_projectile_impact_effects,
                handle_shooting,
                update_fp_view,
            )
                .chain()
                .in_set(FpSystems)
                .run_if(resource_exists::<FpActive>),
        );
    }
}

/// Setup system: parses the map from FpConfig, builds all resources.
///
/// Input handling is NOT included — the caller (binary or game plugin)
/// is responsible for reading input and updating `FpCameraRes`.
fn setup_fp<L: CxLayer + Default>(
    mut commands: Commands,
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    config: Res<FpConfig>,
) {
    let map_data = FpMapData::from_ron(&config.map_ron)
        .unwrap_or_else(|e| panic!("failed to parse FP map: {e}"));
    let map = map_data.to_map();
    let camera = map_data.to_camera();
    let palette = map_data.to_palette();
    let textures = map_data.build_wall_textures();

    let mut static_billboards = Vec::new();
    let mut enemies = Vec::new();
    let mut enemy_sprite_indices = Vec::new();
    let mut mosquitons = Vec::new();

    let procedural_alive = make_enemy_sprite(24, 2);
    let procedural_death = make_death_sprite(24, 1);
    let sprite_pairs: Vec<(CxImage, CxImage)> =
        vec![(procedural_alive.clone(), procedural_death.clone())];
    let mosquiton_sprites = make_mosquiton_billboard_sprites()
        .expect("embedded Mosquiton composed billboard assets should resolve");
    let blood_shot_sprites =
        make_blood_shot_billboard_sprites().expect("embedded blood shot assets should resolve");

    for spawn in &map_data.entities {
        let pos = Vec2::new(spawn.x, spawn.y);
        match &spawn.kind {
            FpEntityKind::Pillar {
                color,
                width,
                height,
            } => {
                static_billboards.push(Billboard {
                    position: pos,
                    height: 0.0,
                    world_height: 1.0,
                    sprite: make_pillar_sprite(*width, *height, *color),
                });
            }
            FpEntityKind::Enemy { health, speed, .. } => {
                enemies.push(FpEnemy::new(pos, *health, *speed));
                enemy_sprite_indices.push(0);
            }
            FpEntityKind::SpriteEnemy { health, speed, .. } => {
                enemies.push(FpEnemy::new(pos, *health, *speed));
                // TODO: asset-loaded sprites (requires async resolution).
                // For now, use procedural fallback.
                enemy_sprite_indices.push(0);
            }
            FpEntityKind::Mosquiton { health, speed } => {
                let config = FpMosquitonConfig {
                    health: *health,
                    move_speed: *speed,
                    ..Default::default()
                };
                mosquitons.push(FpMosquiton::new(pos, config));
            }
        }
    }

    // Render first frame.
    let enemy_bbs = billboards_from_enemies_indexed(&enemies, &enemy_sprite_indices, &sprite_pairs);
    let mosquiton_bbs = billboards_from_mosquitons(&mosquitons, &mosquiton_sprites);
    let all_bbs: Vec<Billboard> = static_billboards.iter().cloned().chain(enemy_bbs).collect();
    let all_bbs: Vec<Billboard> = all_bbs.into_iter().chain(mosquiton_bbs).collect();
    let mut image = CxImage::empty(UVec2::new(config.screen_width, config.screen_height));
    render_fp_scene(&mut image, &map, &camera, &textures, &palette, &all_bbs);
    draw_crosshair(&mut image, 4);
    let initial = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    let handle = sprite_assets.add(initial);

    // Spawn the FP view sprite entity.
    commands.spawn((
        CxSprite(handle.clone()),
        CxPosition(IVec2::ZERO),
        CxAnchor::BottomLeft,
        L::default(),
        CxRenderSpace::Camera,
        Visibility::Visible,
    ));

    commands.insert_resource(FpSpriteHandle(handle));
    commands.insert_resource(FpWallTextures(textures));
    commands.insert_resource(FpCameraRes(camera));
    commands.insert_resource(FpMapRes(map));
    commands.insert_resource(FpPaletteRes(palette));
    commands.insert_resource(FpStaticBillboards(static_billboards));
    commands.insert_resource(FpEnemies(enemies));
    commands.insert_resource(FpEnemySpriteIndices(enemy_sprite_indices));
    commands.insert_resource(FpSpritePairs(sprite_pairs));
    commands.insert_resource(FpProjectiles(Vec::new()));
    commands.insert_resource(FpProjectileImpacts(Vec::new()));
    commands.insert_resource(FpBloodShotSprites(blood_shot_sprites));
    commands.insert_resource(FpMosquitons(mosquitons));
    commands.insert_resource(FpMosquitonSprites(mosquiton_sprites));
    commands.insert_resource(FpPlayerHealth(config.player_max_health));
    commands.insert_resource(FpPlayerDead(false));
    commands.insert_resource(FpActive);

    info!("FP mode initialized");
}

/// Movement helper for external input systems. Call this with the computed
/// move delta to update the camera position with wall collision.
pub fn move_camera(camera: &mut FpCamera, delta: Vec2, map: &FpMap) {
    try_move(&mut camera.position, delta, 0.2, map);
}

fn apply_quick_turn_animation(
    time: Res<Time>,
    mut camera: ResMut<FpCameraRes>,
    mut quick_turn: ResMut<FpQuickTurnState>,
) {
    tick_quick_turn(&mut camera.0, &mut quick_turn, time.delta_secs());
}

fn apply_death_view(
    time: Res<Time>,
    mut camera: ResMut<FpCameraRes>,
    mut death_view: ResMut<FpDeathViewState>,
    dead: Res<FpPlayerDead>,
) {
    if dead.0 {
        tick_death_view(&mut camera.0, &mut death_view, time.delta_secs());
    }
}

fn tick_projectile_impact_effects(time: Res<Time>, mut impacts: ResMut<FpProjectileImpacts>) {
    tick_projectile_impacts(&mut impacts.0, time.delta_secs());
}

#[allow(clippy::too_many_arguments)]
fn tick_enemy_ai(
    time: Res<Time>,
    camera: Res<FpCameraRes>,
    map: Res<FpMapRes>,
    mut enemies: ResMut<FpEnemies>,
    mut mosquitons: ResMut<FpMosquitons>,
    mut projectiles: ResMut<FpProjectiles>,
    mut impacts: ResMut<FpProjectileImpacts>,
    mut health: ResMut<FpPlayerHealth>,
    mut dead: ResMut<FpPlayerDead>,
    mut death_view: ResMut<FpDeathViewState>,
) {
    let dt = time.delta_secs();

    if dead.0 {
        return;
    }

    let new_projs = tick_enemies(&mut enemies.0, camera.0.position, &map.0, dt);
    projectiles.0.extend(new_projs);
    let mosquiton_result = tick_mosquitons(&mut mosquitons.0, camera.0.position, &map.0, dt);
    projectiles.0.extend(mosquiton_result.projectiles);

    let projectile_result = tick_projectiles(&mut projectiles.0, camera.0.position, &map.0, dt);
    impacts.0.extend(projectile_result.impacts);

    apply_player_damage(
        &mut health.0,
        &mut dead.0,
        &mut death_view,
        &camera.0,
        projectile_result.player_damage,
        projectile_result.damage_source,
    );
    apply_player_damage(
        &mut health.0,
        &mut dead.0,
        &mut death_view,
        &camera.0,
        mosquiton_result.player_damage,
        mosquiton_result.damage_source,
    );
}

fn apply_player_damage(
    health: &mut u32,
    dead: &mut bool,
    death_view: &mut FpDeathViewState,
    camera: &FpCamera,
    damage: u32,
    damage_source: Option<Vec2>,
) {
    if *dead || damage == 0 {
        return;
    }

    *health = health.saturating_sub(damage);
    if *health == 0 {
        *dead = true;
        if let Some(source) = damage_source {
            request_death_view(death_view, camera, source);
        }
        info!("Player died!");
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_shooting(
    camera: Res<FpCameraRes>,
    map: Res<FpMapRes>,
    config: Res<FpConfig>,
    mut enemies: ResMut<FpEnemies>,
    mut mosquitons: ResMut<FpMosquitons>,
    mut projectiles: ResMut<FpProjectiles>,
    mut impacts: ResMut<FpProjectileImpacts>,
    dead: Res<FpPlayerDead>,
    mut shoot: ResMut<FpShootRequest>,
) {
    if dead.0 || !shoot.0 {
        return;
    }
    shoot.0 = false;

    let enemy_hit = hitscan(&camera.0, &enemies.0, &map.0);
    let mosquiton_hit = hitscan_mosquitons(&camera.0, &mosquitons.0, &map.0);
    let projectile_hit = hitscan_projectiles(&camera.0, &projectiles.0, &map.0);

    let mut hit = enemy_hit
        .enemy_idx
        .map(|enemy_idx| (FpShotHit::Enemy(enemy_idx), enemy_hit.distance));
    if let Some((mosquiton_idx, distance)) = mosquiton_hit
        && hit.is_none_or(|(_, current_distance)| distance < current_distance)
    {
        hit = Some((FpShotHit::Mosquiton(mosquiton_idx), distance));
    }
    if let Some((projectile_idx, distance)) = projectile_hit
        && hit.is_none_or(|(_, current_distance)| distance < current_distance)
    {
        hit = Some((FpShotHit::Projectile(projectile_idx), distance));
    }

    match hit {
        Some((FpShotHit::Enemy(enemy_idx), distance)) => {
            enemies.0[enemy_idx].take_damage(config.hitscan_damage);
            info!(
                "Hit enemy {enemy_idx} at distance {distance:.1} — health: {}",
                enemies.0[enemy_idx].health
            );
        }
        Some((FpShotHit::Mosquiton(mosquiton_idx), distance)) => {
            mosquitons.0[mosquiton_idx].take_damage(config.hitscan_damage);
            info!(
                "Hit mosquiton {mosquiton_idx} at distance {distance:.1} — health: {}",
                mosquitons.0[mosquiton_idx].health
            );
        }
        Some((FpShotHit::Projectile(projectile_idx), _)) => {
            if let Some(projectile) = projectiles.0.get_mut(projectile_idx) {
                projectile.alive = false;
                impacts
                    .0
                    .push(FpProjectileImpact::destroy(projectile.position));
            }
            projectiles.0.retain(|projectile| projectile.alive);
        }
        None => {}
    }
}

#[derive(Clone, Copy, Debug)]
enum FpShotHit {
    Enemy(usize),
    Mosquiton(usize),
    Projectile(usize),
}

#[allow(clippy::too_many_arguments)]
fn update_fp_view(
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    handle: Res<FpSpriteHandle>,
    textures: Res<FpWallTextures>,
    camera: Res<FpCameraRes>,
    map: Res<FpMapRes>,
    palette: Res<FpPaletteRes>,
    config: Res<FpConfig>,
    sources: FpRenderSources,
    health: Res<FpPlayerHealth>,
    dead: Res<FpPlayerDead>,
    death_view: Res<FpDeathViewState>,
) {
    let mut image = CxImage::empty(UVec2::new(config.screen_width, config.screen_height));

    let enemy_bbs =
        billboards_from_enemies_indexed(&sources.enemies.0, &sources.indices.0, &sources.pairs.0);
    let proj_bbs =
        billboards_from_projectiles(&sources.projectiles.0, &sources.blood_shot_sprites.0.hover);
    let impact_bbs =
        billboards_from_projectile_impacts(&sources.impacts.0, &sources.blood_shot_sprites.0);
    let mosquiton_bbs =
        billboards_from_mosquitons(&sources.mosquitons.0, &sources.mosquiton_sprites.0);
    let all_bbs: Vec<Billboard> = sources
        .static_bbs
        .0
        .iter()
        .cloned()
        .chain(enemy_bbs)
        .chain(mosquiton_bbs)
        .chain(impact_bbs)
        .chain(proj_bbs)
        .collect();

    render_fp_scene(
        &mut image,
        &map.0,
        &camera.0,
        &textures.0,
        &palette.0,
        &all_bbs,
    );

    if dead.0 {
        draw_overlay_tint(&mut image, 2, death_red_density(&death_view));
    } else {
        draw_crosshair(&mut image, 4);
    }

    // Health bar at top-left.
    let bar_w = 20;
    let filled = (health.0 as i32 * bar_w / config.player_max_health as i32).max(0);
    {
        let data = image.data_mut();
        let w = config.screen_width as i32;
        for x in 1..=bar_w {
            let color = if x <= filled { 2 } else { 1 };
            data[(w + x) as usize] = color;
            data[(2 * w + x) as usize] = color;
        }
    }

    if let Some(asset) = sprite_assets.get_mut(&handle.0) {
        *asset = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    }
}

/// Set to `true` from your input system to trigger a hitscan shot.
/// The plugin resets it to `false` after processing.
#[derive(Resource, Default)]
pub struct FpShootRequest(pub bool);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_turn_allows_simultaneous_press_and_debounces_hold() {
        let mut debounce = FpQuickTurnDebounce::default();
        assert!(resolve_quick_turn_pressed(
            true,
            true,
            true,
            true,
            0.0,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            true,
            true,
            false,
            false,
            0.01,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            false,
            true,
            false,
            false,
            0.02,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            true,
            true,
            true,
            false,
            0.03,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            false,
            false,
            false,
            false,
            0.04,
            &mut debounce
        ));
        assert!(resolve_quick_turn_pressed(
            true,
            true,
            true,
            true,
            0.05,
            &mut debounce
        ));
    }

    #[test]
    fn quick_turn_allows_staggered_press_inside_grace() {
        let mut debounce = FpQuickTurnDebounce::default();
        assert!(!resolve_quick_turn_pressed(
            true,
            false,
            true,
            false,
            1.0,
            &mut debounce
        ));
        assert!(resolve_quick_turn_pressed(
            true,
            true,
            false,
            true,
            1.04,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            true,
            true,
            false,
            false,
            1.05,
            &mut debounce
        ));
    }

    #[test]
    fn quick_turn_blocks_staggered_back_then_shift_after_grace_until_release() {
        let mut debounce = FpQuickTurnDebounce::default();
        assert!(!resolve_quick_turn_pressed(
            true,
            false,
            true,
            false,
            2.0,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            true,
            false,
            false,
            false,
            2.09,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            true,
            true,
            false,
            true,
            2.1,
            &mut debounce
        ));
        assert!(!resolve_quick_turn_pressed(
            false,
            false,
            false,
            false,
            2.2,
            &mut debounce
        ));
        assert!(resolve_quick_turn_pressed(
            true,
            true,
            true,
            true,
            2.3,
            &mut debounce
        ));
    }

    #[test]
    fn quick_turn_animates_left_over_point_two_seconds() {
        let mut camera = FpCamera {
            angle: 0.25,
            ..Default::default()
        };
        let mut quick_turn = FpQuickTurnState::default();
        request_quick_turn(&mut quick_turn);

        tick_quick_turn(&mut camera, &mut quick_turn, 0.1);
        assert!((camera.angle - (0.25 + std::f32::consts::FRAC_PI_2)).abs() < 1e-5);
        assert!(quick_turn.remaining_radians > 0.0);

        tick_quick_turn(&mut camera, &mut quick_turn, 0.1);
        assert!((camera.angle - (0.25 + std::f32::consts::PI)).abs() < 1e-5);
        assert!(quick_turn.remaining_radians <= 1e-5);
    }

    #[test]
    fn quick_turn_request_ignored_while_active() {
        let mut quick_turn = FpQuickTurnState::default();
        request_quick_turn(&mut quick_turn);
        tick_quick_turn(&mut FpCamera::default(), &mut quick_turn, 0.05);
        let remaining = quick_turn.remaining_radians;
        request_quick_turn(&mut quick_turn);
        assert_eq!(quick_turn.remaining_radians, remaining);
    }

    #[test]
    fn death_view_turns_toward_killer_and_red_increases() {
        let mut camera = FpCamera {
            position: Vec2::ZERO,
            angle: 0.0,
            ..Default::default()
        };
        let mut death_view = FpDeathViewState::default();

        request_death_view(&mut death_view, &camera, Vec2::Y);
        tick_death_view(&mut camera, &mut death_view, DEATH_TURN_DURATION_SECS * 0.5);
        assert!((camera.angle - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        let half_density = death_red_density(&death_view);
        assert!(half_density > 0.0);
        assert!(half_density < DEATH_RED_MAX_DENSITY);

        tick_death_view(&mut camera, &mut death_view, DEATH_TURN_DURATION_SECS * 0.5);
        assert!((camera.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!((death_red_density(&death_view) - DEATH_RED_MAX_DENSITY).abs() < 1e-5);
    }

    #[test]
    fn death_view_uses_shortest_turn_direction() {
        let mut camera = FpCamera {
            position: Vec2::ZERO,
            angle: 350.0_f32.to_radians(),
            ..Default::default()
        };
        let mut death_view = FpDeathViewState::default();
        let ten_degrees = Vec2::new(10.0_f32.to_radians().cos(), 10.0_f32.to_radians().sin());

        request_death_view(&mut death_view, &camera, ten_degrees);
        tick_death_view(&mut camera, &mut death_view, DEATH_TURN_DURATION_SECS);
        assert!((camera.angle - 10.0_f32.to_radians()).abs() < 1e-5);

        camera.angle = 10.0_f32.to_radians();
        let mut death_view = FpDeathViewState::default();
        let three_fifty_degrees =
            Vec2::new(350.0_f32.to_radians().cos(), 350.0_f32.to_radians().sin());
        request_death_view(&mut death_view, &camera, three_fifty_degrees);
        tick_death_view(&mut camera, &mut death_view, DEATH_TURN_DURATION_SECS);
        assert!((camera.angle - 350.0_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn player_damage_latches_first_killing_source() {
        let camera = FpCamera {
            position: Vec2::ZERO,
            angle: 0.0,
            ..Default::default()
        };
        let mut health = 10;
        let mut dead = false;
        let mut death_view = FpDeathViewState::default();

        apply_player_damage(
            &mut health,
            &mut dead,
            &mut death_view,
            &camera,
            10,
            Some(Vec2::Y),
        );
        let first_target = death_view.target_angle;
        apply_player_damage(
            &mut health,
            &mut dead,
            &mut death_view,
            &camera,
            10,
            Some(Vec2::NEG_Y),
        );

        assert!(dead);
        assert_eq!(health, 0);
        assert!((first_target - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(death_view.target_angle, first_target);
    }
}
