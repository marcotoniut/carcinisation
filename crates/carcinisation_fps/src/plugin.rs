//! Bevy plugin that encapsulates the first-person raycaster systems.
//!
//! The plugin is generic over the game's layer type. The caller provides
//! a [`Config`] that specifies which layer the First-person view renders into
//! and the path to the map RON file.

use std::marker::PhantomData;

use bevy::{ecs::system::SystemParam, prelude::*};
use carapace::prelude::*;
use carcinisation_fps_core::ScreenParticleConfig;
use carcinisation_fps_core::fire_death::perimeter_flames_from_mask;
use carcinisation_fps_core::ground_fire::{
    GroundFire, GroundFireConfig, GroundFireContactState, ground_fire_contact_damage,
    ground_fire_flame_layout, tick_ground_fires, try_spawn_ground_fire,
};

/// System set for First-person plugin systems. External input systems should run
/// `.before(Systems)` so the First-person plugin reads updated state.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Systems;

use crate::{
    billboard::{
        Billboard, billboard_from_enemy, billboard_from_mosquiton, billboard_from_spidey,
        billboards_from_enemies_indexed, billboards_from_mosquitons,
        billboards_from_projectile_impacts, billboards_from_projectiles, billboards_from_spideys,
        make_death_sprite, make_enemy_sprite, make_pillar_sprite,
    },
    camera::Camera,
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
        AttackInput, AttackLoadout, GroundFireVisualConfig, PlayerAttackSprites, PlayerAttackState,
        SnapTurnVisualInput, destroy_projectiles_touching_active_flamethrower,
        draw_player_attack_overlays, flame_wall_mask, process_player_attacks, wall_impact_sprite,
    },
    render::{
        CharDecal, FpWallRenderEffects, Palette, draw_crosshair, draw_overlay_tint,
        render_fp_scene, render_fp_scene_with_effects,
    },
    screen_particles::{
        FpsScreenParticles, draw_fps_screen_particles, update_fps_screen_particles,
    },
    sky::Sky,
    spidey::{
        SpiderShotBillboardSprites, Spidey, SpideyBillboardSprites, SpideyConfig, SpideyState,
        make_spider_shot_billboard_sprites, make_spidey_billboard_sprites, tick_single_spidey,
    },
};

/// Maximum time B can be held before the chord window expires.
/// If no valid direction is pressed within this window, the chord is cancelled.
const CHORD_WINDOW_SECS: f32 = 0.20;

/// Configuration for the First-person plugin.
///
/// Gameplay-tunable fields have sensible defaults. The caller (binary or game
/// plugin) can override any field before inserting the resource.
#[derive(Resource, Clone)]
pub struct Config {
    /// RON map file contents (pre-loaded string).
    pub map_ron: String,
    /// Filesystem path to the map RON file (for hot reload).
    ///
    /// When set, Cmd+R re-reads the map from this path, rebuilds `MapRes` and
    /// `ClientMap`, and logs the update. Left empty in production builds or
    /// when the map is baked into `map_ron`.
    pub map_path: String,
    /// Path to the sky RON config file (used to resolve `.pxi` asset paths).
    pub sky_path: String,
    /// Framebuffer width in pixels.
    pub screen_width: u32,
    /// Framebuffer height in pixels.
    pub screen_height: u32,
    /// Player movement speed in world units per second.
    pub move_speed: f32,
    /// Player manual turn speed in radians per second.
    pub turn_speed: f32,
    /// Damage dealt by the player's hitscan weapon per shot.
    pub hitscan_damage: u32,
    /// Maximum player health points.
    pub player_max_health: u32,
    /// Duration of the 180° quick-turn animation in seconds.
    /// The 90° side turn shares the same angular velocity, completing in half this time.
    pub quick_turn_duration_secs: f32,
    /// Duration of the death camera rotation toward the killer in seconds.
    pub death_turn_duration_secs: f32,
    /// Maximum red overlay density during the death fade (0.0–1.0).
    pub death_red_max_density: f32,
    /// Base intensity added per camera-shake hit.
    pub camera_shake_base_intensity: f32,
    /// Exponential decay rate for camera shake (higher = faster decay).
    pub camera_shake_decay_rate: f32,
    /// Intensity below which camera shake snaps to zero.
    pub camera_shake_threshold: f32,
    /// Who owns FPS combat simulation for enemies.
    pub authority_mode: FpsAuthorityMode,
}

/// Enemy authority model for the FPS plugin.
///
/// `LocalAuthority` is the single-player model: map-authored combat entities
/// spawn locally, tick AI locally, take local player damage, and render through
/// local enemy billboards.
///
/// `RemoteClient` is the multiplayer-client model: map geometry and static
/// scenery remain local, but combat enemies are rendered from replicated
/// network state and are not locally spawned, ticked, or damaged.
///
/// Set at startup. Runtime switching is not supported — entity and billboard
/// setup occurs once during initialization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FpsAuthorityMode {
    #[default]
    LocalAuthority,
    RemoteClient,
}

impl FpsAuthorityMode {
    #[must_use]
    const fn uses_local_combat(self) -> bool {
        matches!(self, Self::LocalAuthority)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            map_ron: String::new(),
            map_path: String::new(),
            sky_path: String::new(),
            screen_width: 160,
            screen_height: 144,
            move_speed: 2.0,
            turn_speed: 2.0,
            hitscan_damage: 37,
            player_max_health: 100,
            quick_turn_duration_secs: 0.4,
            death_turn_duration_secs: 0.45,
            death_red_max_density: 0.85,
            camera_shake_base_intensity: 3.0,
            camera_shake_decay_rate: 12.0,
            camera_shake_threshold: 0.3,
            authority_mode: FpsAuthorityMode::LocalAuthority,
        }
    }
}

/// Per-frame billboard scratch buffer for networked entities.
///
/// Populated by the multiplayer client each frame, consumed by the renderer.
/// Separated from `Config` so serialization/inspection of config doesn't
/// include transient per-frame state.
#[derive(Resource, Default)]
pub struct ExtraBillboards(pub Vec<Billboard>);

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

/// Ground fire hazards spawned when enemies die from fire.
#[derive(Resource, Default)]
pub struct GroundFires(pub Vec<GroundFire>);

/// Per-player cooldown for ground fire contact damage (SP only).
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct GroundFireContactHazardState {
    cooldown_remaining_secs: f32,
}

/// Bundled fire hazard state for the SP tick system (avoids param overflow).
#[derive(SystemParam)]
struct FireHazardState<'w> {
    burning_corpse_contact: ResMut<'w, BurningCorpseContactHazardState>,
    ground_fires: ResMut<'w, GroundFires>,
    gf_contact: ResMut<'w, GroundFireContactHazardState>,
}

#[derive(Resource)]
pub struct BloodShotSprites(pub BloodShotBillboardSprites);

#[derive(Resource)]
pub struct MosquitonSprites(pub MosquitonBillboardSprites);

/// Composed billboard sprites resource for Spidey enemies.
#[derive(Resource)]
pub struct SpideySprites(pub SpideyBillboardSprites);

#[derive(Resource)]
pub struct SpiderShotSprites(pub SpiderShotBillboardSprites);

struct FpMapEntitySetup {
    static_billboards: Vec<Billboard>,
    initial_enemy_billboards: Vec<Billboard>,
    initial_mosquiton_billboards: Vec<Billboard>,
    initial_spidey_billboards: Vec<Billboard>,
    enemies: Vec<Enemy>,
    mosquitons: Vec<Mosquiton>,
    spideys: Vec<Spidey>,
}

#[derive(SystemParam)]
struct RenderSources<'w> {
    static_bbs: Res<'w, StaticBillboards>,
    pairs: Res<'w, SpritePairs>,
    projectiles: Res<'w, Projectiles>,
    impacts: Res<'w, ProjectileImpacts>,
    blood_shot_sprites: Res<'w, BloodShotSprites>,
    spider_shot_sprites: Res<'w, SpiderShotSprites>,
    mosquiton_sprites: Res<'w, MosquitonSprites>,
    spidey_sprites: Res<'w, SpideySprites>,
    char_decals: Res<'w, CharDecals>,
    ground_fires: Res<'w, GroundFires>,
    ground_fire_vis: Res<'w, GroundFireVisualConfig>,
}

#[derive(SystemParam)]
struct ViewResources<'w> {
    textures: Res<'w, WallTextures>,
    camera: Res<'w, CameraRes>,
    map: Res<'w, MapRes>,
    palette: Res<'w, PaletteRes>,
    sky: Res<'w, Sky>,
    config: Res<'w, Config>,
    extra_bbs: Res<'w, ExtraBillboards>,
    health: Res<'w, PlayerHealth>,
    dead: Res<'w, PlayerDead>,
    death_view: Res<'w, DeathViewState>,
    camera_shake: Res<'w, CameraShakeState>,
    screen_particles: Res<'w, FpsScreenParticles>,
    screen_particle_config: Res<'w, ScreenParticleConfig>,
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

/// Marker component on the FPS raycasted view sprite entity.
///
/// External systems (e.g. map-view toggle) use this to find and toggle
/// visibility of the FPS view.
#[derive(Component)]
pub struct FpsViewSprite;

/// Active speed modifier on the player (e.g. web slow).
///
/// Ticked each frame in `tick_enemy_ai`. Callers (`fps_test`, multiplayer client)
/// read this to apply the modifier to player movement via
/// `apply_movement_with_modifier`.
#[derive(Resource, Default)]
pub struct PlayerSpeedModifier(pub Option<carcinisation_fps_core::movement::SpeedModifier>);

/// Resolved FP player intent. Integration layers can build this from any input source.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerIntent {
    pub move_delta: Vec2,
    pub turn_delta: f32,
    pub shoot_pressed: bool,
    pub quick_turn_pressed: bool,
}

/// Which kind of snap turn to perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnKind {
    /// 180° turn (B + Down).
    QuickTurn,
    /// 90° turn left (B + Left).
    SideTurnLeft,
    /// 90° turn right (B + Right).
    SideTurnRight,
}

/// Hold-to-select, release-to-commit chord state machine for snap/quick turns.
///
/// Flow:
/// 1. B `just_pressed` → `ChordMode { since, selected: None }`
/// 2. Direction `just_pressed` during chord mode within window → selects turn kind
/// 3. Selected direction `just_released` → **fire** and enter `BlockedUntilRelease`
/// 4. B released before direction release → cancel
/// 5. Window expires before direction release → cancel
/// 6. Directions already held when B is pressed are blocked from selection.
///
/// Movement/turn is suppressed while in `ChordMode`.
/// Priority when multiple directions pressed same frame: Down > Left > Right.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct TurnChordState {
    phase: TurnChordPhase,
}

impl TurnChordState {
    /// Whether the FSM is in chord mode (B held, awaiting direction or release).
    /// Callers should suppress movement/turn input while this returns `true`.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self.phase, TurnChordPhase::ChordMode { .. })
    }

    /// Whether the FSM is in AimMode (AimCommitment: B held past chord window).
    /// Body is locked, aim offset is active, fire is allowed.
    #[must_use]
    pub const fn is_aim_mode(&self) -> bool {
        matches!(self.phase, TurnChordPhase::AimMode)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum TurnChordPhase {
    #[default]
    Idle,
    /// B is held within the chord window. Direction press selects; direction release fires.
    ChordMode {
        since: f32,
        /// Latched selection. `None` until a valid direction is `just_pressed`.
        selected: Option<TurnKind>,
        /// Directions that were already held when B was pressed (blocked from selection).
        blocked_dirs: u8,
    },
    /// Chord fired or window expired; wait for all keys to release.
    BlockedUntilRelease,
    /// AimCommitment: B held past chord window. Body locked, aim offset active.
    /// Exits to Idle on B release.
    AimMode,
}

/// Raw button state for the turn chord resolver.
#[derive(Clone, Copy, Debug, Default)]
pub struct TurnChordInput {
    pub b_pressed: bool,
    pub b_just_pressed: bool,
    pub b_just_released: bool,
    pub down_pressed: bool,
    pub down_just_pressed: bool,
    pub down_just_released: bool,
    pub left_pressed: bool,
    pub left_just_pressed: bool,
    pub left_just_released: bool,
    pub right_pressed: bool,
    pub right_just_pressed: bool,
    pub right_just_released: bool,
    pub now_secs: f32,
}

/// Runtime state for a smooth quick-turn or side-turn animation.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct QuickTurnState {
    remaining_radians: f32,
    /// Total arc of the turn in radians (constant for the duration).
    total_radians: f32,
    /// Radians per second for the active turn.
    speed: f32,
    /// +1.0 for left, -1.0 for right.
    direction: f32,
    /// Accumulated aim offset in radians (AimCommitment only).
    /// Reset to 0.0 when AimMode exits.
    pub aim_offset: f32,
}

impl QuickTurnState {
    /// Returns `true` while a turn animation is playing.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.remaining_radians > 0.0
    }

    /// Total arc of the current (or last) snap turn.
    #[must_use]
    pub const fn total_radians(&self) -> f32 {
        self.total_radians
    }

    /// Remaining arc of the current snap turn.
    #[must_use]
    pub const fn remaining_radians(&self) -> f32 {
        self.remaining_radians
    }

    /// Turn direction: +1.0 = left, -1.0 = right.
    #[must_use]
    pub const fn direction(&self) -> f32 {
        self.direction
    }
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

const DIR_DOWN: u8 = 1;
const DIR_LEFT: u8 = 2;
const DIR_RIGHT: u8 = 4;

/// Bitmask of directions that were pre-held (pressed but NOT `just_pressed`).
const fn dir_preheld_mask(input: &TurnChordInput) -> u8 {
    let mut m = 0u8;
    if input.down_pressed && !input.down_just_pressed {
        m |= DIR_DOWN;
    }
    if input.left_pressed && !input.left_just_pressed {
        m |= DIR_LEFT;
    }
    if input.right_pressed && !input.right_just_pressed {
        m |= DIR_RIGHT;
    }
    m
}

/// Identify which direction was just pressed and not blocked. Priority: Down > Left > Right.
const fn identify_just_pressed_direction(input: &TurnChordInput, blocked: u8) -> Option<TurnKind> {
    if input.down_just_pressed && blocked & DIR_DOWN == 0 {
        return Some(TurnKind::QuickTurn);
    }
    if input.left_just_pressed && blocked & DIR_LEFT == 0 {
        return Some(TurnKind::SideTurnLeft);
    }
    if input.right_just_pressed && blocked & DIR_RIGHT == 0 {
        return Some(TurnKind::SideTurnRight);
    }
    None
}

/// Whether the selected turn kind's direction key was just released.
const fn selected_direction_just_released(input: &TurnChordInput, kind: TurnKind) -> bool {
    match kind {
        TurnKind::QuickTurn => input.down_just_released,
        TurnKind::SideTurnLeft => input.left_just_released,
        TurnKind::SideTurnRight => input.right_just_released,
    }
}

/// Whether any direction key relevant to chords is pressed.
const fn any_dir_pressed(input: &TurnChordInput) -> bool {
    input.down_pressed || input.left_pressed || input.right_pressed
}

/// Resolve the hold-to-select, release-to-commit chord state machine.
///
/// Returns `Some(TurnKind)` on the frame the selected direction is
/// **released** while B is still held within the chord window. Directions
/// held before B are blocked. Window expiry or B release cancels.
#[must_use]
pub fn resolve_turn_chord(
    input: &TurnChordInput,
    state: &mut TurnChordState,
    aim_commitment: bool,
) -> Option<TurnKind> {
    match state.phase {
        TurnChordPhase::Idle => {
            if input.b_just_pressed {
                let blocked_dirs = dir_preheld_mask(input);
                // Same-frame direction press → select (but don't fire yet).
                let selected = identify_just_pressed_direction(input, blocked_dirs);
                state.phase = TurnChordPhase::ChordMode {
                    since: input.now_secs,
                    selected,
                    blocked_dirs,
                };
            }
            None
        }
        TurnChordPhase::ChordMode {
            since,
            mut selected,
            blocked_dirs,
        } => {
            // Window expired → AimMode (if AimCommitment) or cancel.
            if input.now_secs - since > CHORD_WINDOW_SECS {
                state.phase = if aim_commitment {
                    TurnChordPhase::AimMode
                } else {
                    TurnChordPhase::BlockedUntilRelease
                };
                return None;
            }

            // Accept direction selection if not already selected.
            if selected.is_none() {
                selected = identify_just_pressed_direction(input, blocked_dirs);
            }

            // Selected direction released → fire.
            if let Some(kind) = selected
                && selected_direction_just_released(input, kind)
            {
                state.phase = TurnChordPhase::BlockedUntilRelease;
                return Some(kind);
            }

            // B released before direction release → cancel.
            if input.b_just_released {
                state.phase = TurnChordPhase::Idle;
                return None;
            }

            state.phase = TurnChordPhase::ChordMode {
                since,
                selected,
                blocked_dirs,
            };
            None
        }
        TurnChordPhase::BlockedUntilRelease => {
            if !input.b_pressed && !any_dir_pressed(input) {
                state.phase = TurnChordPhase::Idle;
            }
            None
        }
        TurnChordPhase::AimMode => {
            // B released → exit aim, return to idle.
            if !input.b_pressed {
                state.phase = TurnChordPhase::Idle;
            }
            None
        }
    }
}

/// Start a snap turn animation for the given kind.
///
/// Quick turn = 180° left. Side turns = 90° left/right.
/// Delegates to `fps_core::snap_turn_params` for the shared math.
pub fn request_snap_turn(state: &mut QuickTurnState, kind: TurnKind, config: &Config) {
    if state.remaining_radians > 0.0 {
        return;
    }
    let core_kind = match kind {
        TurnKind::QuickTurn => carcinisation_fps_core::SnapTurnKind::QuickTurn,
        TurnKind::SideTurnLeft => carcinisation_fps_core::SnapTurnKind::Left,
        TurnKind::SideTurnRight => carcinisation_fps_core::SnapTurnKind::Right,
    };
    let params =
        carcinisation_fps_core::snap_turn_params(core_kind, config.quick_turn_duration_secs);
    state.remaining_radians = params.remaining_radians;
    state.total_radians = params.total_radians;
    state.speed = params.speed;
    state.direction = params.direction;
}

/// Advance the active quick-turn animation by `dt` seconds.
/// Delegates to `fps_core::tick_snap_turn` for the shared math.
pub fn tick_quick_turn(camera: &mut Camera, state: &mut QuickTurnState, dt: f32) {
    carcinisation_fps_core::tick_snap_turn(
        &mut camera.angle,
        &mut state.remaining_radians,
        state.speed,
        state.direction,
        dt,
    );
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
pub fn tick_death_view(camera: &mut Camera, state: &mut DeathViewState, dt: f32, config: &Config) {
    if !state.active {
        return;
    }

    state.elapsed = (state.elapsed + dt).min(config.death_turn_duration_secs);
    let t = (state.elapsed / config.death_turn_duration_secs).clamp(0.0, 1.0);
    let delta = signed_angle_delta(state.start_angle, state.target_angle);
    #[allow(clippy::suboptimal_flops)]
    let angle = state.start_angle + delta * t;
    camera.angle = angle.rem_euclid(std::f32::consts::TAU);
}

#[must_use]
pub fn death_red_density(state: &DeathViewState, config: &Config) -> f32 {
    if !state.active {
        return 0.0;
    }
    let t = (state.elapsed / config.death_turn_duration_secs).clamp(0.0, 1.0);
    (t * config.death_red_max_density).clamp(0.0, 1.0)
}

fn signed_angle_delta(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Reinforce an FP camera shake, matching ORS' additive hit shake model.
pub fn request_camera_shake(state: &mut CameraShakeState, config: &Config) {
    state.intensity += config.camera_shake_base_intensity;
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
    config: &Config,
) {
    if state.intensity < config.camera_shake_threshold {
        state.intensity = 0.0;
        state.current_offset = IVec2::ZERO;
        return;
    }

    let angle = angle_sample.clamp(0.0, 1.0) * std::f32::consts::TAU;
    #[allow(clippy::suboptimal_flops)]
    let magnitude = state.intensity * (0.5 + 0.5 * magnitude_sample.clamp(0.0, 1.0));
    let offset = Vec2::new(angle.cos() * magnitude, angle.sin() * magnitude).round();
    state.current_offset = IVec2::new(offset.x as i32, offset.y as i32);
    state.intensity *= (-config.camera_shake_decay_rate * dt).exp();
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
        #[cfg(feature = "hot_reload")]
        app.add_plugins(carcinisation_core::dev_reload::DevReloadPlugin);

        app.init_resource::<ExtraBillboards>();
        app.init_resource::<ShootRequest>();
        app.init_resource::<AttackInput>();
        app.init_resource::<AttackLoadout>();
        app.init_resource::<CharDecals>();
        app.init_resource::<BurningCorpseContactHazardState>();
        app.init_resource::<GroundFires>();
        app.init_resource::<GroundFireContactHazardState>();
        app.insert_resource(GroundFireVisualConfig::load());
        app.insert_resource(carcinisation_fps_core::burning::load_config());
        app.insert_resource(carcinisation_fps_core::FpsMovementConfig::load());
        app.insert_resource(carcinisation_fps_core::FpsCombatConfig::load());
        app.insert_resource(carcinisation_fps_core::FpsVisualConfig::load());
        // Eager-init palette config so parse errors surface at plugin startup.
        crate::avatar_palette::colour_groups();
        let flame_cfg = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
        app.insert_resource(PlayerAttackState::new(flame_cfg));
        app.insert_resource(flame_cfg);
        app.init_resource::<QuickTurnState>();
        app.init_resource::<TurnChordState>();
        app.init_resource::<DeathViewState>();
        app.init_resource::<CameraShakeState>();
        app.insert_resource(ScreenParticleConfig::load());
        app.init_resource::<FpsScreenParticles>();

        #[cfg(feature = "hot_reload")]
        {
            carcinisation_core::watch_config!(app, "assets/config/attacks/player_flamethrower.ron");
            carcinisation_core::watch_config!(app, "assets/config/status/burning.ron");
            carcinisation_core::watch_config!(app, "assets/config/fp/movement.ron");
            carcinisation_core::watch_config!(app, "assets/config/fp/combat.ron");
            carcinisation_core::watch_config!(app, "assets/config/fp/visuals.ron");
            carcinisation_core::watch_config!(app, "assets/config/fp/screen_particles.ron");
            carcinisation_core::watch_config!(app, "assets/config/attacks/ground_fire.ron");
            // Sprite PXI files are not auto-polled (10 stat() calls per poll
            // cause frame stutters). Use Cmd+R for manual sprite reload.
        }

        app.add_systems(
            Update,
            (
                #[cfg(feature = "hot_reload")]
                reload_flamethrower_config.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_burn_config.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_movement_config.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_combat_config.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_visual_config.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_screen_particles_config.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_ground_fire_visual.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_map.in_set(Systems),
                #[cfg(feature = "hot_reload")]
                reload_attack_sprites.in_set(Systems),
                apply_quick_turn_animation.in_set(Systems),
                handle_shooting.in_set(Systems),
                tick_enemy_ai.in_set(Systems).after(handle_shooting),
                apply_death_view.in_set(Systems),
                tick_projectile_impact_effects.in_set(Systems),
                tick_camera_shake_effect.in_set(Systems),
                update_fps_screen_particles
                    .in_set(Systems)
                    .before(update_fp_view),
                apply_view_bob
                    .in_set(Systems)
                    .after(handle_shooting)
                    .after(apply_death_view),
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

    let procedural_alive = make_enemy_sprite(24, 2);
    let procedural_death = make_death_sprite(24, 1);
    let sprite_pairs: Vec<(CxImage, CxImage)> = vec![(procedural_alive, procedural_death)];
    let mosquiton_sprites = make_mosquiton_billboard_sprites()
        .expect("embedded Mosquiton composed billboard assets should resolve");
    let blood_shot_sprites =
        make_blood_shot_billboard_sprites().expect("embedded blood shot assets should resolve");
    let spider_shot_sprites =
        make_spider_shot_billboard_sprites().expect("embedded spider shot assets should resolve");
    let spidey_sprites = make_spidey_billboard_sprites()
        .expect("embedded Spidey composed billboard assets should resolve");

    let combat_config = carcinisation_fps_core::FpsCombatConfig::load();
    let entity_setup = build_map_entity_setup(
        &map_data,
        config.authority_mode,
        &sprite_pairs,
        &mosquiton_sprites,
        &spidey_sprites,
        &combat_config,
    );

    for enemy in &entity_setup.enemies {
        commands.spawn((enemy.clone(), EnemySpriteIndex(0)));
    }
    for mosquiton in &entity_setup.mosquitons {
        commands.spawn(mosquiton.clone());
    }
    for spidey in &entity_setup.spideys {
        commands.spawn(spidey.clone());
    }

    let all_bbs: Vec<Billboard> = entity_setup
        .static_billboards
        .iter()
        .cloned()
        .chain(entity_setup.initial_enemy_billboards.iter().cloned())
        .chain(entity_setup.initial_mosquiton_billboards.iter().cloned())
        .chain(entity_setup.initial_spidey_billboards.iter().cloned())
        .collect();
    let sky_ron = std::fs::read_to_string(&config.sky_path)
        .unwrap_or_else(|e| panic!("failed to read sky RON {}: {}", config.sky_path, e));
    let workspace_root = std::env::current_dir()
        .unwrap_or_else(|e| panic!("failed to get current dir: {e}"))
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
        FpsViewSprite,
    ));

    commands.insert_resource(SpriteHandle(handle));
    commands.insert_resource(WallTextures(textures));
    commands.insert_resource(CameraRes(camera));
    commands.insert_resource(MapRes(map));
    commands.insert_resource(PaletteRes(palette));
    commands.insert_resource(sky);
    commands.insert_resource(StaticBillboards(entity_setup.static_billboards));
    commands.insert_resource(SpritePairs(sprite_pairs));
    commands.insert_resource(Projectiles(Vec::new()));
    commands.insert_resource(ProjectileImpacts(Vec::new()));
    commands.insert_resource(CharDecals::default());
    commands.insert_resource(BloodShotSprites(blood_shot_sprites));
    commands.insert_resource(SpiderShotSprites(spider_shot_sprites));
    commands.insert_resource(MosquitonSprites(mosquiton_sprites));
    commands.insert_resource(SpideySprites(spidey_sprites));
    commands.insert_resource(PlayerAttackSprites::load());
    commands.insert_resource(PlayerHealth(config.player_max_health));
    commands.insert_resource(PlayerDead(false));
    commands.insert_resource(PlayerSpeedModifier::default());
    commands.insert_resource(Active);

    info!("First-person mode initialized");
}

fn build_map_entity_setup(
    map_data: &MapData,
    authority_mode: FpsAuthorityMode,
    sprite_pairs: &[(CxImage, CxImage)],
    mosquiton_sprites: &MosquitonBillboardSprites,
    spidey_sprites: &SpideyBillboardSprites,
    combat_config: &carcinisation_fps_core::FpsCombatConfig,
) -> FpMapEntitySetup {
    let mut setup = FpMapEntitySetup {
        static_billboards: Vec::new(),
        initial_enemy_billboards: Vec::new(),
        initial_mosquiton_billboards: Vec::new(),
        initial_spidey_billboards: Vec::new(),
        enemies: Vec::new(),
        mosquitons: Vec::new(),
        spideys: Vec::new(),
    };

    for spawn in &map_data.entities {
        let pos = Vec2::new(spawn.x, spawn.y);
        match &spawn.kind {
            EntityKind::Pillar {
                color,
                width,
                height,
            } => {
                setup.static_billboards.push(Billboard {
                    position: pos,
                    height: 0.0,
                    world_height: 1.0,
                    sprite: std::sync::Arc::new(make_pillar_sprite(*width, *height, *color)),
                    flip_x: false,
                    palette_variant: None,
                });
            }
            EntityKind::Enemy { health, speed, .. }
            | EntityKind::SpriteEnemy { health, speed, .. } => {
                if authority_mode.uses_local_combat() {
                    let enemy = Enemy::new(pos, *health, *speed);
                    setup.initial_enemy_billboards.push(billboard_from_enemy(
                        &enemy,
                        0,
                        sprite_pairs,
                    ));
                    setup.enemies.push(enemy);
                }
            }
            EntityKind::Mosquiton { health, speed } => {
                if authority_mode.uses_local_combat() {
                    let config = MosquitonConfig {
                        health: *health,
                        move_speed: *speed,
                        shoot_cue_secs: mosquiton_sprites.shoot_cue_elapsed_secs,
                        ..Default::default()
                    };
                    let mosquiton = Mosquiton::new(pos, config);
                    setup
                        .initial_mosquiton_billboards
                        .push(billboard_from_mosquiton(&mosquiton, mosquiton_sprites));
                    setup.mosquitons.push(mosquiton);
                }
            }
            EntityKind::Spidey { health, speed } => {
                if authority_mode.uses_local_combat() {
                    let config = SpideyConfig {
                        health: *health,
                        ..SpideyConfig::from_combat_config(combat_config)
                    }
                    .with_authored_speed(*speed);
                    let spidey = Spidey::new(pos, config);
                    setup
                        .initial_spidey_billboards
                        .push(billboard_from_spidey(&spidey, spidey_sprites));
                    setup.spideys.push(spidey);
                }
            }
            EntityKind::Pickup { .. } => {
                // Pickups are server-authoritative — the client ignores them
                // during local map setup. The server spawns `NetPickup` entities
                // for multiplayer, and the FPS client renders them via
                // `queue_pickup_billboards`.
            }
        }
    }

    setup
}

fn apply_quick_turn_animation(
    time: Res<Time>,
    mut camera: ResMut<CameraRes>,
    mut quick_turn: ResMut<QuickTurnState>,
    config: Res<Config>,
) {
    let dt = time.delta_secs();
    if config.authority_mode == FpsAuthorityMode::RemoteClient {
        // In multiplayer, prediction owns the camera angle. Only tick
        // the QuickTurnState timer (for input suppression) without
        // rotating the camera — sync_camera_from_net_player reads
        // PredictedRenderState instead.
        if quick_turn.remaining_radians > 0.0 {
            let step = (quick_turn.speed * dt)
                .min(quick_turn.remaining_radians)
                .max(0.0);
            quick_turn.remaining_radians -= step;
        }
    } else {
        tick_quick_turn(&mut camera.0, &mut quick_turn, dt);
    }
}

fn apply_death_view(
    time: Res<Time>,
    mut camera: ResMut<CameraRes>,
    mut death_view: ResMut<DeathViewState>,
    dead: Res<PlayerDead>,
    config: Res<Config>,
) {
    if dead.0 {
        tick_death_view(&mut camera.0, &mut death_view, time.delta_secs(), &config);
    }
}

/// Apply walk view bob to the camera. Zeroed when dead to avoid bobbing
/// death camera. Runs after `handle_shooting` computes `view_bob`.
fn apply_view_bob(
    mut camera: ResMut<CameraRes>,
    attack_state: Res<PlayerAttackState>,
    dead: Res<PlayerDead>,
    visual_config: Res<carcinisation_fps_core::FpsVisualConfig>,
) {
    camera.0.view_bob = if dead.0 { 0.0 } else { attack_state.view_bob };
    camera.0.view_bob_near = visual_config.view_bob_near;
    camera.0.view_bob_mid = visual_config.view_bob_mid;
}

fn tick_projectile_impact_effects(time: Res<Time>, mut impacts: ResMut<ProjectileImpacts>) {
    tick_projectile_impacts(&mut impacts.0, time.delta_secs());
}

fn tick_camera_shake_effect(
    time: Res<Time>,
    mut shake: ResMut<CameraShakeState>,
    config: Res<Config>,
) {
    tick_camera_shake(
        &mut shake,
        time.delta_secs(),
        rand::random::<f32>(),
        rand::random::<f32>(),
        &config,
    );
}

/// Bundled enemy queries for the SP tick system (avoids param overflow).
#[derive(SystemParam)]
struct EnemyQueries<'w, 's> {
    enemies: Query<'w, 's, (Entity, &'static mut Enemy)>,
    mosquitons: Query<'w, 's, (Entity, &'static mut Mosquiton)>,
    spideys: Query<'w, 's, (Entity, &'static mut Spidey)>,
}

#[allow(clippy::too_many_arguments)]
fn tick_enemy_ai(
    time: Res<Time>,
    camera: Res<CameraRes>,
    map: Res<MapRes>,
    mut enemies: EnemyQueries,
    mut projectiles: ResMut<Projectiles>,
    mut impacts: ResMut<ProjectileImpacts>,
    mut health: ResMut<PlayerHealth>,
    mut dead: ResMut<PlayerDead>,
    mut death_view: ResMut<DeathViewState>,
    mut camera_shake: ResMut<CameraShakeState>,
    attack_state: Res<PlayerAttackState>,
    mut fire_hazard: FireHazardState,
    mut commands: Commands,
    config: Res<Config>,
    burn_config: Res<carcinisation_fps_core::BurnConfig>,
    mut speed_modifier: ResMut<PlayerSpeedModifier>,
) {
    if !config.authority_mode.uses_local_combat() {
        return;
    }

    let dt = time.delta_secs();

    // Tick speed modifier (e.g. web slow) regardless of dead state.
    // Movement distance is approximated from speed * dt when the player is
    // SpeedModifier is ticked by the movement owner (fps_test, MP client),
    // not here — the plugin doesn't know the player's movement distance.

    if dead.0 {
        return;
    }

    let player_pos = camera.0.position;

    // Tick enemies and collect dead entities for despawning.
    let mut dead_enemies = Vec::new();
    for (entity, mut enemy) in &mut enemies.enemies {
        if let Some(proj) = tick_single_enemy(&mut enemy, player_pos, &map.0, dt) {
            projectiles.0.push(proj);
        }

        // Tick burn state (exposure was applied in handle_shooting).
        if enemy.is_alive() {
            let result = carcinisation_fps_core::tick_burning(
                &mut enemy.burn_state,
                &burn_config,
                dt,
                false,
            );
            if result.damage > 0 {
                enemy.take_damage_from(
                    result.damage,
                    carcinisation_fps_core::DamageKind::Fire,
                    attack_state.shared().burning_corpse_duration_secs,
                );
            }
        } else if enemy.burn_state.is_burning() {
            carcinisation_fps_core::burning::extinguish(&mut enemy.burn_state);
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
    for (entity, mut mosquiton) in &mut enemies.mosquitons {
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
                &config,
            );
        }

        // Tick burn state for mosquitons.
        if mosquiton.is_alive() {
            let result = carcinisation_fps_core::tick_burning(
                &mut mosquiton.burn_state,
                &burn_config,
                dt,
                false,
            );
            if result.damage > 0 {
                mosquiton.take_damage_from(
                    result.damage,
                    carcinisation_fps_core::DamageKind::Fire,
                    attack_state.shared().burning_corpse_duration_secs,
                );
            }
        } else if mosquiton.burn_state.is_burning() {
            carcinisation_fps_core::burning::extinguish(&mut mosquiton.burn_state);
        }

        if matches!(mosquiton.state, MosquitonState::Dead) {
            dead_mosquitons.push(entity);
        }
    }
    for entity in dead_mosquitons {
        commands.entity(entity).despawn();
    }

    // Tick spideys and collect dead entities for despawning.
    let mut dead_spideys = Vec::new();
    for (entity, mut spidey) in &mut enemies.spideys {
        let (proj, dmg) = tick_single_spidey(&mut spidey, player_pos, &map.0, dt);
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
                &config,
            );
        }

        // Tick burn state for spideys.
        if spidey.is_alive() {
            let result = carcinisation_fps_core::tick_burning(
                &mut spidey.burn_state,
                &burn_config,
                dt,
                false,
            );
            if result.damage > 0 {
                spidey.take_damage_from(
                    result.damage,
                    carcinisation_fps_core::DamageKind::Fire,
                    attack_state.shared().burning_corpse_duration_secs,
                );
            }
        } else if spidey.burn_state.is_burning() {
            carcinisation_fps_core::burning::extinguish(&mut spidey.burn_state);
        }

        if matches!(spidey.state, SpideyState::Dead) {
            dead_spideys.push(entity);
        }
    }
    for entity in dead_spideys {
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
        &config,
    );

    // Apply web slow effect if a WebShot hit the player this tick.
    if let Some(slow) = projectile_result.slow_effect {
        use carcinisation_fps_core::movement::SpeedModifier;
        match &mut speed_modifier.0 {
            Some(existing) => existing.refresh(slow.multiplier, slow.duration),
            None => speed_modifier.0 = Some(SpeedModifier::new(slow.multiplier, slow.duration)),
        }
    }

    // Collect burning corpses from remaining enemies, mosquitons, and spideys.
    let mut burning_corpses = Vec::new();
    for (_, enemy) in enemies.enemies.iter() {
        if matches!(enemy.state, EnemyState::BurningCorpse { .. }) {
            burning_corpses.push(enemy.position);
        }
    }
    for (_, mosquiton) in enemies.mosquitons.iter() {
        if matches!(mosquiton.state, MosquitonState::BurningCorpse { .. }) {
            burning_corpses.push(mosquiton.position);
        }
    }
    for (_, spidey) in enemies.spideys.iter() {
        if matches!(spidey.state, SpideyState::BurningCorpse { .. }) {
            burning_corpses.push(spidey.position);
        }
    }

    // Spawn ground fires from new burning corpses.
    let gf_config = GroundFireConfig::default();
    for &pos in &burning_corpses {
        try_spawn_ground_fire(&mut fire_hazard.ground_fires.0, pos, &gf_config);
    }

    // Tick ground fire lifetimes.
    tick_ground_fires(&mut fire_hazard.ground_fires.0, dt);

    // Burning corpse contact damage (player proximity to corpses).
    let burning_corpse_contact_result = tick_burning_corpse_contact_damage(
        &camera.0,
        &burning_corpses,
        attack_state.shared(),
        &mut fire_hazard.burning_corpse_contact,
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
        &config,
    );

    // Ground fire contact damage (separate cooldown, lower DPS).
    let mut gf_state = GroundFireContactState {
        cooldown_remaining_secs: fire_hazard.gf_contact.cooldown_remaining_secs,
    };
    let gf_contact_result = ground_fire_contact_damage(
        camera.0.position,
        &fire_hazard.ground_fires.0,
        &gf_config,
        &mut gf_state,
        dt,
    );
    fire_hazard.gf_contact.cooldown_remaining_secs = gf_state.cooldown_remaining_secs;
    apply_player_damage(
        &mut health.0,
        &mut dead.0,
        &mut death_view,
        &mut camera_shake,
        &camera.0,
        gf_contact_result.player_damage,
        gf_contact_result.damage_source,
        &config,
    );

    // Crossfire: merge burning corpse + ground fire positions for enemy damage.
    let mut all_fire_positions = burning_corpses;
    all_fire_positions.extend(fire_hazard.ground_fires.0.iter().map(|f| f.position));
    tick_burning_corpse_crossfire_query(
        &mut enemies.enemies,
        &mut enemies.mosquitons,
        &mut enemies.spideys,
        &all_fire_positions,
        attack_state.shared(),
        &burn_config,
        dt,
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
    config: &carcinisation_fps_core::PlayerFlamethrowerConfig,
    state: &mut BurningCorpseContactHazardState,
    dt: f32,
) -> BurningCorpseContactDamageResult {
    state.cooldown_remaining_secs = (state.cooldown_remaining_secs - dt).max(0.0);

    if config.burning_corpse_contact_damage == 0 || config.burning_corpse_contact_radius <= 0.0 {
        return BurningCorpseContactDamageResult::default();
    }

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

// ── Hot reload systems ────────────────────────────────────────────────────

/// Reload `PlayerFlamethrowerConfig` from disk on Cmd+R.
///
/// Updates both the `Res<PlayerFlamethrowerConfig>` and the cached copy inside
/// `PlayerAttackState` so gameplay systems see the new values immediately.
#[cfg(feature = "hot_reload")]
fn reload_flamethrower_config(
    events: Option<bevy::prelude::MessageReader<carcinisation_core::dev_reload::DevReloadRequest>>,
    mut flame_res: bevy::prelude::ResMut<carcinisation_fps_core::PlayerFlamethrowerConfig>,
    mut attack_state: bevy::prelude::ResMut<PlayerAttackState>,
) {
    let Some(mut events) = events else { return };
    // Drain all events and reload only once.
    if events.read().count() == 0 {
        return;
    }
    let reloaded: carcinisation_fps_core::PlayerFlamethrowerConfig =
        carcinisation_core::ron_config!("assets/config/attacks/player_flamethrower.ron");
    *flame_res = reloaded;
    attack_state.update_shared(reloaded);
    bevy::log::info!(
        "Reloaded PlayerFlamethrowerConfig from assets/config/attacks/player_flamethrower.ron"
    );
}

// Reload `BurnConfig` from disk on Cmd+R.
#[cfg(feature = "hot_reload")]
carcinisation_core::reload_ron_system!(
    reload_burn_config,
    carcinisation_fps_core::BurnConfig,
    "assets/config/status/burning.ron"
);

// Reload `FpsMovementConfig` from disk on Cmd+R.
#[cfg(feature = "hot_reload")]
carcinisation_core::reload_ron_system!(
    reload_movement_config,
    carcinisation_fps_core::FpsMovementConfig,
    "assets/config/fp/movement.ron"
);

// Reload `FpsCombatConfig` from disk on Cmd+R.
#[cfg(feature = "hot_reload")]
carcinisation_core::reload_ron_system!(
    reload_combat_config,
    carcinisation_fps_core::FpsCombatConfig,
    "assets/config/fp/combat.ron"
);

// Reload `FpsVisualConfig` from disk on Cmd+R.
#[cfg(feature = "hot_reload")]
carcinisation_core::reload_ron_system!(
    reload_visual_config,
    carcinisation_fps_core::FpsVisualConfig,
    "assets/config/fp/visuals.ron"
);

// Reload `ScreenParticleConfig` from disk on Cmd+R.
#[cfg(feature = "hot_reload")]
carcinisation_core::reload_ron_system!(
    reload_screen_particles_config,
    ScreenParticleConfig,
    "assets/config/fp/screen_particles.ron",
    |config: &ScreenParticleConfig| config.validate_or_panic()
);

// Reload `GroundFireVisualConfig` from disk on Cmd+R.
#[cfg(feature = "hot_reload")]
carcinisation_core::reload_ron_system!(
    reload_ground_fire_visual,
    GroundFireVisualConfig,
    "assets/config/attacks/ground_fire.ron"
);

/// Reload the FPS map from disk on Cmd+R.
///
/// Re-reads the map RON file from `Config.map_path` and rebuilds `MapRes`.
/// Geometry and collision data are reconstructed; entities are NOT re-spawned.
/// Skipped when `map_path` is empty (production builds where the map is baked
/// into `map_ron`).
///
/// Downstream systems (e.g. client prediction) should detect the `MapRes`
/// change and update their own derived maps (e.g. `ClientMap`).
#[cfg(feature = "hot_reload")]
fn reload_map(
    events: Option<bevy::prelude::MessageReader<carcinisation_core::dev_reload::DevReloadRequest>>,
    config: bevy::prelude::Res<Config>,
    map_res: Option<bevy::prelude::ResMut<MapRes>>,
) {
    let Some(mut events) = events else { return };
    if events.read().count() == 0 {
        return;
    }
    if config.map_path.is_empty() {
        return;
    }
    let body = match std::fs::read_to_string(&config.map_path) {
        Ok(b) => b,
        Err(e) => {
            bevy::log::warn!("Map reload: failed to read {}: {e}", config.map_path);
            return;
        }
    };
    let map_data =
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| MapData::from_ron(&body))) {
            Ok(Ok(data)) => data,
            Ok(Err(e)) => {
                bevy::log::warn!("Map reload: parse error in {}: {e}", config.map_path);
                return;
            }
            Err(_) => {
                bevy::log::warn!("Map reload: panic while parsing {}", config.map_path);
                return;
            }
        };
    if let Some(mut mr) = map_res {
        mr.0 = map_data.to_map();
    }
    bevy::log::warn!(
        "Map reloaded from {} — geometry/collision rebuilt, entities NOT re-spawned",
        config.map_path
    );
}

/// Reload all FPS attack sprites from disk.
///
/// Catches panics from corrupt/incompatible filesystem data and keeps
/// the previous sprites if loading fails.
#[cfg(feature = "hot_reload")]
fn reload_attack_sprites(
    events: Option<bevy::prelude::MessageReader<carcinisation_core::dev_reload::DevReloadRequest>>,
    sprites: Option<bevy::prelude::ResMut<PlayerAttackSprites>>,
) {
    let Some(mut events) = events else { return };
    let Some(mut sprites) = sprites else { return };
    if events.read().next().is_none() {
        return;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(PlayerAttackSprites::load)) {
        Ok(reloaded) => {
            *sprites = reloaded;
            bevy::log::info!("Reloaded PlayerAttackSprites");
        }
        Err(_) => {
            bevy::log::warn!("PlayerAttackSprites reload failed, keeping previous sprites");
        }
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
    config: &carcinisation_fps_core::PlayerFlamethrowerConfig,
    burn_config: &carcinisation_fps_core::BurnConfig,
    dt: f32,
) {
    if burning_corpses.is_empty() {
        return;
    }

    let radius = config.burning_corpse_contact_radius;

    for mosquiton in mosquitons.iter_mut() {
        if !mosquiton.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(mosquiton.position, burning_corpses, radius).is_some() {
            carcinisation_fps_core::apply_exposure(
                &mut mosquiton.burn_state,
                burn_config,
                burn_config.ground_fire_exposure_per_sec,
                dt,
            );
        }
    }

    for enemy in enemies.iter_mut() {
        if !enemy.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(enemy.position, burning_corpses, radius).is_some() {
            carcinisation_fps_core::apply_exposure(
                &mut enemy.burn_state,
                burn_config,
                burn_config.ground_fire_exposure_per_sec,
                dt,
            );
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
    spidey_q: &mut Query<(Entity, &mut Spidey)>,
    burning_corpses: &[Vec2],
    config: &carcinisation_fps_core::PlayerFlamethrowerConfig,
    burn_config: &carcinisation_fps_core::BurnConfig,
    dt: f32,
) {
    if burning_corpses.is_empty() {
        return;
    }

    let radius = config.burning_corpse_contact_radius;

    for (_, mut mosquiton) in mosquiton_q.iter_mut() {
        if !mosquiton.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(mosquiton.position, burning_corpses, radius).is_some() {
            carcinisation_fps_core::apply_exposure(
                &mut mosquiton.burn_state,
                burn_config,
                burn_config.ground_fire_exposure_per_sec,
                dt,
            );
        }
    }

    for (_, mut enemy) in enemy_q.iter_mut() {
        if !enemy.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(enemy.position, burning_corpses, radius).is_some() {
            carcinisation_fps_core::apply_exposure(
                &mut enemy.burn_state,
                burn_config,
                burn_config.ground_fire_exposure_per_sec,
                dt,
            );
        }
    }

    for (_, mut spidey) in spidey_q.iter_mut() {
        if !spidey.is_alive() {
            continue;
        }
        if closest_burning_corpse_to(spidey.position, burning_corpses, radius).is_some() {
            carcinisation_fps_core::apply_exposure(
                &mut spidey.burn_state,
                burn_config,
                burn_config.ground_fire_exposure_per_sec,
                dt,
            );
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
    config: &Config,
) {
    if *dead || damage == 0 {
        return;
    }

    request_camera_shake(camera_shake, config);
    *health = health.saturating_sub(damage);
    if *health == 0 {
        *dead = true;
        if let Some(source) = damage_source {
            request_death_view(death_view, camera, source);
        }
        info!("Player died!");
    }
}

/// Bundled attack resources to stay within Bevy's 16-param system limit.
#[derive(bevy::ecs::system::SystemParam)]
struct AttackResources<'w> {
    input: ResMut<'w, AttackInput>,
    loadout: ResMut<'w, AttackLoadout>,
    state: ResMut<'w, PlayerAttackState>,
    sprites: Res<'w, PlayerAttackSprites>,
    burn_config: Res<'w, carcinisation_fps_core::BurnConfig>,
    visual_config: Res<'w, carcinisation_fps_core::FpsVisualConfig>,
}

#[allow(clippy::too_many_arguments)]
fn handle_shooting(
    camera: Res<CameraRes>,
    map: Res<MapRes>,
    config: Res<Config>,
    time: Res<Time>,
    mut enemy_q: Query<(Entity, &mut Enemy)>,
    mut mosquiton_q: Query<(Entity, &mut Mosquiton)>,
    mut spidey_q: Query<(Entity, &mut Spidey)>,
    mut projectiles: ResMut<Projectiles>,
    mut impacts: ResMut<ProjectileImpacts>,
    mut char_decals: ResMut<CharDecals>,
    dead: Res<PlayerDead>,
    mut shoot: ResMut<ShootRequest>,
    mut attack: AttackResources,
    quick_turn: Res<QuickTurnState>,
    mut prev_camera_angle: Local<f32>,
) {
    // Derive aim_turn_velocity from camera angle change so the flamethrower
    // chain bends during turns (whip effect).
    if attack.input.aim_turn_velocity.abs() <= f32::EPSILON {
        attack.input.aim_turn_velocity = carcinisation_fps_core::angular_velocity_clamped(
            camera.0.angle,
            *prev_camera_angle,
            time.delta_secs(),
        );
    }
    *prev_camera_angle = camera.0.angle;

    if dead.0 {
        shoot.0 = false;
        attack.input.clear_edges();
        return;
    }

    if !config.authority_mode.uses_local_combat() {
        let mut enemies = Vec::new();
        let mut mosquitons = Vec::new();
        let mut spideys = Vec::new();
        process_player_attacks(
            &camera.0,
            &map.0,
            &attack.sprites,
            config.hitscan_damage,
            time.delta_secs(),
            time.elapsed_secs(),
            &mut attack.input,
            &mut attack.loadout,
            &mut attack.state,
            &mut enemies,
            &mut mosquitons,
            &mut spideys,
            &mut projectiles.0,
            &mut impacts.0,
            &mut char_decals.0,
            config.screen_height as f32,
            &mut shoot.0,
            &attack.burn_config,
            attack.visual_config.view_bob_amplitude,
            attack.visual_config.view_bob_freq_mult,
            SnapTurnVisualInput {
                remaining: quick_turn.remaining_radians(),
                total: quick_turn.total_radians(),
                direction: quick_turn.direction(),
            },
        );
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

    // Gather spideys into Vecs for slice-based attack processing.
    let mut spidey_entities: Vec<Entity> = Vec::new();
    let mut spideys: Vec<Spidey> = Vec::new();
    for (entity, spidey) in spidey_q.iter() {
        spidey_entities.push(entity);
        spideys.push(spidey.clone());
    }

    process_player_attacks(
        &camera.0,
        &map.0,
        &attack.sprites,
        config.hitscan_damage,
        time.delta_secs(),
        time.elapsed_secs(),
        &mut attack.input,
        &mut attack.loadout,
        &mut attack.state,
        &mut enemies,
        &mut mosquitons,
        &mut spideys,
        &mut projectiles.0,
        &mut impacts.0,
        &mut char_decals.0,
        config.screen_height as f32,
        &mut shoot.0,
        &attack.burn_config,
        attack.visual_config.view_bob_amplitude,
        attack.visual_config.view_bob_freq_mult,
        SnapTurnVisualInput {
            remaining: quick_turn.remaining_radians(),
            total: quick_turn.total_radians(),
            direction: quick_turn.direction(),
        },
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

    for (i, &entity) in spidey_entities.iter().enumerate() {
        if !matches!(spideys[i].state, SpideyState::Dead)
            && let Ok((_, mut s)) = spidey_q.get_mut(entity)
        {
            *s = spideys[i].clone();
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
    spidey_q: Query<&Spidey>,
    burn_config: Res<carcinisation_fps_core::BurnConfig>,
    mut zbuffer: Local<Vec<f32>>,
) {
    let mut image = CxImage::empty(UVec2::new(
        view.config.screen_width,
        view.config.screen_height,
    ));

    // Gather enemies with sprite indices, mosquitons, and spideys for billboard rendering.
    let mut enemies: Vec<Enemy> = Vec::new();
    let mut indices: Vec<usize> = Vec::new();
    for (e, idx) in enemy_q.iter() {
        enemies.push(e.clone());
        indices.push(idx.0);
    }
    let mosquitons: Vec<Mosquiton> = mosquiton_q.iter().cloned().collect();
    let spideys: Vec<Spidey> = spidey_q.iter().cloned().collect();

    let enemy_bbs = billboards_from_enemies_indexed(&enemies, &indices, &sources.pairs.0);
    let corpse_flame_bbs = burning_corpse_flame_billboards(
        &enemies,
        &indices,
        &sources.pairs.0,
        &mosquitons,
        &sources.mosquiton_sprites.0,
        &spideys,
        &sources.spidey_sprites.0,
        &CorpseFlameContext {
            attack_sprites: &view.attack_sprites,
            config: view.attack_state.shared(),
            camera: &view.camera.0,
            elapsed_secs: time.elapsed_secs(),
        },
    );
    let ground_fire_bbs = ground_fire_billboards(
        &sources.ground_fires.0,
        &view.attack_sprites,
        &view.camera.0,
        time.elapsed_secs(),
        &sources.ground_fire_vis,
    );
    let proj_bbs = billboards_from_projectiles(
        &sources.projectiles.0,
        &sources.blood_shot_sprites.0.hover,
        &sources.spider_shot_sprites.0.hover,
    );
    let impact_bbs = billboards_from_projectile_impacts(
        &sources.impacts.0,
        &sources.blood_shot_sprites.0,
        &sources.spider_shot_sprites.0,
    );
    let mosquiton_bbs = billboards_from_mosquitons(&mosquitons, &sources.mosquiton_sprites.0);
    let spidey_bbs = billboards_from_spideys(&spideys, &sources.spidey_sprites.0);
    let alive_burn_bbs = alive_burn_flame_billboards(
        &enemies,
        &indices,
        &sources.pairs.0,
        &mosquitons,
        &sources.mosquiton_sprites.0,
        &spideys,
        &sources.spidey_sprites.0,
        &CorpseFlameContext {
            attack_sprites: &view.attack_sprites,
            config: view.attack_state.shared(),
            camera: &view.camera.0,
            elapsed_secs: time.elapsed_secs(),
        },
        &burn_config,
    );
    let local_flame_bbs = view
        .attack_state
        .flame_chain_billboards(&view.camera.0, &view.attack_sprites);
    let all_bbs: Vec<Billboard> = sources
        .static_bbs
        .0
        .iter()
        .cloned()
        .chain(view.extra_bbs.0.iter().cloned())
        .chain(ground_fire_bbs)
        .chain(corpse_flame_bbs)
        .chain(alive_burn_bbs)
        .chain(enemy_bbs)
        .chain(mosquiton_bbs)
        .chain(spidey_bbs)
        .chain(impact_bbs)
        .chain(proj_bbs)
        .chain(local_flame_bbs)
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
        &mut zbuffer,
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
    draw_fps_screen_particles(
        &mut image,
        &view.screen_particles,
        &view.screen_particle_config,
    );

    if view.dead.0 {
        draw_overlay_tint(
            &mut image,
            2,
            death_red_density(&view.death_view, &view.config),
        );
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
    config: &'a carcinisation_fps_core::PlayerFlamethrowerConfig,
    camera: &'a Camera,
    elapsed_secs: f32,
}

fn burning_corpse_flame_billboards(
    enemies: &[Enemy],
    enemy_sprite_indices: &[usize],
    enemy_sprite_pairs: &[(CxImage, CxImage)],
    mosquitons: &[Mosquiton],
    mosquiton_sprites: &MosquitonBillboardSprites,
    spideys: &[Spidey],
    spidey_sprites: &SpideyBillboardSprites,
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
    for spidey in spideys {
        if let SpideyState::BurningCorpse { seed, .. } = spidey.state {
            push_burning_corpse_flames(
                &mut billboards,
                spidey.position,
                spidey.visual_height,
                spidey.config.billboard_height,
                seed,
                spidey_sprites.alive_sprite_at(0.0),
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
            sprite: std::sync::Arc::clone(
                ctx.attack_sprites
                    .flame_frame_loop(ctx.elapsed_secs + flame.phase_secs),
            ),
            flip_x: false,
            palette_variant: None,
        });
    }
}

/// Generate flame billboards on alive burning enemies/mosquitons/spideys.
/// Flame count scales with burn intensity.
fn alive_burn_flame_billboards(
    enemies: &[Enemy],
    enemy_sprite_indices: &[usize],
    enemy_sprite_pairs: &[(CxImage, CxImage)],
    mosquitons: &[Mosquiton],
    mosquiton_sprites: &MosquitonBillboardSprites,
    spideys: &[Spidey],
    spidey_sprites: &SpideyBillboardSprites,
    ctx: &CorpseFlameContext<'_>,
    burn_config: &carcinisation_fps_core::BurnConfig,
) -> Vec<Billboard> {
    let mut billboards = Vec::new();

    for (index, enemy) in enemies.iter().enumerate() {
        if !enemy.is_alive() || !enemy.burn_state.is_burning() {
            continue;
        }
        let pair_index = enemy_sprite_indices.get(index).copied().unwrap_or(0);
        let Some((alive_sprite, _)) = enemy_sprite_pairs
            .get(pair_index)
            .or_else(|| enemy_sprite_pairs.first())
        else {
            continue;
        };
        push_alive_burn_flames_sp(
            &mut billboards,
            enemy.position,
            0.0,
            1.0,
            enemy.burn_state.intensity,
            index as u32,
            alive_sprite,
            ctx,
            burn_config,
        );
    }

    for (mi, mosquiton) in mosquitons.iter().enumerate() {
        if !mosquiton.is_alive() || !mosquiton.burn_state.is_burning() {
            continue;
        }
        let sprite = mosquiton_sprites.alive_sprite_at(0.0);
        push_alive_burn_flames_sp(
            &mut billboards,
            mosquiton.position,
            mosquiton.height,
            mosquiton.config.billboard_height,
            mosquiton.burn_state.intensity,
            (enemies.len() + mi) as u32,
            sprite,
            ctx,
            burn_config,
        );
    }

    for (si, spidey) in spideys.iter().enumerate() {
        if !spidey.is_alive() || !spidey.burn_state.is_burning() {
            continue;
        }
        push_alive_burn_flames_sp(
            &mut billboards,
            spidey.position,
            spidey.visual_height,
            spidey.config.billboard_height,
            spidey.burn_state.intensity,
            (enemies.len() + mosquitons.len() + si) as u32,
            spidey_sprites.alive_sprite_at(0.0),
            ctx,
            burn_config,
        );
    }

    billboards
}

fn push_alive_burn_flames_sp(
    billboards: &mut Vec<Billboard>,
    position: Vec2,
    height: f32,
    base_world_height: f32,
    intensity: f32,
    seed: u32,
    sprite: &CxImage,
    ctx: &CorpseFlameContext<'_>,
    burn_config: &carcinisation_fps_core::BurnConfig,
) {
    let all_flames = carcinisation_fps_core::centered_flames_from_mask(
        seed,
        sprite.width(),
        sprite.height(),
        |x, y| sprite.data()[y * sprite.width() + x] != TRANSPARENT_INDEX,
        burn_config.max_burn_flames,
    );
    if all_flames.is_empty() {
        return;
    }

    let visible =
        carcinisation_fps_core::burn_flame_count(intensity, all_flames.len(), burn_config);
    if visible == 0 {
        return;
    }

    let to_enemy = position - ctx.camera.position;
    let distance = to_enemy.length().max(0.1);
    let behind_dir = if distance > 0.001 {
        to_enemy / distance
    } else {
        ctx.camera.direction()
    };
    let right = Vec2::new(-ctx.camera.direction().y, ctx.camera.direction().x);
    let px_to_world = base_world_height / sprite.height() as f32;

    let flame_size = carcinisation_fps_core::burn_flame_scale(intensity, burn_config);
    for flame in all_flames.iter().take(visible) {
        let lateral_units = flame.offset_px.x * px_to_world;
        let vertical_units = flame.offset_px.y * px_to_world;

        billboards.push(Billboard {
            position: position - behind_dir * 0.04 + right * lateral_units,
            height: height + vertical_units,
            world_height: base_world_height * flame_size * flame.scale,
            sprite: std::sync::Arc::clone(
                ctx.attack_sprites
                    .flame_frame_loop(ctx.elapsed_secs + flame.phase_secs),
            ),
            flip_x: false,
            palette_variant: None,
        });
    }
}

/// Crop the bottom N rows from a `CxImage`, returning a new image.
#[must_use]
pub fn crop_bottom(image: &CxImage, rows: usize) -> CxImage {
    let w = image.width();
    let h = image.height();
    if rows >= h {
        return CxImage::empty(UVec2::new(w as u32, 1));
    }
    let new_h = h - rows;
    CxImage::new(image.data()[..new_h * w].to_vec(), w)
}

/// Generate billboards for active ground fires.
fn ground_fire_billboards(
    fires: &[GroundFire],
    attack_sprites: &PlayerAttackSprites,
    camera: &Camera,
    elapsed_secs: f32,
    vis: &GroundFireVisualConfig,
) -> Vec<Billboard> {
    let mut billboards = Vec::new();

    let cam_dir = camera.direction();
    let right = Vec2::new(-cam_dir.y, cam_dir.x);

    let gf_config = GroundFireConfig::default();

    for fire in fires {
        let intensity = fire.intensity(&gf_config);
        let flames = ground_fire_flame_layout(fire.seed, vis.flame_count, vis.visual_radius);
        for (offset, scale, phase) in &flames {
            let full_sprite = attack_sprites.flame_frame_loop(elapsed_secs + phase);
            let cropped = crop_bottom(full_sprite, vis.crop_bottom_px);
            let world_height = vis.flame_world_height * scale * intensity;
            // Bottom of the cropped sprite sits at ground level (-0.5).
            #[allow(clippy::suboptimal_flops)]
            let height = -0.5 + world_height * 0.5;
            billboards.push(Billboard {
                position: fire.position + right * offset.x + cam_dir * offset.y,
                height,
                world_height,
                sprite: std::sync::Arc::new(cropped),
                flip_x: false,
                palette_variant: None,
            });
        }
    }

    billboards
}

/// Set to `true` from your input system to trigger a hitscan shot.
/// The plugin resets it to `false` after processing.
#[derive(Resource, Default)]
pub struct ShootRequest(pub bool);

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;

    fn test_room_map_data() -> MapData {
        MapData::from_ron(include_str!(
            "../../../assets/config/fp/test_room.fp_map.ron"
        ))
        .expect("test_room.fp_map.ron should parse")
    }

    fn setup_from_test_room(authority_mode: FpsAuthorityMode) -> FpMapEntitySetup {
        let sprite_pairs = vec![(make_enemy_sprite(24, 2), make_death_sprite(24, 1))];
        let mosquiton_sprites = make_mosquiton_billboard_sprites().unwrap();
        let spidey_sprites = make_spidey_billboard_sprites().unwrap();
        let combat_config = carcinisation_fps_core::FpsCombatConfig::load();
        build_map_entity_setup(
            &test_room_map_data(),
            authority_mode,
            &sprite_pairs,
            &mosquiton_sprites,
            &spidey_sprites,
            &combat_config,
        )
    }

    fn contact_hazard_test_config() -> carcinisation_fps_core::PlayerFlamethrowerConfig {
        let mut config = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
        config.burning_corpse_contact_damage = 1;
        config.burning_corpse_contact_tick_ms = NonZeroU64::new(300).unwrap();
        config.burning_corpse_contact_radius = 0.6;
        config
    }

    #[test]
    fn local_authority_map_setup_spawns_local_combat_entities() {
        let setup = setup_from_test_room(FpsAuthorityMode::LocalAuthority);

        assert_eq!(setup.static_billboards.len(), 4);
        assert_eq!(setup.enemies.len(), 0);
        assert_eq!(setup.initial_enemy_billboards.len(), 0);
        assert_eq!(setup.mosquitons.len(), 6);
        assert_eq!(setup.initial_mosquiton_billboards.len(), 6);
    }

    #[test]
    fn remote_client_map_setup_suppresses_local_combat_entities() {
        let setup = setup_from_test_room(FpsAuthorityMode::RemoteClient);

        assert_eq!(setup.static_billboards.len(), 4);
        assert_eq!(setup.enemies.len(), 0);
        assert_eq!(setup.initial_enemy_billboards.len(), 0);
        assert_eq!(setup.mosquitons.len(), 0);
        assert_eq!(setup.initial_mosquiton_billboards.len(), 0);
    }

    #[test]
    fn burning_corpse_flames_are_deterministic_billboards() {
        let sprites = PlayerAttackSprites::load();
        let enemy_pairs = vec![(make_enemy_sprite(24, 2), make_death_sprite(24, 1))];
        let mosquiton_sprites = make_mosquiton_billboard_sprites().unwrap();
        let config = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
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

        let spidey_sprites = make_spidey_billboard_sprites().unwrap();
        let first = burning_corpse_flame_billboards(
            &enemies,
            &[0],
            &enemy_pairs,
            &[],
            &mosquiton_sprites,
            &[],
            &spidey_sprites,
            &ctx,
        );
        let second = burning_corpse_flame_billboards(
            &enemies,
            &[0],
            &enemy_pairs,
            &[],
            &mosquiton_sprites,
            &[],
            &spidey_sprites,
            &ctx,
        );

        assert_eq!(first.len(), config.burning_flame_count.get());
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
        let config = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
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

        let spidey_sprites = make_spidey_billboard_sprites().unwrap();
        let first = burning_corpse_flame_billboards(
            &[],
            &[],
            &enemy_pairs,
            &mosquitons,
            &mosquiton_sprites,
            &[],
            &spidey_sprites,
            &ctx,
        );
        mosquitons[0].animation_time = 0.75;
        let second = burning_corpse_flame_billboards(
            &[],
            &[],
            &enemy_pairs,
            &mosquitons,
            &mosquiton_sprites,
            &[],
            &spidey_sprites,
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
    fn crossfire_exposure_builds_burn_on_living_enemy() {
        let config = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
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

        let burn_cfg = carcinisation_fps_core::burning::load_config();
        let dt = 1.0 / 30.0;
        tick_burning_corpse_crossfire(
            &mut enemies,
            &mut mosquitons,
            &corpses,
            &config,
            &burn_cfg,
            dt,
        );

        // Crossfire applies exposure — enemy should now be burning.
        assert!(enemies[0].burn_state.is_burning());
        assert!(enemies[0].burn_state.intensity > 0.0);
    }

    #[test]
    fn crossfire_exposure_builds_burn_on_living_mosquiton() {
        let config = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
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

        let burn_cfg = carcinisation_fps_core::burning::load_config();
        let dt = 1.0 / 30.0;
        tick_burning_corpse_crossfire(
            &mut enemies,
            &mut mosquitons,
            &corpses,
            &config,
            &burn_cfg,
            dt,
        );

        assert!(mosquitons[0].burn_state.is_burning());
        assert!(mosquitons[0].burn_state.intensity > 0.0);
    }

    #[test]
    fn crossfire_exposure_ignores_out_of_range() {
        let mut config = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
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

        let burn_cfg = carcinisation_fps_core::burning::load_config();
        tick_burning_corpse_crossfire(
            &mut enemies,
            &mut mosquitons,
            &corpses,
            &config,
            &burn_cfg,
            1.0 / 30.0,
        );

        assert!(!enemies[0].burn_state.is_burning());
    }

    // --- ground fire ---

    #[test]
    fn ground_fire_spawns_from_burning_corpse_position() {
        let gf_config = GroundFireConfig::default();
        let mut fires = Vec::new();
        // Enemy in BurningCorpse state at (3.0, 3.0).
        let pos = Vec2::new(3.0, 3.0);
        try_spawn_ground_fire(&mut fires, pos, &gf_config);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].position, pos);
    }

    #[test]
    fn ground_fire_no_duplicate_at_same_position() {
        let gf_config = GroundFireConfig::default();
        let mut fires = Vec::new();
        let pos = Vec2::new(3.0, 3.0);
        try_spawn_ground_fire(&mut fires, pos, &gf_config);
        // Second attempt at same position should be deduplicated.
        let spawned = try_spawn_ground_fire(&mut fires, pos, &gf_config);
        assert!(!spawned);
        assert_eq!(fires.len(), 1);
    }

    #[test]
    fn ground_fire_expires_after_lifetime() {
        let mut fires = vec![GroundFire {
            position: Vec2::new(3.0, 3.0),
            remaining_secs: 1.0,
            seed: 42,
        }];
        tick_ground_fires(&mut fires, 1.1);
        assert!(fires.is_empty());
    }

    #[test]
    fn ground_fire_billboards_anchor_at_ground_level() {
        let vis = GroundFireVisualConfig::load();
        let fires = vec![GroundFire {
            position: Vec2::new(3.0, 3.0),
            remaining_secs: 2.0,
            seed: 42,
        }];
        let attack_sprites = PlayerAttackSprites::load();
        let camera = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let bbs = ground_fire_billboards(&fires, &attack_sprites, &camera, 0.0, &vis);
        assert_eq!(bbs.len(), vis.flame_count);
        // All flames should be at or below eye level (negative height).
        for bb in &bbs {
            assert!(
                bb.height < 0.0,
                "ground fire billboard height {} should be negative",
                bb.height
            );
        }
    }

    #[test]
    fn non_burning_death_does_not_spawn_ground_fire() {
        let gf_config = GroundFireConfig::default();
        let mut fires = Vec::new();
        // Only BurningCorpse state should trigger ground fire, not Dying/Dead.
        // Simulate: no burning corpse positions → no ground fires.
        let burning_corpses: Vec<Vec2> = vec![];
        for &pos in &burning_corpses {
            try_spawn_ground_fire(&mut fires, pos, &gf_config);
        }
        assert!(fires.is_empty());
    }

    /// Helper: `b` = (pressed, `just_pressed`, `just_released`).
    fn chord_input_at(
        b: (bool, bool, bool),
        down: (bool, bool, bool),
        left: (bool, bool, bool),
        right: (bool, bool, bool),
        now_secs: f32,
    ) -> TurnChordInput {
        TurnChordInput {
            b_pressed: b.0,
            b_just_pressed: b.1,
            b_just_released: b.2,
            down_pressed: down.0,
            down_just_pressed: down.1,
            down_just_released: down.2,
            left_pressed: left.0,
            left_just_pressed: left.1,
            left_just_released: left.2,
            right_pressed: right.0,
            right_just_pressed: right.1,
            right_just_released: right.2,
            now_secs,
        }
    }

    fn chord_input(
        b: (bool, bool, bool),
        down: (bool, bool, bool),
        left: (bool, bool, bool),
        right: (bool, bool, bool),
    ) -> TurnChordInput {
        chord_input_at(b, down, left, right, 0.0)
    }

    const NONE: (bool, bool, bool) = (false, false, false);
    const B_OFF: (bool, bool, bool) = (false, false, false);
    const B_PRESSED: (bool, bool, bool) = (true, true, false);
    const B_HELD: (bool, bool, bool) = (true, false, false);
    const B_RELEASED: (bool, bool, bool) = (false, false, true);

    // --- Selection (press selects, does not fire) ---

    #[test]
    fn b_then_down_press_selects_without_firing() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        // Down just_pressed → selects, but does NOT fire.
        let input = chord_input(B_HELD, (true, true, false), NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
    }

    #[test]
    fn b_then_left_press_selects_without_firing() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, (true, true, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
    }

    #[test]
    fn b_then_right_press_selects_without_firing() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, NONE, (true, true, false));
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
    }

    // --- Commit (direction release fires) ---

    #[test]
    fn releasing_down_fires_quick_turn() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Press Down → select.
        let input = chord_input(B_HELD, (true, true, false), NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Release Down → fire.
        let input = chord_input(B_HELD, (false, false, true), NONE, NONE);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::QuickTurn)
        );
    }

    #[test]
    fn releasing_left_fires_snap_left() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, (true, true, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, (false, false, true), NONE);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnLeft)
        );
    }

    #[test]
    fn releasing_right_fires_snap_right() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, NONE, (true, true, false));
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, NONE, (false, false, true));
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnRight)
        );
    }

    #[test]
    fn holding_selected_direction_does_not_fire() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Press Left → select.
        let input = chord_input(B_HELD, NONE, (true, true, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Continue holding Left → still pending, no fire.
        let input = chord_input(B_HELD, NONE, (true, false, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
    }

    // --- Safety ---

    #[test]
    fn pre_held_left_then_b_press_does_not_snap() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_OFF, NONE, (true, false, false), NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // B pressed while Left held → Left is blocked.
        let input = chord_input_at(B_PRESSED, NONE, (true, false, false), NONE, 0.1);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        // Left released → no selection was made, so no fire.
        let input = chord_input_at(B_HELD, NONE, (false, false, true), NONE, 0.12);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
    }

    #[test]
    fn pre_held_down_then_b_press_does_not_snap() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_OFF, (true, false, false), NONE, NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input_at(B_PRESSED, (true, false, false), NONE, NONE, 0.1);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        // Down released → blocked direction, no fire.
        let input = chord_input_at(B_HELD, (false, false, true), NONE, NONE, 0.12);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
    }

    #[test]
    fn b_release_before_direction_release_cancels() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Press Left → select.
        let input = chord_input(B_HELD, NONE, (true, true, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // B released while Left still held → cancel.
        let input = chord_input(B_RELEASED, NONE, (true, false, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(!state.is_pending());
    }

    #[test]
    fn window_expiry_before_direction_release_cancels() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_PRESSED, NONE, NONE, NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Left pressed at 50ms.
        let input = chord_input_at(B_HELD, NONE, (true, true, false), NONE, 0.05);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Window expires at 210ms (Left still held).
        let input = chord_input_at(B_HELD, NONE, (true, false, false), NONE, 0.21);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(!state.is_pending());
        // Left released after window → no fire.
        let input = chord_input_at(B_HELD, NONE, (false, false, true), NONE, 0.25);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
    }

    #[test]
    fn after_fire_blocked_until_release_prevents_repeat() {
        let mut state = TurnChordState::default();
        // Full chord: B → Down press → Down release → fires.
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, (true, true, false), NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, (false, false, true), NONE, NONE);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::QuickTurn)
        );
        // B still held → blocked.
        let input = chord_input(B_HELD, NONE, (true, true, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
    }

    #[test]
    fn release_all_then_retrigger_works() {
        let mut state = TurnChordState::default();
        // Fire once.
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, NONE, (true, true, false));
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, NONE, (false, false, true));
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnRight)
        );
        // Release all.
        let input = chord_input(B_OFF, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Fire again.
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, (true, true, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input(B_HELD, NONE, (false, false, true), NONE);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnLeft)
        );
    }

    // --- Movement / misc ---

    #[test]
    fn movement_suppressed_while_direction_selected() {
        let mut state = TurnChordState::default();
        assert!(!state.is_pending());
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        let _ = resolve_turn_chord(&input, &mut state, false);
        assert!(state.is_pending());
        // Select Left.
        let input = chord_input(B_HELD, NONE, (true, true, false), NONE);
        let _ = resolve_turn_chord(&input, &mut state, false);
        assert!(
            state.is_pending(),
            "should stay pending while direction held"
        );
        // Hold Left.
        let input = chord_input(B_HELD, NONE, (true, false, false), NONE);
        let _ = resolve_turn_chord(&input, &mut state, false);
        assert!(state.is_pending());
    }

    #[test]
    fn b_press_release_no_direction_does_nothing() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        let input = chord_input(B_RELEASED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(!state.is_pending());
    }

    #[test]
    fn normal_backward_unaffected() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_OFF, (true, true, false), NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(!state.is_pending());
    }

    #[test]
    fn down_priority_over_left_on_same_frame() {
        let mut state = TurnChordState::default();
        // B + Down + Left same frame → Down selected. Release Down → fires QuickTurn.
        let input = chord_input(B_PRESSED, (true, true, false), (true, true, false), NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        let input = chord_input(B_HELD, (false, false, true), (true, false, false), NONE);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::QuickTurn)
        );
    }

    // --- Timing ---

    #[test]
    fn direction_at_190ms_selects_within_window() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_PRESSED, NONE, NONE, NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Right at 190ms (within 200ms window) → selects.
        let input = chord_input_at(B_HELD, NONE, NONE, (true, true, false), 0.19);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        // Release Right → fires.
        let input = chord_input_at(B_HELD, NONE, NONE, (false, false, true), 0.195);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnRight)
        );
    }

    #[test]
    fn direction_after_window_does_not_select() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_PRESSED, NONE, NONE, NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Window expires at 210ms.
        let input = chord_input_at(B_HELD, NONE, NONE, NONE, 0.21);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(!state.is_pending());
        // Direction pressed after expiry → no selection.
        let input = chord_input_at(B_HELD, NONE, (true, true, false), NONE, 0.25);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
    }

    #[test]
    fn same_frame_direction_press_and_release_fires() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Left just_pressed AND just_released on the same frame (fleeting tap).
        let input = chord_input(B_HELD, NONE, (false, true, true), NONE);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnLeft)
        );
    }

    #[test]
    fn pre_held_right_then_b_press_does_not_snap() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_OFF, NONE, NONE, (true, false, false), 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // B pressed while Right held → Right is blocked.
        let input = chord_input_at(B_PRESSED, NONE, NONE, (true, false, false), 0.1);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        // Right released → blocked direction, no fire.
        let input = chord_input_at(B_HELD, NONE, NONE, (false, false, true), 0.12);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
    }

    #[test]
    fn blocked_direction_selectable_after_fresh_b_press() {
        let mut state = TurnChordState::default();
        // Left pre-held → B press → Left blocked → B release → cancel.
        let input = chord_input_at(B_OFF, NONE, (true, false, false), NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input_at(B_PRESSED, NONE, (true, false, false), NONE, 0.1);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input_at(B_RELEASED, NONE, (true, false, false), NONE, 0.15);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Release all.
        let input = chord_input_at(B_OFF, NONE, NONE, NONE, 0.3);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Fresh chord: Left now just_pressed (not pre-held) → selectable.
        let input = chord_input_at(B_PRESSED, NONE, NONE, NONE, 0.5);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input_at(B_HELD, NONE, (true, true, false), NONE, 0.55);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        let input = chord_input_at(B_HELD, NONE, (false, false, true), NONE, 0.6);
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnLeft)
        );
    }

    #[test]
    fn left_priority_over_right_on_same_frame() {
        let mut state = TurnChordState::default();
        let input = chord_input(B_PRESSED, NONE, NONE, NONE);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Left + Right just_pressed same frame → Left wins.
        let input = chord_input(B_HELD, NONE, (true, true, false), (true, true, false));
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending());
        // Release Left → fires SideTurnLeft.
        let input = chord_input(B_HELD, NONE, (false, false, true), (true, false, false));
        assert_eq!(
            resolve_turn_chord(&input, &mut state, false),
            Some(TurnKind::SideTurnLeft)
        );
    }

    #[test]
    fn chord_window_exact_boundary_selects() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_PRESSED, NONE, NONE, NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Direction at exactly 200ms — strict `>` means this is still within window.
        let input = chord_input_at(B_HELD, NONE, NONE, (true, true, false), 0.20);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(state.is_pending(), "200ms should still be within window");
    }

    #[test]
    fn chord_window_exact_boundary_plus_epsilon_expires() {
        let mut state = TurnChordState::default();
        let input = chord_input_at(B_PRESSED, NONE, NONE, NONE, 0.0);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        // Direction at 200.1ms — just past window → expired.
        let input = chord_input_at(B_HELD, NONE, NONE, (true, true, false), 0.2001);
        assert_eq!(resolve_turn_chord(&input, &mut state, false), None);
        assert!(!state.is_pending());
    }

    #[test]
    fn snap_turn_animates_over_configured_duration() {
        let config = Config::default();
        let mut camera = Camera {
            angle: 0.25,
            ..Default::default()
        };
        let mut turn = QuickTurnState::default();
        request_snap_turn(&mut turn, TurnKind::QuickTurn, &config);

        tick_quick_turn(&mut camera, &mut turn, 0.2);
        assert!((camera.angle - (0.25 + std::f32::consts::FRAC_PI_2)).abs() < 1e-5);
        assert!(turn.is_active());

        tick_quick_turn(&mut camera, &mut turn, 0.2);
        assert!((camera.angle - (0.25 + std::f32::consts::PI)).abs() < 1e-5);
        assert!(!turn.is_active());
    }

    #[test]
    fn side_turn_shares_angular_velocity_with_quick_turn() {
        let config = Config::default();
        let mut camera = Camera {
            angle: 0.0,
            ..Default::default()
        };
        let mut turn = QuickTurnState::default();
        request_snap_turn(&mut turn, TurnKind::SideTurnLeft, &config);

        tick_quick_turn(
            &mut camera,
            &mut turn,
            config.quick_turn_duration_secs / 2.0,
        );
        assert!((camera.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!(!turn.is_active());
    }

    #[test]
    fn side_turn_right_rotates_negative() {
        let config = Config::default();
        let mut camera = Camera {
            angle: 1.0,
            ..Default::default()
        };
        let mut turn = QuickTurnState::default();
        request_snap_turn(&mut turn, TurnKind::SideTurnRight, &config);

        tick_quick_turn(
            &mut camera,
            &mut turn,
            config.quick_turn_duration_secs / 2.0,
        );
        let expected = (1.0 - std::f32::consts::FRAC_PI_2).rem_euclid(std::f32::consts::TAU);
        assert!((camera.angle - expected).abs() < 1e-5);
        assert!(!turn.is_active());
    }

    #[test]
    fn snap_turn_request_ignored_while_active() {
        let config = Config::default();
        let mut turn = QuickTurnState::default();
        request_snap_turn(&mut turn, TurnKind::QuickTurn, &config);
        tick_quick_turn(&mut Camera::default(), &mut turn, 0.05);
        let remaining = turn.remaining_radians;
        request_snap_turn(&mut turn, TurnKind::SideTurnLeft, &config);
        assert_eq!(turn.remaining_radians, remaining);
    }

    /// In `RemoteClient` mode, `apply_quick_turn_animation` must NOT rotate
    /// the camera — prediction owns the angle. It should only tick the
    /// `QuickTurnState` timer for input suppression.
    #[test]
    fn remote_client_quick_turn_does_not_rotate_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs(1));

        let config = Config {
            authority_mode: FpsAuthorityMode::RemoteClient,
            ..Default::default()
        };
        app.insert_resource(config);

        let camera = Camera {
            angle: 1.0,
            ..Default::default()
        };
        app.insert_resource(CameraRes(camera));

        let mut turn = QuickTurnState::default();
        request_snap_turn(&mut turn, TurnKind::QuickTurn, &Config::default());
        assert!(turn.is_active());
        app.insert_resource(turn);

        app.add_systems(Update, apply_quick_turn_animation);

        // First update has dt=0; second has real dt.
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.update();

        let cam = app.world().resource::<CameraRes>();
        assert!(
            (cam.0.angle - 1.0).abs() < 1e-6,
            "camera angle should be unchanged in RemoteClient mode: got {:.4}",
            cam.0.angle
        );

        // QuickTurnState should have ticked down (for input suppression).
        let qt = app.world().resource::<QuickTurnState>();
        assert!(
            qt.remaining_radians < std::f32::consts::PI,
            "QuickTurnState should have decremented: {:.4}",
            qt.remaining_radians
        );
    }

    /// In `LocalAuthority` mode, `apply_quick_turn_animation` DOES rotate camera.
    #[test]
    fn local_authority_quick_turn_rotates_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs(1));

        let config = Config::default(); // LocalAuthority by default
        assert_eq!(config.authority_mode, FpsAuthorityMode::LocalAuthority);
        app.insert_resource(config);

        let camera = Camera {
            angle: 0.0,
            ..Default::default()
        };
        app.insert_resource(CameraRes(camera));

        let mut turn = QuickTurnState::default();
        request_snap_turn(&mut turn, TurnKind::QuickTurn, &Config::default());
        app.insert_resource(turn);

        app.add_systems(Update, apply_quick_turn_animation);

        // First update has dt=0; second has real dt.
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.update();

        let cam = app.world().resource::<CameraRes>();
        assert!(
            cam.0.angle > 0.01,
            "camera angle should have rotated in LocalAuthority mode: got {:.4}",
            cam.0.angle
        );
    }

    #[test]
    fn death_view_turns_toward_killer_and_red_increases() {
        let config = Config::default();
        let mut camera = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            ..Default::default()
        };
        let mut death_view = DeathViewState::default();

        request_death_view(&mut death_view, &camera, Vec2::Y);
        tick_death_view(
            &mut camera,
            &mut death_view,
            config.death_turn_duration_secs * 0.5,
            &config,
        );
        assert!((camera.angle - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        let half_density = death_red_density(&death_view, &config);
        assert!(half_density > 0.0);
        assert!(half_density < config.death_red_max_density);

        tick_death_view(
            &mut camera,
            &mut death_view,
            config.death_turn_duration_secs * 0.5,
            &config,
        );
        assert!((camera.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!(
            (death_red_density(&death_view, &config) - config.death_red_max_density).abs() < 1e-5
        );
    }

    #[test]
    fn death_view_uses_shortest_turn_direction() {
        let config = Config::default();
        let mut camera = Camera {
            position: Vec2::ZERO,
            angle: 350.0_f32.to_radians(),
            ..Default::default()
        };
        let mut death_view = DeathViewState::default();
        let ten_degrees = Vec2::new(10.0_f32.to_radians().cos(), 10.0_f32.to_radians().sin());

        request_death_view(&mut death_view, &camera, ten_degrees);
        tick_death_view(
            &mut camera,
            &mut death_view,
            config.death_turn_duration_secs,
            &config,
        );
        assert!((camera.angle - 10.0_f32.to_radians()).abs() < 1e-5);

        camera.angle = 10.0_f32.to_radians();
        let mut death_view = DeathViewState::default();
        let three_fifty_degrees =
            Vec2::new(350.0_f32.to_radians().cos(), 350.0_f32.to_radians().sin());
        request_death_view(&mut death_view, &camera, three_fifty_degrees);
        tick_death_view(
            &mut camera,
            &mut death_view,
            config.death_turn_duration_secs,
            &config,
        );
        assert!((camera.angle - 350.0_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn player_damage_latches_first_killing_source() {
        let config = Config::default();
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
            &config,
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
            &config,
        );

        assert!(dead);
        assert_eq!(health, 0);
        assert!((first_target - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(death_view.target_angle, first_target);
    }

    #[test]
    fn player_damage_requests_camera_shake() {
        let config = Config::default();
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
            &config,
        );

        assert_eq!(health, 99);
        assert!(!dead);
        assert_eq!(camera_shake.intensity, config.camera_shake_base_intensity);
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

        let sprites = PlayerAttackSprites::load();
        let burn_config = carcinisation_fps_core::burning::load_config();
        process_player_attacks(
            &camera,
            &map,
            &sprites,
            15,
            1.0 / 60.0,
            0.0,
            &mut input,
            &mut loadout,
            &mut attack_state,
            &mut enemies,
            &mut mosquitons,
            &mut [],
            &mut setup_projectiles,
            &mut setup_impacts,
            &mut char_decals,
            144.0,
            &mut shoot_request,
            &burn_config,
            1.5,
            2.0,
            SnapTurnVisualInput::default(),
        );

        let mut projectiles = vec![Projectile {
            position: Vec2::new(0.7, 0.0),
            source_position: Vec2::new(3.0, 0.0),
            direction: -Vec2::X,
            speed: 10.0,
            radius: 0.3,
            damage: 10,
            lifetime: 1.0,
            initial_lifetime: 1.0,
            alive: true,
            kind: carcinisation_fps_core::ProjectileKind::default(),
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
        let config = Config::default();
        let mut camera_shake = CameraShakeState::default();
        request_camera_shake(&mut camera_shake, &config);
        request_camera_shake(&mut camera_shake, &config);

        assert_eq!(
            camera_shake.intensity,
            config.camera_shake_base_intensity * 2.0
        );

        tick_camera_shake(&mut camera_shake, 0.1, 0.0, 1.0, &config);
        assert_eq!(
            camera_shake.current_offset,
            IVec2::new((config.camera_shake_base_intensity * 2.0) as i32, 0)
        );
        assert!(camera_shake.intensity < config.camera_shake_base_intensity * 2.0);

        camera_shake.intensity = config.camera_shake_threshold * 0.5;
        camera_shake.current_offset = IVec2::new(1, 1);
        tick_camera_shake(&mut camera_shake, 0.016, 0.0, 0.0, &config);
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
