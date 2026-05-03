//! Bevy plugin that encapsulates the first-person raycaster systems.
//!
//! The plugin is generic over the game's layer type. The caller provides
//! a [`Config`] that specifies which layer the First-person view renders into
//! and the path to the map RON file.

use std::marker::PhantomData;

use bevy::{ecs::system::SystemParam, prelude::*};
use carapace::prelude::*;
use carcinisation_base::fire_death::{DamageKind, perimeter_flames_from_mask};

/// System set for First-person plugin systems. External input systems should run
/// `.before(Systems)` so the First-person plugin reads updated state.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Systems;

use crate::{
    billboard::{
        Billboard, billboard_from_enemy, billboard_from_mosquiton, billboards_from_enemies_indexed,
        billboards_from_mosquitons, billboards_from_projectile_impacts,
        billboards_from_projectiles, make_death_sprite, make_enemy_sprite, make_pillar_sprite,
    },
    camera::Camera,
    collision::try_move,
    data::{EntityKind, MapData},
    enemy::{
        Enemy, EnemyState, Projectile, ProjectileImpact, ProjectileTickResult,
        tick_projectile_impacts, tick_projectiles, tick_single_enemy,
    },
    map::Map,
    mosquiton::{
        BloodShotBillboardSprites, Mosquiton, MosquitonBillboardSprites, MosquitonConfig,
        MosquitonState, make_blood_shot_billboard_sprites, make_mosquiton_billboard_sprites,
        tick_single_mosquiton,
    },
    player_attack::{
        AttackInput, AttackLoadout, FlamethrowerConfig, PlayerAttackSprites, PlayerAttackState,
        destroy_projectiles_touching_active_flamethrower, draw_player_attack_overlays,
        flame_wall_mask, process_player_attacks, wall_impact_sprite,
    },
    render::{
        CharDecal, FpWallRenderEffects, Palette, draw_crosshair, draw_overlay_tint,
        render_fp_scene, render_fp_scene_with_effects,
    },
    sky::Sky,
};

const QUICK_TURN_DURATION_SECS: f32 = 0.4;
const QUICK_TURN_RADIANS: f32 = std::f32::consts::PI;
const SIDE_TURN_DURATION_SECS: f32 = 0.4;
const SIDE_TURN_RADIANS: f32 = std::f32::consts::FRAC_PI_2;
const QUICK_TURN_GRACE_WINDOW_SECS: f32 = 0.08;
const DEATH_TURN_DURATION_SECS: f32 = 0.45;
const DEATH_RED_MAX_DENSITY: f32 = 0.85;
const CAMERA_SHAKE_BASE_INTENSITY: f32 = 3.0;
const CAMERA_SHAKE_DECAY_RATE: f32 = 12.0;
const CAMERA_SHAKE_THRESHOLD: f32 = 0.3;

/// Configuration for the First-person plugin.
#[derive(Resource, Clone)]
pub struct Config {
    /// RON map file contents (pre-loaded string).
    pub map_ron: String,
    /// Path to the sky RON config file (used to resolve .pxi asset paths).
    pub sky_path: String,
    /// The layer value the FP sprite entity should render into.
    /// This is stored as a closure that produces the layer component.
    pub screen_width: u32,
    pub screen_height: u32,
    pub move_speed: f32,
    pub turn_speed: f32,
    pub hitscan_damage: u32,
    pub player_max_health: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            map_ron: String::new(),
            sky_path: String::new(),
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
pub struct SpriteHandle(pub Handle<CxSpriteAsset>);

#[derive(Resource)]
pub struct WallTextures(pub Vec<CxImage>);

#[derive(Resource)]
pub struct CameraRes(pub Camera);

#[derive(Resource)]
pub struct MapRes(pub Map);

#[derive(Resource)]
pub struct PaletteRes(pub Palette);

#[derive(Resource)]
pub struct StaticBillboards(pub Vec<Billboard>);

/// Sprite index paired with each enemy entity for billboard resolution.
#[derive(Component)]
pub struct EnemySpriteIndex(pub usize);

#[derive(Resource)]
pub struct SpritePairs(pub Vec<(CxImage, CxImage)>);

#[derive(Resource)]
pub struct Projectiles(pub Vec<Projectile>);

#[derive(Resource)]
pub struct ProjectileImpacts(pub Vec<ProjectileImpact>);

#[derive(Resource, Default)]
pub struct CharDecals(pub Vec<CharDecal>);

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct BurningCorpseContactHazardState {
    cooldown_remaining_secs: f32,
}

#[derive(Resource)]
pub struct BloodShotSprites(pub BloodShotBillboardSprites);

#[derive(Resource)]
pub struct MosquitonSprites(pub MosquitonBillboardSprites);

#[derive(SystemParam)]
struct RenderSources<'w> {
    static_bbs: Res<'w, StaticBillboards>,
    pairs: Res<'w, SpritePairs>,
    projectiles: Res<'w, Projectiles>,
    impacts: Res<'w, ProjectileImpacts>,
    blood_shot_sprites: Res<'w, BloodShotSprites>,
    mosquiton_sprites: Res<'w, MosquitonSprites>,
    char_decals: Res<'w, CharDecals>,
}

#[derive(SystemParam)]
struct ViewResources<'w> {
    textures: Res<'w, WallTextures>,
    camera: Res<'w, CameraRes>,
    map: Res<'w, MapRes>,
    palette: Res<'w, PaletteRes>,
    sky: Res<'w, Sky>,
    config: Res<'w, Config>,
    health: Res<'w, PlayerHealth>,
    dead: Res<'w, PlayerDead>,
    death_view: Res<'w, DeathViewState>,
    camera_shake: Res<'w, CameraShakeState>,
    attack_sprites: Res<'w, PlayerAttackSprites>,
    attack_loadout: Res<'w, AttackLoadout>,
    attack_state: Res<'w, PlayerAttackState>,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerHealth(pub u32);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerDead(pub bool);

/// Marker resource indicating First-person mode is active.
#[derive(Resource)]
pub struct Active;

/// Resolved FP player intent. Integration layers can build this from any input source.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerIntent {
    pub move_delta: Vec2,
    pub turn_delta: f32,
    pub shoot_pressed: bool,
    pub quick_turn_pressed: bool,
}

/// Debouncer for simultaneous-press chords that fire on arrow release.
///
/// Flow: both keys pressed within grace window → Armed → direction key
/// released → fires. The modifier key (B/shift) can stay held.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChordDebounce {
    phase: ChordPhase,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum ChordPhase {
    #[default]
    Idle,
    /// One key pressed, waiting for the other within the grace window.
    GraceWindow {
        since: f32,
    },
    /// Both keys were pressed in time; waiting for direction key release to fire.
    Armed,
    /// Chord was consumed or missed; wait for full release before re-arming.
    BlockedUntilRelease,
}

/// Debouncer for the Back+B quick-turn chord.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct QuickTurnDebounce(pub ChordDebounce);

/// Debouncer for the B+Left side-turn chord.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SideTurnLeftDebounce(pub ChordDebounce);

/// Debouncer for the B+Right side-turn chord.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SideTurnRightDebounce(pub ChordDebounce);

/// Runtime state for a smooth 180-degree left quick-turn.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct QuickTurnState {
    remaining_radians: f32,
    /// Radians per second for the active turn.
    speed: f32,
    /// +1.0 for left, -1.0 for right.
    direction: f32,
}

/// Runtime state for death camera facing and red fade.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct DeathViewState {
    active: bool,
    elapsed: f32,
    start_angle: f32,
    target_angle: f32,
}

/// Runtime state for FP hit camera shake.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CameraShakeState {
    intensity: f32,
    current_offset: IVec2,
}

/// Resolve a two-button chord that fires on release of the direction key.
///
/// `mod_pressed`/`dir_pressed` = held state this frame.
/// `mod_just_pressed`/`dir_just_pressed` = pressed this frame.
/// `dir_just_released` = direction key released this frame (fires the chord).
///
/// The modifier key (B/shift) can stay held after the chord fires.
#[must_use]
pub fn resolve_chord_released(
    mod_pressed: bool,
    dir_pressed: bool,
    mod_just_pressed: bool,
    dir_just_pressed: bool,
    dir_just_released: bool,
    now_secs: f32,
    debounce: &mut ChordDebounce,
) -> bool {
    // Full release of both keys resets to idle.
    if !mod_pressed && !dir_pressed && !dir_just_released {
        debounce.phase = ChordPhase::Idle;
        return false;
    }

    match debounce.phase {
        ChordPhase::Idle => {
            if mod_pressed && dir_pressed && (mod_just_pressed && dir_just_pressed) {
                debounce.phase = ChordPhase::Armed;
            } else if mod_just_pressed || dir_just_pressed {
                debounce.phase = ChordPhase::GraceWindow { since: now_secs };
            } else if mod_pressed || dir_pressed {
                debounce.phase = ChordPhase::BlockedUntilRelease;
            }
            false
        }
        ChordPhase::GraceWindow { since } => {
            if mod_pressed && dir_pressed && now_secs - since <= QUICK_TURN_GRACE_WINDOW_SECS {
                debounce.phase = ChordPhase::Armed;
                false
            } else if now_secs - since > QUICK_TURN_GRACE_WINDOW_SECS {
                debounce.phase = ChordPhase::BlockedUntilRelease;
                false
            } else {
                false
            }
        }
        ChordPhase::Armed => {
            if dir_just_released {
                debounce.phase = ChordPhase::BlockedUntilRelease;
                true
            } else {
                false
            }
        }
        ChordPhase::BlockedUntilRelease => {
            if !mod_pressed && !dir_pressed {
                debounce.phase = ChordPhase::Idle;
            }
            false
        }
    }
}

/// Convenience wrapper: resolve Back+B quick-turn chord (fires on Back release).
#[must_use]
pub fn resolve_quick_turn_pressed(
    back_pressed: bool,
    b_pressed: bool,
    back_just_pressed: bool,
    b_just_pressed: bool,
    back_just_released: bool,
    now_secs: f32,
    debounce: &mut QuickTurnDebounce,
) -> bool {
    resolve_chord_released(
        b_pressed,
        back_pressed,
        b_just_pressed,
        back_just_pressed,
        back_just_released,
        now_secs,
        &mut debounce.0,
    )
}

/// Convenience wrapper: resolve B+direction side-turn chord (fires on direction release).
#[must_use]
pub fn resolve_side_turn_pressed(
    b_pressed: bool,
    dir_pressed: bool,
    b_just_pressed: bool,
    dir_just_pressed: bool,
    dir_just_released: bool,
    now_secs: f32,
    debounce: &mut ChordDebounce,
) -> bool {
    resolve_chord_released(
        b_pressed,
        dir_pressed,
        b_just_pressed,
        dir_just_pressed,
        dir_just_released,
        now_secs,
        debounce,
    )
}

/// Start a smooth 180-degree quick-turn to the left.
pub fn request_quick_turn(state: &mut QuickTurnState) {
    if state.remaining_radians <= 0.0 {
        state.remaining_radians = QUICK_TURN_RADIANS;
        state.speed = QUICK_TURN_RADIANS / QUICK_TURN_DURATION_SECS;
        state.direction = 1.0;
    }
}

/// Start a smooth 90-degree side turn. `left` = true turns left, false turns right.
pub fn request_side_turn(state: &mut QuickTurnState, left: bool) {
    if state.remaining_radians <= 0.0 {
        state.remaining_radians = SIDE_TURN_RADIANS;
        state.speed = SIDE_TURN_RADIANS / SIDE_TURN_DURATION_SECS;
        state.direction = if left { 1.0 } else { -1.0 };
    }
}

/// Advance the active quick-turn animation by `dt` seconds.
pub fn tick_quick_turn(camera: &mut Camera, state: &mut QuickTurnState, dt: f32) {
    if state.remaining_radians <= 0.0 {
        return;
    }

    let step = (state.speed * dt)
        .min(state.remaining_radians)
        .max(0.0);
    camera.angle = (camera.angle + step * state.direction).rem_euclid(std::f32::consts::TAU);
    state.remaining_radians -= step;
}

/// Start the death view: rotate toward the source that killed the player.
pub fn request_death_view(state: &mut DeathViewState, camera: &Camera, killer_position: Vec2) {
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
pub fn tick_death_view(camera: &mut Camera, state: &mut DeathViewState, dt: f32) {
    if !state.active {
        return;
    }

    state.elapsed = (state.elapsed + dt).min(DEATH_TURN_DURATION_SECS);
    let t = (state.elapsed / DEATH_TURN_DURATION_SECS).clamp(0.0, 1.0);
    let delta = signed_angle_delta(state.start_angle, state.target_angle);
    camera.angle = (state.start_angle + delta * t).rem_euclid(std::f32::consts::TAU);
}

#[must_use]
pub fn death_red_density(state: &DeathViewState) -> f32 {
    if !state.active {
        return 0.0;
    }
    let t = (state.elapsed / DEATH_TURN_DURATION_SECS).clamp(0.0, 1.0);
    (t * DEATH_RED_MAX_DENSITY).clamp(0.0, 1.0)
}

fn signed_angle_delta(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Reinforce an FP camera shake, matching ORS' additive hit shake model.
pub fn request_camera_shake(state: &mut CameraShakeState) {
    state.intensity += CAMERA_SHAKE_BASE_INTENSITY;
}

/// Advance FP camera shake with caller-provided random samples.
///
/// `angle_sample` and `magnitude_sample` are expected in `0.0..=1.0`.
/// They are parameters so the behavior can be tested deterministically while
/// the Bevy system uses real randomness.
pub fn tick_camera_shake(
    state: &mut CameraShakeState,
    dt: f32,
    angle_sample: f32,
    magnitude_sample: f32,
) {
    if state.intensity < CAMERA_SHAKE_THRESHOLD {
        state.intensity = 0.0;
        state.current_offset = IVec2::ZERO;
        return;
    }

    let angle = angle_sample.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let magnitude = state.intensity * (0.5 + 0.5 * magnitude_sample.clamp(0.0, 1.0));
    let offset = Vec2::new(angle.cos() * magnitude, angle.sin() * magnitude).round();
    state.current_offset = IVec2::new(offset.x as i32, offset.y as i32);
    state.intensity *= (-CAMERA_SHAKE_DECAY_RATE * dt).exp();
}

fn apply_framebuffer_offset(image: &mut CxImage, offset: IVec2) {
    if offset == IVec2::ZERO {
        return;
    }

    let width = image.width() as i32;
    let height = image.height() as i32;
    let source = image.data().to_vec();
    let target = image.data_mut();

    for y in 0..height {
        for x in 0..width {
            let src_x = (x - offset.x).clamp(0, width - 1);
            let src_y = (y - offset.y).clamp(0, height - 1);
            target[(y * width + x) as usize] = source[(src_y * width + src_x) as usize];
        }
    }
}

// --- Plugin ---

/// First-person raycaster plugin.
///
/// Generic over `L: CxLayer` so it works with any game's layer enum.
/// Insert [`Config`] before adding this plugin, or the setup system
/// will panic.
pub struct FpsPlugin<L: CxLayer> {
    _l: PhantomData<L>,
}

impl<L: CxLayer> Default for FpsPlugin<L> {
    fn default() -> Self {
        Self { _l: PhantomData }
    }
}

impl<L: CxLayer> FpsPlugin<L> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L: CxLayer + Default> bevy::prelude::Plugin for FpsPlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_fp::<L>);
        app.register_type::<AttackInput>();
        app.register_type::<AttackLoadout>();
        app.register_type::<PlayerHealth>();
        app.register_type::<PlayerDead>();
        app.init_resource::<ShootRequest>();
        app.init_resource::<AttackInput>();
        app.init_resource::<AttackLoadout>();
        app.init_resource::<PlayerAttackState>();
        app.init_resource::<CharDecals>();
        app.init_resource::<BurningCorpseContactHazardState>();
        app.init_resource::<QuickTurnState>();
        app.init_resource::<SideTurnLeftDebounce>();
        app.init_resource::<SideTurnRightDebounce>();
        app.init_resource::<DeathViewState>();
        app.init_resource::<CameraShakeState>();
        app.add_systems(
            Update,
            (
                apply_quick_turn_animation.in_set(Systems),
                handle_shooting.in_set(Systems),
                tick_enemy_ai.in_set(Systems).after(handle_shooting),
                apply_death_view.in_set(Systems),
                tick_projectile_impact_effects.in_set(Systems),
                tick_camera_shake_effect.in_set(Systems),
                update_fp_view.in_set(Systems).after(tick_enemy_ai),
            )
                .run_if(resource_exists::<Active>),
        );
    }
}

/// Setup system: parses the map from `Config`, builds all resources.
///
/// Input handling is NOT included — the caller (binary or game plugin)
/// is responsible for reading input and updating `CameraRes`.
fn setup_fp<L: CxLayer + Default>(
    mut commands: Commands,
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    config: Res<Config>,
) {
    let map_data = MapData::from_ron(&config.map_ron)
        .unwrap_or_else(|e| panic!("failed to parse FP map: {e}"));
    let map = map_data.to_map();
    let camera = map_data.to_camera();
    let palette = map_data.to_palette();
    let textures = map_data.build_wall_textures();

    let mut static_billboards = Vec::new();

    let procedural_alive = make_enemy_sprite(24, 2);
    let procedural_death = make_death_sprite(24, 1);
    let sprite_pairs: Vec<(CxImage, CxImage)> =
        vec![(procedural_alive.clone(), procedural_death.clone())];
    let mosquiton_sprites = make_mosquiton_billboard_sprites()
        .expect("embedded Mosquiton composed billboard assets should resolve");
    let blood_shot_sprites =
        make_blood_shot_billboard_sprites().expect("embedded blood shot assets should resolve");

    // Temporary Vecs for first-frame render before entities exist.
    let mut enemy_bbs = Vec::new();
    let mut mosquiton_bbs = Vec::new();

    for spawn in &map_data.entities {
        let pos = Vec2::new(spawn.x, spawn.y);
        match &spawn.kind {
            EntityKind::Pillar {
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
            EntityKind::Enemy { health, speed, .. }
            | EntityKind::SpriteEnemy { health, speed, .. } => {
                let enemy = Enemy::new(pos, *health, *speed);
                enemy_bbs.push(billboard_from_enemy(&enemy, 0, &sprite_pairs));
                commands.spawn((enemy, EnemySpriteIndex(0)));
            }
            EntityKind::Mosquiton { health, speed } => {
                let config = MosquitonConfig {
                    health: *health,
                    move_speed: *speed,
                    ..Default::default()
                };
                let mosquiton = Mosquiton::new(pos, config);
                mosquiton_bbs.push(billboard_from_mosquiton(&mosquiton, &mosquiton_sprites));
                commands.spawn(mosquiton);
            }
        }
    }

    let all_bbs: Vec<Billboard> = static_billboards
        .iter()
        .cloned()
        .chain(enemy_bbs)
        .chain(mosquiton_bbs)
        .collect();
    let sky_ron = std::fs::read_to_string(&config.sky_path)
        .unwrap_or_else(|e| panic!("failed to read sky RON {}: {}", config.sky_path, e));
    let workspace_root = std::env::current_dir()
        .unwrap_or_else(|e| panic!("failed to get current dir: {}", e))
        .to_string_lossy()
        .to_string();
    let sky = Sky::from_ron(&sky_ron, &workspace_root);
    let mut image = CxImage::empty(UVec2::new(config.screen_width, config.screen_height));
    render_fp_scene(
        &mut image,
        &map,
        &camera,
        &textures,
        &palette,
        &all_bbs,
        Some(&sky),
    );
    draw_crosshair(&mut image, 4);
    let initial = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    let handle = sprite_assets.add(initial);

    // Spawn the First-person view sprite entity.
    commands.spawn((
        CxSprite(handle.clone()),
        CxPosition(IVec2::ZERO),
        CxAnchor::BottomLeft,
        L::default(),
        CxRenderSpace::Camera,
        Visibility::Visible,
    ));

    commands.insert_resource(SpriteHandle(handle));
    commands.insert_resource(WallTextures(textures));
    commands.insert_resource(CameraRes(camera));
    commands.insert_resource(MapRes(map));
    commands.insert_resource(PaletteRes(palette));
    commands.insert_resource(sky);
    commands.insert_resource(StaticBillboards(static_billboards));
    commands.insert_resource(SpritePairs(sprite_pairs));
    commands.insert_resource(Projectiles(Vec::new()));
    commands.insert_resource(ProjectileImpacts(Vec::new()));
    commands.insert_resource(CharDecals::default());
    commands.insert_resource(BloodShotSprites(blood_shot_sprites));
    commands.insert_resource(MosquitonSprites(mosquiton_sprites));
    commands.insert_resource(PlayerAttackSprites::load());
    commands.insert_resource(PlayerHealth(config.player_max_health));
    commands.insert_resource(PlayerDead(false));
    commands.insert_resource(Active);

    info!("First-person mode initialized");
}

/// Movement helper for external input systems. Call this with the computed
/// move delta to update the camera position with wall collision.
pub fn move_camera(camera: &mut Camera, delta: Vec2, map: &Map) {
    try_move(&mut camera.position, delta, 0.2, map);
}

fn apply_quick_turn_animation(
    time: Res<Time>,
    mut camera: ResMut<CameraRes>,
    mut quick_turn: ResMut<QuickTurnState>,
) {
    tick_quick_turn(&mut camera.0, &mut quick_turn, time.delta_secs());
}

fn apply_death_view(
    time: Res<Time>,
    mut camera: ResMut<CameraRes>,
    mut death_view: ResMut<DeathViewState>,
    dead: Res<PlayerDead>,
) {
    if dead.0 {
        tick_death_view(&mut camera.0, &mut death_view, time.delta_secs());
    }
}

fn tick_projectile_impact_effects(time: Res<Time>, mut impacts: ResMut<ProjectileImpacts>) {
    tick_projectile_impacts(&mut impacts.0, time.delta_secs());
}

fn tick_camera_shake_effect(time: Res<Time>, mut shake: ResMut<CameraShakeState>) {
    tick_camera_shake(
        &mut shake,
        time.delta_secs(),
        rand::random::<f32>(),
        rand::random::<f32>(),
    );
}

#[allow(clippy::too_many_arguments)]
fn tick_enemy_ai(
    time: Res<Time>,
    camera: Res<CameraRes>,
    map: Res<MapRes>,
    mut enemy_q: Query<(Entity, &mut Enemy)>,
    mut mosquiton_q: Query<(Entity, &mut Mosquiton)>,
    mut projectiles: ResMut<Projectiles>,
    mut impacts: ResMut<ProjectileImpacts>,
    mut health: ResMut<PlayerHealth>,
    mut dead: ResMut<PlayerDead>,
    mut death_view: ResMut<DeathViewState>,
    mut camera_shake: ResMut<CameraShakeState>,
    attack_state: Res<PlayerAttackState>,
    mut burning_corpse_contact: ResMut<BurningCorpseContactHazardState>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();

    if dead.0 {
        return;
    }

    let player_pos = camera.0.position;

    // Tick enemies and collect dead entities for despawning.
    let mut dead_enemies = Vec::new();
    for (entity, mut enemy) in enemy_q.iter_mut() {
        if let Some(proj) = tick_single_enemy(&mut enemy, player_pos, &map.0, dt) {
            projectiles.0.push(proj);
        }
        if matches!(enemy.state, EnemyState::Dead) {
            dead_enemies.push(entity);
        }
    }
    for entity in dead_enemies {
        commands.entity(entity).despawn();
    }

    // Tick mosquitons and collect dead entities for despawning.
    let mut dead_mosquitons = Vec::new();
    for (entity, mut mosquiton) in mosquiton_q.iter_mut() {
        let (proj, dmg) = tick_single_mosquiton(&mut mosquiton, player_pos, &map.0, dt);
        if let Some(p) = proj {
            projectiles.0.push(p);
        }
        if let Some((amount, source)) = dmg {
            apply_player_damage(
                &mut health.0,
                &mut dead.0,
                &mut death_view,
                &mut camera_shake,
                &camera.0,
                amount,
                Some(source),
            );
        }
        if matches!(mosquiton.state, MosquitonState::Dead) {
            dead_mosquitons.push(entity);
        }
    }
    for entity in dead_mosquitons {
        commands.entity(entity).despawn();
    }

    destroy_projectiles_touching_active_flamethrower(
        &camera.0,
        &map.0,
        &attack_state,
        &mut projectiles.0,
        &mut impacts.0,
    );

    let projectile_result = intercept_and_tick_projectiles(
        &camera.0,
        &map.0,
        &attack_state,
        &mut projectiles.0,
        &mut impacts.0,
        dt,
    );

    apply_player_damage(
        &mut health.0,
        &mut dead.0,
        &mut death_view,
        &mut camera_shake,
        &camera.0,
        projectile_result.player_damage,
        projectile_result.damage_source,
    );

    // Collect burning corpses from remaining enemies and mosquitons.
    let mut burning_corpses = Vec::new();
    for (_, enemy) in enemy_q.iter() {
        if matches!(enemy.state, EnemyState::BurningCorpse { .. }) {
            burning_corpses.push(enemy.position);
        }
    }
    for (_, mosquiton) in mosquiton_q.iter() {
        if matches!(mosquiton.state, MosquitonState::BurningCorpse { .. }) {
            burning_corpses.push(mosquiton.position);
        }
    }
    let burning_corpse_contact_result = tick_burning_corpse_contact_damage(
        &camera.0,
        &burning_corpses,
        attack_state.config(),
        &mut burning_corpse_contact,
        dt,
    );
    apply_player_damage(
        &mut health.0,
        &mut dead.0,
        &mut death_view,
        &mut camera_shake,
        &camera.0,
        burning_corpse_contact_result.player_damage,
        burning_corpse_contact_result.damage_source,
    );
    tick_burning_corpse_crossfire_query(
        &mut enemy_q,
        &mut mosquiton_q,
        &burning_corpses,
        attack_state.config(),
    );
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct BurningCorpseContactDamageResult {
    player_damage: u32,
    damage_source: Option<Vec2>,
}

fn tick_burning_corpse_contact_damage(
    camera: &Camera,
    burning_corpses: &[Vec2],
    config: &FlamethrowerConfig,
    state: &mut BurningCorpseContactHazardState,
    dt: f32,
) -> BurningCorpseContactDamageResult {
    state.cooldown_remaining_secs = (state.cooldown_remaining_secs - dt).max(0.0);

    if config.burning_corpse_contact_damage == 0 || config.burning_corpse_contact_radius <= 0.0 {
        return BurningCorpseContactDamageResult::default();
    }

    assert!(
        config.burning_corpse_contact_tick_ms > 0,
        "burning_corpse_contact_tick_ms must be greater than zero"
    );
    if state.cooldown_remaining_secs > 0.0 {
        return BurningCorpseContactDamageResult::default();
    }

    let Some(source) = closest_burning_corpse_to(
        camera.position,
        burning_corpses,
        config.burning_corpse_contact_radius,
    ) else {
        return BurningCorpseContactDamageResult::default();
    };

    state.cooldown_remaining_secs = config.burning_corpse_contact_tick_secs();
    BurningCorpseContactDamageResult {
        player_damage: config.burning_corpse_contact_damage,
        damage_source: Some(source),
    }
}

#[cfg(test)]
fn collect_burning_corpses(enemies: &[Enemy], mosquitons: &[Mosquiton]) -> Vec<Vec2> {
    let mut corpses = Vec::new();
    for enemy in enemies {
        if matches!(enemy.state, EnemyState::BurningCorpse { .. }) {
            corpses.push(enemy.position);
        }
    }
    for mosquiton in mosquitons {
        if matches!(mosquiton.state, MosquitonState::BurningCorpse { .. }) {
            corpses.push(mosquiton.position);
        }
    }
    corpses
}

#[cfg(test)]
fn tick_burning_corpse_crossfire(
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    burning_corpses: &[Vec2],
    config: &FlamethrowerConfig,
) {
    if config.burning_corpse_crossfire_damage == 0 || burning_corpses.is_empty() {
        return;
    }

    let fire_death_secs = config.burning_corpse_duration_secs;
    let radius = config.burning_corpse_contact_radius;
    let damage = config.burning_corpse_crossfire_damage;

    for mosquiton in mosquitons.iter_mut() {
        if !mosquiton.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(mosquiton.position, burning_corpses, radius).is_some() {
            mosquiton.take_damage_from(damage, DamageKind::Fire, fire_death_secs);
        }
    }

    for enemy in enemies.iter_mut() {
        if !enemy.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(enemy.position, burning_corpses, radius).is_some() {
            enemy.take_damage_from(damage, DamageKind::Fire, fire_death_secs);
        }
    }
}

fn closest_burning_corpse_to(pos: Vec2, corpses: &[Vec2], radius: f32) -> Option<Vec2> {
    let radius_sq = radius * radius;
    corpses
        .iter()
        .filter_map(|&corpse| {
            let dist_sq = corpse.distance_squared(pos);
            if dist_sq <= radius_sq {
                Some((corpse, dist_sq))
            } else {
                None
            }
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(pos, _)| pos)
}

fn tick_burning_corpse_crossfire_query(
    enemy_q: &mut Query<(Entity, &mut Enemy)>,
    mosquiton_q: &mut Query<(Entity, &mut Mosquiton)>,
    burning_corpses: &[Vec2],
    config: &FlamethrowerConfig,
) {
    if config.burning_corpse_crossfire_damage == 0 || burning_corpses.is_empty() {
        return;
    }

    let fire_death_secs = config.burning_corpse_duration_secs;
    let radius = config.burning_corpse_contact_radius;
    let damage = config.burning_corpse_crossfire_damage;

    for (_, mut mosquiton) in mosquiton_q.iter_mut() {
        if !mosquiton.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(mosquiton.position, burning_corpses, radius).is_some() {
            mosquiton.take_damage_from(damage, DamageKind::Fire, fire_death_secs);
        }
    }

    for (_, mut enemy) in enemy_q.iter_mut() {
        if !enemy.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(enemy.position, burning_corpses, radius).is_some() {
            enemy.take_damage_from(damage, DamageKind::Fire, fire_death_secs);
        }
    }
}

fn intercept_and_tick_projectiles(
    camera: &Camera,
    map: &Map,
    attack_state: &PlayerAttackState,
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    dt: f32,
) -> ProjectileTickResult {
    destroy_projectiles_touching_active_flamethrower(
        camera,
        map,
        attack_state,
        projectiles,
        impacts,
    );
    let result = tick_projectiles(projectiles, camera.position, map, dt);
    impacts.extend(result.impacts.iter().cloned());
    result
}

fn apply_player_damage(
    health: &mut u32,
    dead: &mut bool,
    death_view: &mut DeathViewState,
    camera_shake: &mut CameraShakeState,
    camera: &Camera,
    damage: u32,
    damage_source: Option<Vec2>,
) {
    if *dead || damage == 0 {
        return;
    }

    request_camera_shake(camera_shake);
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
    camera: Res<CameraRes>,
    map: Res<MapRes>,
    config: Res<Config>,
    time: Res<Time>,
    mut enemy_q: Query<(Entity, &mut Enemy)>,
    mut mosquiton_q: Query<(Entity, &mut Mosquiton)>,
    mut projectiles: ResMut<Projectiles>,
    mut impacts: ResMut<ProjectileImpacts>,
    mut char_decals: ResMut<CharDecals>,
    dead: Res<PlayerDead>,
    mut shoot: ResMut<ShootRequest>,
    mut attack_input: ResMut<AttackInput>,
    mut attack_loadout: ResMut<AttackLoadout>,
    mut attack_state: ResMut<PlayerAttackState>,
) {
    if dead.0 {
        shoot.0 = false;
        attack_input.clear_edges();
        return;
    }

    // Gather enemies into Vecs for slice-based attack processing.
    let mut enemy_entities: Vec<Entity> = Vec::new();
    let mut enemies: Vec<Enemy> = Vec::new();
    for (entity, enemy) in enemy_q.iter() {
        enemy_entities.push(entity);
        enemies.push(enemy.clone());
    }

    // Gather mosquitons into Vecs for slice-based attack processing.
    let mut mosquiton_entities: Vec<Entity> = Vec::new();
    let mut mosquitons: Vec<Mosquiton> = Vec::new();
    for (entity, mosquiton) in mosquiton_q.iter() {
        mosquiton_entities.push(entity);
        mosquitons.push(mosquiton.clone());
    }

    process_player_attacks(
        &camera.0,
        &map.0,
        config.hitscan_damage,
        time.delta_secs(),
        time.elapsed_secs(),
        &mut attack_input,
        &mut attack_loadout,
        &mut attack_state,
        &mut enemies,
        &mut mosquitons,
        &mut projectiles.0,
        &mut impacts.0,
        &mut char_decals.0,
        config.screen_height as f32,
        &mut shoot.0,
    );

    // Scatter: write back changes to entities.
    // Despawn is handled by tick_enemy_ai which runs after this system.
    for (i, &entity) in enemy_entities.iter().enumerate() {
        if !matches!(enemies[i].state, EnemyState::Dead)
            && let Ok((_, mut e)) = enemy_q.get_mut(entity)
        {
            *e = enemies[i].clone();
        }
    }

    for (i, &entity) in mosquiton_entities.iter().enumerate() {
        if !matches!(mosquitons[i].state, MosquitonState::Dead)
            && let Ok((_, mut m)) = mosquiton_q.get_mut(entity)
        {
            *m = mosquitons[i].clone();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_fp_view(
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    handle: Res<SpriteHandle>,
    sources: RenderSources,
    view: ViewResources,
    time: Res<Time>,
    enemy_q: Query<(&Enemy, &EnemySpriteIndex)>,
    mosquiton_q: Query<&Mosquiton>,
) {
    let mut image = CxImage::empty(UVec2::new(
        view.config.screen_width,
        view.config.screen_height,
    ));

    // Gather enemies with sprite indices and mosquitons for billboard rendering.
    let mut enemies: Vec<Enemy> = Vec::new();
    let mut indices: Vec<usize> = Vec::new();
    for (e, idx) in enemy_q.iter() {
        enemies.push(e.clone());
        indices.push(idx.0);
    }
    let mosquitons: Vec<Mosquiton> = mosquiton_q.iter().cloned().collect();

    let enemy_bbs = billboards_from_enemies_indexed(&enemies, &indices, &sources.pairs.0);
    let corpse_flame_bbs = burning_corpse_flame_billboards(
        &enemies,
        &indices,
        &sources.pairs.0,
        &mosquitons,
        &sources.mosquiton_sprites.0,
        &CorpseFlameContext {
            attack_sprites: &view.attack_sprites,
            config: view.attack_state.config(),
            camera: &view.camera.0,
            elapsed_secs: time.elapsed_secs(),
        },
    );
    let proj_bbs =
        billboards_from_projectiles(&sources.projectiles.0, &sources.blood_shot_sprites.0.hover);
    let impact_bbs =
        billboards_from_projectile_impacts(&sources.impacts.0, &sources.blood_shot_sprites.0);
    let mosquiton_bbs = billboards_from_mosquitons(&mosquitons, &sources.mosquiton_sprites.0);
    let all_bbs: Vec<Billboard> = sources
        .static_bbs
        .0
        .iter()
        .cloned()
        .chain(corpse_flame_bbs)
        .chain(enemy_bbs)
        .chain(mosquiton_bbs)
        .chain(impact_bbs)
        .chain(proj_bbs)
        .collect();

    let impact_sprite = wall_impact_sprite(&view.attack_state, &view.attack_sprites);
    let impact_sprites = impact_sprite.into_iter().collect::<Vec<_>>();
    let wall_effects = FpWallRenderEffects {
        char_decals: &sources.char_decals.0,
        char_mask: Some(flame_wall_mask(&view.attack_sprites)),
        surface_sprites: &impact_sprites,
    };

    render_fp_scene_with_effects(
        &mut image,
        &view.map.0,
        &view.camera.0,
        &view.textures.0,
        &view.palette.0,
        &all_bbs,
        &wall_effects,
        Some(&view.sky),
    );

    apply_framebuffer_offset(&mut image, view.camera_shake.current_offset);
    draw_player_attack_overlays(
        &mut image,
        &view.camera.0,
        &view.map.0,
        &view.attack_sprites,
        &view.attack_loadout,
        &view.attack_state,
        time.elapsed_secs(),
    );

    if view.dead.0 {
        draw_overlay_tint(&mut image, 2, death_red_density(&view.death_view));
    } else {
        draw_crosshair(&mut image, 4);
    }

    // Health bar at bottom-left.
    let bar_w = 20;
    let filled = (view.health.0 as i32 * bar_w / view.config.player_max_health as i32).max(0);
    {
        let data = image.data_mut();
        let w = view.config.screen_width as i32;
        let h = view.config.screen_height as i32;
        for x in 1..=bar_w {
            let color = if x <= filled { 2 } else { 1 };
            data[((h - 3) * w + x) as usize] = color;
            data[((h - 2) * w + x) as usize] = color;
        }
    }

    if let Some(asset) = sprite_assets.get_mut(&handle.0) {
        *asset = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    }
}

struct CorpseFlameContext<'a> {
    attack_sprites: &'a PlayerAttackSprites,
    config: &'a FlamethrowerConfig,
    camera: &'a Camera,
    elapsed_secs: f32,
}

fn burning_corpse_flame_billboards(
    enemies: &[Enemy],
    enemy_sprite_indices: &[usize],
    enemy_sprite_pairs: &[(CxImage, CxImage)],
    mosquitons: &[Mosquiton],
    mosquiton_sprites: &MosquitonBillboardSprites,
    ctx: &CorpseFlameContext<'_>,
) -> Vec<Billboard> {
    let mut billboards = Vec::new();
    for (index, enemy) in enemies.iter().enumerate() {
        if let EnemyState::BurningCorpse { seed, .. } = enemy.state {
            let pair_index = enemy_sprite_indices.get(index).copied().unwrap_or(0);
            let Some((corpse_sprite, _death_sprite)) = enemy_sprite_pairs
                .get(pair_index)
                .or_else(|| enemy_sprite_pairs.first())
            else {
                continue;
            };
            push_burning_corpse_flames(
                &mut billboards,
                enemy.position,
                0.0,
                1.0,
                seed,
                corpse_sprite,
                ctx,
            );
        }
    }
    for mosquiton in mosquitons {
        if let MosquitonState::BurningCorpse { seed, .. } = mosquiton.state {
            let corpse_sprite = mosquiton_sprites.alive_sprite_at(0.0);
            push_burning_corpse_flames(
                &mut billboards,
                mosquiton.position,
                mosquiton.height,
                mosquiton.config.billboard_height,
                seed,
                corpse_sprite,
                ctx,
            );
        }
    }
    billboards
}
fn push_burning_corpse_flames(
    billboards: &mut Vec<Billboard>,
    position: Vec2,
    height: f32,
    base_world_height: f32,
    seed: u32,
    corpse_sprite: &CxImage,
    ctx: &CorpseFlameContext<'_>,
) {
    if ctx.config.burning_flame_count == 0 {
        return;
    }

    let to_corpse = position - ctx.camera.position;
    let distance = to_corpse.length().max(0.1);

    let behind_dir = if distance > 0.001 {
        to_corpse / distance
    } else {
        ctx.camera.direction()
    };

    let right = Vec2::new(-ctx.camera.direction().y, ctx.camera.direction().x);

    let fire_config = ctx.config.fire_death_config();

    let flames = perimeter_flames_from_mask(
        seed,
        corpse_sprite.width(),
        corpse_sprite.height(),
        |x, y| corpse_sprite.data()[y * corpse_sprite.width() + x] != TRANSPARENT_INDEX,
        &fire_config,
    );

    let px_to_world = base_world_height / corpse_sprite.height() as f32;

    for flame in flames {
        let lateral_units = flame.offset_px.x * px_to_world;
        let vertical_units = flame.offset_px.y * px_to_world;

        billboards.push(Billboard {
            position: position + behind_dir * 0.04 + right * lateral_units,
            height: height + vertical_units,
            world_height: base_world_height * 0.35 * flame.scale,
            sprite: ctx
                .attack_sprites
                .flame_frame_loop(ctx.elapsed_secs + flame.phase_secs)
                .clone(),
        });
    }
}

/// Set to `true` from your input system to trigger a hitscan shot.
/// The plugin resets it to `false` after processing.
#[derive(Resource, Default)]
pub struct ShootRequest(pub bool);

#[cfg(test)]
mod tests {
    use super::*;

    fn contact_hazard_test_config() -> FlamethrowerConfig {
        let mut config = FlamethrowerConfig::load();
        config.burning_corpse_contact_damage = 1;
        config.burning_corpse_contact_tick_ms = 300;
        config.burning_corpse_contact_radius = 0.6;
        config
    }

    #[test]
    fn burning_corpse_flames_are_deterministic_billboards() {
        let sprites = PlayerAttackSprites::load();
        let enemy_pairs = vec![(make_enemy_sprite(24, 2), make_death_sprite(24, 1))];
        let mosquiton_sprites = make_mosquiton_billboard_sprites().unwrap();
        let config = FlamethrowerConfig::load();
        let camera = Camera {
            position: Vec2::new(1.0, 1.0),
            angle: 0.0,
            ..Default::default()
        };
        let mut enemies = vec![Enemy::new(Vec2::new(3.0, 1.0), 10, 1.0)];
        enemies[0].state = EnemyState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };

        let ctx = CorpseFlameContext {
            attack_sprites: &sprites,
            config: &config,
            camera: &camera,
            elapsed_secs: 0.2,
        };

        let first = burning_corpse_flame_billboards(
            &enemies,
            &[0],
            &enemy_pairs,
            &[],
            &mosquiton_sprites,
            &ctx,
        );
        let second = burning_corpse_flame_billboards(
            &enemies,
            &[0],
            &enemy_pairs,
            &[],
            &mosquiton_sprites,
            &ctx,
        );

        assert_eq!(first.len(), config.burning_flame_count);
        assert_eq!(second.len(), first.len());
        assert_eq!(first[0].position, second[0].position);
        assert_eq!(first[0].height, second[0].height);
        assert!(
            first
                .iter()
                .any(|flame| flame.position.distance(enemies[0].position) > 0.05)
        );
    }

    #[test]
    fn mosquiton_burning_corpse_flames_use_frozen_frame() {
        let sprites = PlayerAttackSprites::load();
        let enemy_pairs = vec![(make_enemy_sprite(24, 2), make_death_sprite(24, 1))];
        let mosquiton_sprites = make_mosquiton_billboard_sprites().unwrap();
        let config = FlamethrowerConfig::load();
        let camera = Camera {
            position: Vec2::new(1.0, 1.0),
            angle: 0.0,
            ..Default::default()
        };
        let mut mosquitons = vec![Mosquiton::new(
            Vec2::new(3.0, 1.0),
            MosquitonConfig::default(),
        )];
        mosquitons[0].state = MosquitonState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };

        let ctx = CorpseFlameContext {
            attack_sprites: &sprites,
            config: &config,
            camera: &camera,
            elapsed_secs: 0.2,
        };

        let first = burning_corpse_flame_billboards(
            &[],
            &[],
            &enemy_pairs,
            &mosquitons,
            &mosquiton_sprites,
            &ctx,
        );
        mosquitons[0].animation_time = 0.75;
        let second = burning_corpse_flame_billboards(
            &[],
            &[],
            &enemy_pairs,
            &mosquitons,
            &mosquiton_sprites,
            &ctx,
        );

        assert_eq!(second.len(), first.len());
        assert_eq!(first[0].position, second[0].position);
        assert_eq!(first[0].height, second[0].height);
    }

    #[test]
    fn burning_corpse_contact_damage_ticks_on_global_cooldown() {
        let config = contact_hazard_test_config();
        let camera = Camera {
            position: Vec2::new(1.0, 1.0),
            ..Default::default()
        };
        let mut enemies = vec![Enemy::new(Vec2::new(1.5, 1.0), 10, 1.0)];
        enemies[0].state = EnemyState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };
        let corpses = collect_burning_corpses(&enemies, &[]);
        let mut state = BurningCorpseContactHazardState::default();

        let first = tick_burning_corpse_contact_damage(&camera, &corpses, &config, &mut state, 0.0);
        let cooldown_blocked =
            tick_burning_corpse_contact_damage(&camera, &corpses, &config, &mut state, 0.1);
        let after_tick =
            tick_burning_corpse_contact_damage(&camera, &corpses, &config, &mut state, 0.201);

        assert_eq!(first.player_damage, 1);
        assert_eq!(first.damage_source, Some(enemies[0].position));
        assert_eq!(cooldown_blocked.player_damage, 0);
        assert_eq!(after_tick.player_damage, 1);
    }

    #[test]
    fn burning_corpse_contact_damage_stops_when_away_or_dead() {
        let config = contact_hazard_test_config();
        let camera = Camera {
            position: Vec2::new(1.0, 1.0),
            ..Default::default()
        };
        let mut enemies = vec![
            Enemy::new(Vec2::new(2.0, 1.0), 10, 1.0),
            Enemy::new(Vec2::new(1.2, 1.0), 10, 1.0),
        ];
        enemies[0].state = EnemyState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };
        enemies[1].state = EnemyState::Dead;
        let corpses = collect_burning_corpses(&enemies, &[]);
        let mut state = BurningCorpseContactHazardState::default();

        let result =
            tick_burning_corpse_contact_damage(&camera, &corpses, &config, &mut state, 0.3);

        assert_eq!(result.player_damage, 0);
    }

    #[test]
    fn multiple_burning_corpses_do_not_stack_contact_damage() {
        let config = contact_hazard_test_config();
        let camera = Camera {
            position: Vec2::new(1.0, 1.0),
            ..Default::default()
        };
        let mut enemies = [
            Enemy::new(Vec2::new(1.4, 1.0), 10, 1.0),
            Enemy::new(Vec2::new(1.5, 1.0), 10, 1.0),
        ];
        for enemy in &mut enemies {
            enemy.state = EnemyState::BurningCorpse {
                timer: 1.0,
                seed: 42,
            };
        }
        let corpses = collect_burning_corpses(&enemies, &[]);
        let mut state = BurningCorpseContactHazardState::default();

        let first = tick_burning_corpse_contact_damage(&camera, &corpses, &config, &mut state, 0.0);
        let same_frame =
            tick_burning_corpse_contact_damage(&camera, &corpses, &config, &mut state, 0.0);

        assert_eq!(first.player_damage, 1);
        assert_eq!(first.damage_source, Some(enemies[0].position));
        assert_eq!(same_frame.player_damage, 0);
    }

    #[test]
    fn mosquiton_burning_corpse_contact_damage_uses_same_hazard() {
        let config = contact_hazard_test_config();
        let camera = Camera {
            position: Vec2::new(1.0, 1.0),
            ..Default::default()
        };
        let mut mosquitons = vec![Mosquiton::new(
            Vec2::new(1.2, 1.0),
            MosquitonConfig::default(),
        )];
        mosquitons[0].state = MosquitonState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };
        let corpses = collect_burning_corpses(&[], &mosquitons);
        let mut state = BurningCorpseContactHazardState::default();

        let result =
            tick_burning_corpse_contact_damage(&camera, &corpses, &config, &mut state, 0.0);

        assert_eq!(result.player_damage, 1);
        assert_eq!(result.damage_source, Some(mosquitons[0].position));
    }

    #[test]
    fn crossfire_damage_hits_living_enemy_near_burning_mosquiton() {
        let mut config = FlamethrowerConfig::load();
        config.burning_corpse_crossfire_damage = 10;
        let mut enemies = vec![Enemy::new(Vec2::new(1.0, 0.0), 10, 1.0)];
        let mut mosquitons = vec![Mosquiton::new(
            Vec2::new(1.3, 0.0),
            MosquitonConfig::default(),
        )];
        mosquitons[0].state = MosquitonState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };
        let corpses = collect_burning_corpses(&enemies, &mosquitons);

        tick_burning_corpse_crossfire(&mut enemies, &mut mosquitons, &corpses, &config);

        assert_eq!(enemies[0].health, 0);
        assert!(matches!(enemies[0].state, EnemyState::BurningCorpse { .. }));
    }

    #[test]
    fn crossfire_damage_hits_living_mosquiton_near_burning_enemy() {
        let mut config = FlamethrowerConfig::load();
        config.burning_corpse_crossfire_damage = MosquitonConfig::default().health;
        let mut enemies = vec![Enemy::new(Vec2::new(1.0, 0.0), 10, 1.0)];
        enemies[0].state = EnemyState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };
        let mut mosquitons = vec![Mosquiton::new(
            Vec2::new(1.3, 0.0),
            MosquitonConfig::default(),
        )];
        let corpses = collect_burning_corpses(&enemies, &mosquitons);

        tick_burning_corpse_crossfire(&mut enemies, &mut mosquitons, &corpses, &config);

        assert_eq!(mosquitons[0].health, 0);
        assert!(matches!(
            mosquitons[0].state,
            MosquitonState::BurningCorpse { .. }
        ));
    }

    #[test]
    fn crossfire_damage_ignores_out_of_range() {
        let mut config = FlamethrowerConfig::load();
        config.burning_corpse_crossfire_damage = 2;
        config.burning_corpse_contact_radius = 0.5;
        let mut enemies = vec![Enemy::new(Vec2::new(0.0, 0.0), 10, 1.0)];
        let mut mosquitons = vec![Mosquiton::new(
            Vec2::new(1.0, 0.0),
            MosquitonConfig::default(),
        )];
        mosquitons[0].state = MosquitonState::BurningCorpse {
            timer: 1.0,
            seed: 42,
        };
        let corpses = collect_burning_corpses(&enemies, &mosquitons);

        tick_burning_corpse_crossfire(&mut enemies, &mut mosquitons, &corpses, &config);

        assert_eq!(enemies[0].health, 10);
    }

    #[test]
    fn quick_turn_fires_on_back_release_after_simultaneous_press() {
        let mut debounce = QuickTurnDebounce::default();
        // Simultaneous press: arms the chord, does not fire yet.
        assert!(!resolve_quick_turn_pressed(
            true, true, true, true, false, 0.0, &mut debounce
        ));
        // Still held: nothing fires.
        assert!(!resolve_quick_turn_pressed(
            true, true, false, false, false, 0.01, &mut debounce
        ));
        // Back released: fires.
        assert!(resolve_quick_turn_pressed(
            false, true, false, false, true, 0.02, &mut debounce
        ));
        // Already consumed, hold blocked.
        assert!(!resolve_quick_turn_pressed(
            false, true, false, false, false, 0.03, &mut debounce
        ));
        // Full release resets.
        assert!(!resolve_quick_turn_pressed(
            false, false, false, false, false, 0.04, &mut debounce
        ));
        // Can re-arm and fire again.
        assert!(!resolve_quick_turn_pressed(
            true, true, true, true, false, 0.05, &mut debounce
        ));
        assert!(resolve_quick_turn_pressed(
            false, true, false, false, true, 0.06, &mut debounce
        ));
    }

    #[test]
    fn quick_turn_fires_on_release_after_staggered_press_inside_grace() {
        let mut debounce = QuickTurnDebounce::default();
        // Back pressed first.
        assert!(!resolve_quick_turn_pressed(
            true, false, true, false, false, 1.0, &mut debounce
        ));
        // B pressed within grace: arms.
        assert!(!resolve_quick_turn_pressed(
            true, true, false, true, false, 1.04, &mut debounce
        ));
        // Back released: fires.
        assert!(resolve_quick_turn_pressed(
            false, true, false, false, true, 1.05, &mut debounce
        ));
    }

    #[test]
    fn quick_turn_blocks_staggered_press_after_grace_until_release() {
        let mut debounce = QuickTurnDebounce::default();
        // Back pressed first.
        assert!(!resolve_quick_turn_pressed(
            true, false, true, false, false, 2.0, &mut debounce
        ));
        // Grace window passes without B.
        assert!(!resolve_quick_turn_pressed(
            true, false, false, false, false, 2.09, &mut debounce
        ));
        // B pressed too late: blocked.
        assert!(!resolve_quick_turn_pressed(
            true, true, false, true, false, 2.1, &mut debounce
        ));
        // Full release resets.
        assert!(!resolve_quick_turn_pressed(
            false, false, false, false, false, 2.2, &mut debounce
        ));
        // Can fire again after reset.
        assert!(!resolve_quick_turn_pressed(
            true, true, true, true, false, 2.3, &mut debounce
        ));
        assert!(resolve_quick_turn_pressed(
            false, true, false, false, true, 2.31, &mut debounce
        ));
    }

    #[test]
    fn quick_turn_animates_left_over_point_four_seconds() {
        let mut camera = Camera {
            angle: 0.25,
            ..Default::default()
        };
        let mut quick_turn = QuickTurnState::default();
        request_quick_turn(&mut quick_turn);

        tick_quick_turn(&mut camera, &mut quick_turn, 0.2);
        assert!((camera.angle - (0.25 + std::f32::consts::FRAC_PI_2)).abs() < 1e-5);
        assert!(quick_turn.remaining_radians > 0.0);

        tick_quick_turn(&mut camera, &mut quick_turn, 0.2);
        assert!((camera.angle - (0.25 + std::f32::consts::PI)).abs() < 1e-5);
        assert!(quick_turn.remaining_radians <= 1e-5);
    }

    #[test]
    fn side_turn_left_rotates_90_degrees() {
        let mut camera = Camera {
            angle: 0.0,
            ..Default::default()
        };
        let mut state = QuickTurnState::default();
        request_side_turn(&mut state, true);

        tick_quick_turn(&mut camera, &mut state, 0.4);
        assert!((camera.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!(state.remaining_radians <= 1e-5);
    }

    #[test]
    fn side_turn_right_rotates_negative_90_degrees() {
        let mut camera = Camera {
            angle: 1.0,
            ..Default::default()
        };
        let mut state = QuickTurnState::default();
        request_side_turn(&mut state, false);

        tick_quick_turn(&mut camera, &mut state, 0.4);
        let expected = (1.0 - std::f32::consts::FRAC_PI_2).rem_euclid(std::f32::consts::TAU);
        assert!((camera.angle - expected).abs() < 1e-5);
        assert!(state.remaining_radians <= 1e-5);
    }

    #[test]
    fn quick_turn_request_ignored_while_active() {
        let mut quick_turn = QuickTurnState::default();
        request_quick_turn(&mut quick_turn);
        tick_quick_turn(&mut Camera::default(), &mut quick_turn, 0.05);
        let remaining = quick_turn.remaining_radians;
        request_quick_turn(&mut quick_turn);
        assert_eq!(quick_turn.remaining_radians, remaining);
    }

    #[test]
    fn death_view_turns_toward_killer_and_red_increases() {
        let mut camera = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            ..Default::default()
        };
        let mut death_view = DeathViewState::default();

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
        let mut camera = Camera {
            position: Vec2::ZERO,
            angle: 350.0_f32.to_radians(),
            ..Default::default()
        };
        let mut death_view = DeathViewState::default();
        let ten_degrees = Vec2::new(10.0_f32.to_radians().cos(), 10.0_f32.to_radians().sin());

        request_death_view(&mut death_view, &camera, ten_degrees);
        tick_death_view(&mut camera, &mut death_view, DEATH_TURN_DURATION_SECS);
        assert!((camera.angle - 10.0_f32.to_radians()).abs() < 1e-5);

        camera.angle = 10.0_f32.to_radians();
        let mut death_view = DeathViewState::default();
        let three_fifty_degrees =
            Vec2::new(350.0_f32.to_radians().cos(), 350.0_f32.to_radians().sin());
        request_death_view(&mut death_view, &camera, three_fifty_degrees);
        tick_death_view(&mut camera, &mut death_view, DEATH_TURN_DURATION_SECS);
        assert!((camera.angle - 350.0_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn player_damage_latches_first_killing_source() {
        let camera = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            ..Default::default()
        };
        let mut health = 10;
        let mut dead = false;
        let mut death_view = DeathViewState::default();
        let mut camera_shake = CameraShakeState::default();

        apply_player_damage(
            &mut health,
            &mut dead,
            &mut death_view,
            &mut camera_shake,
            &camera,
            10,
            Some(Vec2::Y),
        );
        let first_target = death_view.target_angle;
        apply_player_damage(
            &mut health,
            &mut dead,
            &mut death_view,
            &mut camera_shake,
            &camera,
            10,
            Some(Vec2::NEG_Y),
        );

        assert!(dead);
        assert_eq!(health, 0);
        assert!((first_target - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(death_view.target_angle, first_target);
    }

    #[test]
    fn player_damage_requests_camera_shake() {
        let camera = Camera::default();
        let mut health = 100;
        let mut dead = false;
        let mut death_view = DeathViewState::default();
        let mut camera_shake = CameraShakeState::default();

        apply_player_damage(
            &mut health,
            &mut dead,
            &mut death_view,
            &mut camera_shake,
            &camera,
            1,
            None,
        );

        assert_eq!(health, 99);
        assert!(!dead);
        assert_eq!(camera_shake.intensity, CAMERA_SHAKE_BASE_INTENSITY);
    }

    #[test]
    fn active_flamethrower_intercepts_projectile_before_projectile_damage() {
        let camera = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            ..Default::default()
        };
        let map = Map {
            width: 8,
            height: 8,
            cells: vec![0; 64],
        };
        let mut input = AttackInput {
            shoot_just_pressed: true,
            shoot_held: true,
            cursor_x: 80.0,
            ..Default::default()
        };
        let mut loadout = AttackLoadout::default();
        let mut attack_state = PlayerAttackState::default();
        let mut enemies = Vec::new();
        let mut mosquitons = Vec::new();
        let mut setup_projectiles = Vec::new();
        let mut setup_impacts = Vec::new();
        let mut char_decals = Vec::new();
        let mut shoot_request = false;

        process_player_attacks(
            &camera,
            &map,
            15,
            1.0 / 60.0,
            0.0,
            &mut input,
            &mut loadout,
            &mut attack_state,
            &mut enemies,
            &mut mosquitons,
            &mut setup_projectiles,
            &mut setup_impacts,
            &mut char_decals,
            144.0,
            &mut shoot_request,
        );

        let mut projectiles = vec![Projectile {
            position: Vec2::new(0.7, 0.0),
            source_position: Vec2::new(3.0, 0.0),
            direction: -Vec2::X,
            speed: 10.0,
            radius: 0.3,
            damage: 10,
            lifetime: 1.0,
            alive: true,
        }];
        let mut impacts = Vec::new();

        let result = intercept_and_tick_projectiles(
            &camera,
            &map,
            &attack_state,
            &mut projectiles,
            &mut impacts,
            1.0,
        );

        assert_eq!(result.player_damage, 0);
        assert!(projectiles.is_empty());
        assert_eq!(impacts.len(), 1);
    }

    #[test]
    fn camera_shake_reinforces_decays_and_clears() {
        let mut camera_shake = CameraShakeState::default();
        request_camera_shake(&mut camera_shake);
        request_camera_shake(&mut camera_shake);

        assert_eq!(camera_shake.intensity, CAMERA_SHAKE_BASE_INTENSITY * 2.0);

        tick_camera_shake(&mut camera_shake, 0.1, 0.0, 1.0);
        assert_eq!(
            camera_shake.current_offset,
            IVec2::new((CAMERA_SHAKE_BASE_INTENSITY * 2.0) as i32, 0)
        );
        assert!(camera_shake.intensity < CAMERA_SHAKE_BASE_INTENSITY * 2.0);

        camera_shake.intensity = CAMERA_SHAKE_THRESHOLD * 0.5;
        camera_shake.current_offset = IVec2::new(1, 1);
        tick_camera_shake(&mut camera_shake, 0.016, 0.0, 0.0);
        assert_eq!(camera_shake.intensity, 0.0);
        assert_eq!(camera_shake.current_offset, IVec2::ZERO);
    }

    #[test]
    fn framebuffer_offset_clamps_edges() {
        let mut image = CxImage::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9], 3);

        apply_framebuffer_offset(&mut image, IVec2::new(1, 0));

        assert_eq!(image.data(), &[1, 1, 2, 4, 4, 5, 7, 7, 8]);
    }
}
