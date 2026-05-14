use std::sync::Arc;

use bevy::prelude::*;
#[cfg(not(target_family = "wasm"))]
use bevy_renet2::netcode::NetcodeClientTransport;
use bevy_replicon::prelude::*;
#[cfg(not(target_family = "wasm"))]
use bevy_replicon_renet2::RenetChannelsExt;
#[cfg(not(target_family = "wasm"))]
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient};
use carcinisation_fps::billboard::Billboard;
use carcinisation_fps::billboard::{
    make_blood_shot_sprite, make_damage_invert_sprite, make_enemy_sprite,
    make_mosquiton_placeholder_sprite,
};
use carcinisation_fps::directional_billboard::{
    BillboardAnimationState, DirectionalBillboardAtlas, make_player_billboard_atlas,
    resolve_billboard,
};
use carcinisation_fps::player_attack::{
    AttackInput, AttackLoadout, PlayerAttackSprites, PlayerAttackState, RemoteFlameConfig,
};
use carcinisation_fps::plugin::CharDecals;
use carcinisation_fps::plugin::{
    Active, BloodShotSprites, CameraRes, CameraShakeState, Config, DeathViewState, MapRes,
    MosquitonSprites, Systems, request_camera_shake,
};
use carcinisation_fps::raycast::{HitSide, WallSurfaceId};
use carcinisation_fps::render::CharDecal;
use carcinisation_net::components::NetEnemy;
use carcinisation_net::{
    DamageEffect, DeathEffect, EnemyAttackKind, EnemyAttackVisual, FlameActive, FlameCharMark,
    HitConfirm, MuzzleFlash, NetAttackId, NetEnemyState, NetEnemyType, NetHealth, NetPlayer,
    NetProjectile, NetProtocolPlugin, NetworkObjectId, PlayerId, PlayerIdAssigned,
    register_net_all,
};
use std::net::SocketAddr;
#[cfg(not(target_family = "wasm"))]
use std::time::{SystemTime, UNIX_EPOCH};

pub mod input;
pub use input::{ClientInputSequence, collect_and_send_intent};

/// Tracks which `PlayerId` belongs to this client.
#[derive(Resource, Debug, Default)]
pub struct LocalPlayerId(pub Option<PlayerId>);

/// Client connection state machine.
///
/// Transitions:
///   Connecting ──(PlayerIdAssigned)──→ Connected
///   Connecting ──(timeout/error)─────→ Failed
///   Connected  ──(transport drop)────→ Disconnected
#[derive(Resource, Debug, Clone)]
pub enum ConnectionState {
    Connecting {
        addr: SocketAddr,
        start_time: std::time::Instant,
    },
    Connected,
    Failed {
        reason: String,
    },
    Disconnected {
        reason: String,
    },
}

const CONNECTION_TIMEOUT_SECS: f64 = 15.0;

const SHOW_NET_INFO_ENV: &str = "CARCINISATION_SHOW_NET_INFO";

/// Toggle resource for the FPS/ping/connection HUD.
#[derive(Resource, Debug)]
struct NetInfoVisible(bool);

impl Default for NetInfoVisible {
    fn default() -> Self {
        let enabled = std::env::var(SHOW_NET_INFO_ENV)
            .ok()
            .and_then(|v| match v.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            })
            .unwrap_or(false);
        Self(enabled)
    }
}

/// Whether the local player's flamethrower is currently active (from server `FlameActive` event).
#[derive(Resource, Debug, Default)]
struct LocalFlameActive(bool);

/// Projectile visual speed for client-side extrapolation.
const PROJECTILE_VISUAL_SPEED: f32 = carcinisation_fps_core::PROJECTILE_SPEED;

/// Fallback shoot animation duration if sprites not loaded.
const SHOOT_ANIM_DURATION_FALLBACK: f32 = 0.2;
/// Fallback melee animation duration if sprites not loaded.
const MELEE_ANIM_DURATION_FALLBACK: f32 = 0.3;

/// Client-side one-shot animation override per enemy, triggered by `EnemyAttackVisual`.
#[derive(Resource, Debug, Default)]
struct EnemyAttackOverrides(std::collections::HashMap<NetworkObjectId, AttackAnimOverride>);

/// Client-side hit impact billboards (blood splats at hit locations).
#[derive(Resource, Debug, Default)]
struct HitImpacts(Vec<HitImpact>);

#[derive(Debug, Clone)]
struct HitImpact {
    position: Vec2,
    age: f32,
    lifetime: f32,
    kind: carcinisation_net::HitImpactKind,
}

use carcinisation_net::HitImpactKind;

/// Matches SP `ProjectileImpact::hit` lifetime.
const HIT_IMPACT_LIFETIME: f32 = 0.18;
/// Matches SP `ProjectileImpact::destroy` lifetime.
const DESTROY_IMPACT_LIFETIME: f32 = 0.3;

/// Client-side damage flicker tracker per enemy, triggered by `DamageEffect`.
#[derive(Resource, Debug, Default)]
struct EnemyDamageFlickers(
    std::collections::HashMap<NetworkObjectId, carcinisation_fps_core::enemy::DamageFlicker>,
);

#[derive(Debug, Clone, Copy)]
struct AttackAnimOverride {
    kind: EnemyAttackKind,
    /// Time elapsed since override started (for `sprite_at` sampling).
    elapsed: f32,
    /// Total animation duration.
    duration: f32,
}

pub struct FpsClientPlugin {
    pub connect_addr: SocketAddr,
}

impl Plugin for FpsClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetProtocolPlugin)
            .add_plugins(RepliconSharedPlugin {
                auth_method: AuthMethod::None,
            })
            .add_plugins(ClientPlugin)
            .add_plugins(ClientMessagePlugin);

        register_net_all(app);

        if !app.is_plugin_added::<bevy::diagnostic::FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
        }
        app.add_plugins(bevy_replicon_renet2::RepliconRenetPlugins)
            .init_resource::<input::ClientInputSequence>()
            .init_resource::<input::InputSendTimer>()
            .init_resource::<carcinisation_fps::plugin::TurnChordState>()
            .init_resource::<carcinisation_fps::plugin::QuickTurnState>()
            .init_resource::<LocalPlayerId>()
            .init_resource::<LocalFlameActive>()
            .init_resource::<EnemyAttackOverrides>()
            .init_resource::<EnemyDamageFlickers>()
            .init_resource::<HitImpacts>()
            .insert_resource(ConnectionState::Connecting {
                addr: self.connect_addr,
                start_time: std::time::Instant::now(),
            })
            .add_observer(handle_player_id_assigned)
            .add_observer(handle_muzzle_flash)
            .add_observer(handle_damage_effect)
            .add_observer(handle_death_effect)
            .add_observer(handle_flame_active)
            .add_observer(handle_flame_char_mark)
            .add_observer(handle_enemy_attack_visual)
            .add_observer(handle_hit_confirm)
            .add_systems(Update, collect_and_send_intent.run_if(is_connected))
            .add_systems(
                Update,
                (
                    tick_attack_overrides,
                    tick_damage_flickers,
                    tick_hit_impacts,
                    sync_player_lifecycle_state,
                    sync_camera_from_net_player,
                )
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            .add_systems(
                Update,
                sync_weapon_hud_and_flame_visual
                    .before(Systems)
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            );

        info!("FpsClientPlugin: connect addr = {:?}", self.connect_addr);

        #[cfg(not(target_family = "wasm"))]
        {
            let _ = dotenvy::dotenv_override();
            app.insert_resource(ConnectAddr(self.connect_addr));
            app.init_resource::<NetInfoVisible>();
        }
        #[cfg(not(target_family = "wasm"))]
        {
            use init_client_setup as _init;
            app.add_systems(Startup, _init);
        }
        #[cfg(not(target_family = "wasm"))]
        {
            use setup_client_info_text as _info;
            app.add_systems(Startup, _info);
        }
        #[cfg(not(target_family = "wasm"))]
        {
            use kickstart_client_transport as _kick;
            app.add_systems(
                PreUpdate,
                _kick
                    .run_if(resource_added::<bevy_replicon_renet2::renet2::RenetClient>)
                    .before(bevy_renet2::prelude::RenetReceive),
            );
        }
        #[cfg(not(target_family = "wasm"))]
        {
            use monitor_connection as _monitor;
            app.add_systems(Update, _monitor);
        }
        #[cfg(not(target_family = "wasm"))]
        {
            app.add_systems(Update, (toggle_net_info, update_client_info_text));
        }
        // Overlay controls visibility — shown during non-Connected states.
    }
}

#[derive(Resource)]
#[cfg(not(target_family = "wasm"))]
struct ConnectAddr(SocketAddr);

fn handle_player_id_assigned(
    trigger: On<PlayerIdAssigned>,
    mut local_id: ResMut<LocalPlayerId>,
    mut connection_state: ResMut<ConnectionState>,
) {
    local_id.0 = Some(trigger.0);
    if !matches!(*connection_state, ConnectionState::Connected) {
        *connection_state = ConnectionState::Connected;
        info!("Connection established — PlayerId {:?}", trigger.0);
    }
}

fn handle_muzzle_flash(
    trigger: On<MuzzleFlash>,
    local_id: Res<LocalPlayerId>,
    mut attack_state: ResMut<PlayerAttackState>,
) {
    let flash = trigger.event();
    if local_id.0 == Some(flash.player_id) {
        attack_state.trigger_muzzle_flash();
    }
}

/// Hit confirmation — creates an impact billboard at the hit location.
/// Distinguishes enemy hits (static blood splat) from projectile destroy (animated).
/// Create impact billboard directly from event position — no entity lookup needed.
fn handle_hit_confirm(trigger: On<HitConfirm>, mut impacts: ResMut<HitImpacts>) {
    let confirm = trigger.event();
    let (lifetime, kind) = match confirm.kind {
        HitImpactKind::Hit => (HIT_IMPACT_LIFETIME, HitImpactKind::Hit),
        HitImpactKind::Destroy => (DESTROY_IMPACT_LIFETIME, HitImpactKind::Destroy),
    };
    impacts.0.push(HitImpact {
        position: confirm.position,
        age: 0.0,
        lifetime,
        kind,
    });
}

fn handle_damage_effect(
    trigger: On<DamageEffect>,
    local_id: Res<LocalPlayerId>,
    mut camera_shake: ResMut<CameraShakeState>,
    config: Res<Config>,
    mut flickers: ResMut<EnemyDamageFlickers>,
    mut health: ResMut<carcinisation_fps::plugin::PlayerHealth>,
) {
    let effect = trigger.event();
    debug!(
        "DamageEffect: target={:?} damage={:.0} remaining={:.0}",
        effect.target_id, effect.damage, effect.remaining_health
    );

    // Trigger screen shake and sync health for the local player.
    let is_local = local_id.0.is_some_and(|pid| effect.target_id.0 == pid.0);
    if is_local {
        request_camera_shake(&mut camera_shake, &config);
        health.0 = effect.remaining_health.ceil() as u32;
    }

    // Trigger/restart damage flicker for enemies (not players).
    if !effect.was_player {
        flickers.0.insert(
            effect.target_id,
            carcinisation_fps_core::enemy::DamageFlicker::new(),
        );
    }
}

fn handle_death_effect(
    trigger: On<DeathEffect>,
    local_id: Res<LocalPlayerId>,
    mut dead: ResMut<carcinisation_fps::plugin::PlayerDead>,
    mut death_view: ResMut<carcinisation_fps::plugin::DeathViewState>,
    camera: Res<CameraRes>,
    net_enemies: Query<&NetEnemy>,
) {
    let effect = trigger.event();
    debug!(
        "DeathEffect: target={:?} was_player={}",
        effect.target_id, effect.was_player
    );

    // Trigger death screen for local player.
    if effect.was_player {
        let is_local = local_id.0.is_some_and(|pid| effect.target_id.0 == pid.0);
        if is_local {
            dead.0 = true;
            // Find nearest enemy as the "killer" for death view rotation.
            let killer_pos = net_enemies
                .iter()
                .map(|e| (e.position, e.position.distance(camera.0.position)))
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(pos, _)| pos);
            if let Some(source) = killer_pos {
                carcinisation_fps::plugin::request_death_view(&mut death_view, &camera.0, source);
            }
        }
    }
}

/// Handles `FlameActive` unreliable events for immediate VFX responsiveness.
///
/// Remote flame state is authoritative via `NetPlayer.flame_active` (replicated).
/// This event handler only updates the local player's `LocalFlameActive` for
/// instant HUD feedback — remote players reconcile against the replicated field.
fn handle_flame_active(
    trigger: On<FlameActive>,
    local_id: Res<LocalPlayerId>,
    mut flame_active: ResMut<LocalFlameActive>,
) {
    let event = trigger.event();
    if local_id.0 == Some(event.player_id) {
        flame_active.0 = event.active;
    }
    // Remote player flame state comes from NetPlayer.flame_active (replicated).
    // Unreliable events for remote players are intentionally ignored — the
    // replicated component is the authoritative reconciliation source.
}

/// Maximum number of char decals to keep.
const MAX_FLAME_CHAR_DECALS: usize = 128;

fn handle_flame_char_mark(trigger: On<FlameCharMark>, mut char_decals: ResMut<CharDecals>) {
    let mark = trigger.event();
    let surface_id = WallSurfaceId {
        cell_x: mark.cell_x,
        cell_y: mark.cell_y,
        side: if mark.side == 0 {
            HitSide::Vertical
        } else {
            HitSide::Horizontal
        },
        normal_sign: mark.normal_sign,
    };

    // Dedup: skip if a nearby decal already exists on the same surface.
    let dominated = char_decals
        .0
        .iter()
        .rev()
        .take(12)
        .any(|d| d.surface_id == surface_id && (d.u - mark.u).abs() < 0.025);
    if dominated {
        return;
    }

    let flip_x = mark.seed & 1 != 0;
    let flip_y = mark.seed & 2 != 0;
    let intensity = if mark.seed & 4 != 0 { 0.88 } else { 0.58 };

    char_decals.0.push(CharDecal {
        surface_id,
        u: mark.u,
        v: 0.5,
        width: 0.30,
        height: 0.30,
        intensity,
        flip_x,
        flip_y,
        seed: mark.seed,
    });

    // Cap total decals.
    if char_decals.0.len() > MAX_FLAME_CHAR_DECALS {
        let excess = char_decals.0.len() - MAX_FLAME_CHAR_DECALS;
        char_decals.0.drain(..excess);
    }
}

fn handle_enemy_attack_visual(
    trigger: On<EnemyAttackVisual>,
    mut overrides: ResMut<EnemyAttackOverrides>,
    sprites: Option<Res<MosquitonSprites>>,
) {
    let event = trigger.event();
    let duration = match event.kind {
        EnemyAttackKind::Ranged => sprites
            .as_ref()
            .map_or(SHOOT_ANIM_DURATION_FALLBACK, |s| s.0.shoot_duration()),
        EnemyAttackKind::Melee => sprites
            .as_ref()
            .map_or(MELEE_ANIM_DURATION_FALLBACK, |s| s.0.melee_duration()),
    };
    overrides.0.insert(
        event.object_id,
        AttackAnimOverride {
            kind: event.kind,
            elapsed: 0.0,
            duration,
        },
    );
}

/// Tick attack animation overrides — advance elapsed and remove expired entries.
fn tick_attack_overrides(mut overrides: ResMut<EnemyAttackOverrides>, time: Res<Time>) {
    let dt = time.delta_secs();
    overrides.0.retain(|_, o| {
        o.elapsed += dt;
        o.elapsed < o.duration
    });
}

/// Tick hit impact lifetimes and remove expired ones.
fn tick_hit_impacts(mut impacts: ResMut<HitImpacts>, time: Res<Time>) {
    let dt = time.delta_secs();
    for impact in &mut impacts.0 {
        impact.age += dt;
    }
    impacts.0.retain(|i| i.age < i.lifetime);
}

/// Tick damage flicker timers — advance phase and remove finished flickers.
fn tick_damage_flickers(mut flickers: ResMut<EnemyDamageFlickers>, time: Res<Time>) {
    let dt = time.delta_secs();
    flickers.0.retain(|_, flicker| {
        if let Some(next) = (*flicker).tick(dt) {
            *flicker = next;
            true
        } else {
            false
        }
    });
}

/// Sync the client weapon HUD from replicated state and drive local flamethrower visuals.
///
/// Local flame state uses two sources:
/// - `LocalFlameActive`: set by unreliable `FlameActive` events (immediate, may be lost)
/// - `player.flame_active`: replicated field on `NetPlayer` (authoritative, eventual)
///
/// The replicated field acts as a reconciliation fallback — if an event is
/// dropped, the next replication tick corrects the local state.
fn sync_weapon_hud_and_flame_visual(
    net_players: Query<&NetPlayer>,
    local_id: Res<LocalPlayerId>,
    mut loadout: ResMut<AttackLoadout>,
    mut attack_input: ResMut<AttackInput>,
    mut flame_active: ResMut<LocalFlameActive>,
    mut was_flame_active: Local<bool>,
) {
    let Some(my_id) = local_id.0 else {
        *was_flame_active = false;
        return;
    };
    let Some(player) = net_players.iter().find(|p| p.player_id == my_id) else {
        *was_flame_active = false;
        return;
    };
    let target_idx = match player.current_attack {
        NetAttackId::Projectile => 0,                // Flamethrower
        NetAttackId::None | NetAttackId::Melee => 1, // Pistol (Melee falls back)
    };
    if loadout.index != target_idx {
        loadout.index = target_idx;
    }

    // Reconcile: replicated state overrides event-driven state if they disagree.
    // This recovers from dropped FlameActive events within one replication tick.
    if flame_active.0 != player.flame_active {
        flame_active.0 = player.flame_active;
    }

    let active = flame_active.0 && matches!(player.current_attack, NetAttackId::Projectile);
    if active {
        attack_input.shoot_held = true;
        if !*was_flame_active {
            attack_input.shoot_just_pressed = true;
        }
    } else {
        attack_input.shoot_held = false;
        if *was_flame_active {
            attack_input.shoot_just_released = true;
        }
    }
    *was_flame_active = active;
}

/// Run condition: only run gameplay systems when fully connected.
fn is_connected(state: Res<ConnectionState>) -> bool {
    matches!(*state, ConnectionState::Connected)
}

/// Monitor renet client state and drive connection lifecycle.
///
/// Transitions:
/// - `Connecting` → `Failed` on transport disconnect or timeout
/// - `Connected` → `Disconnected` on transport drop
#[cfg(not(target_family = "wasm"))]
fn monitor_connection(
    client: Res<RenetClient>,
    mut connection_state: ResMut<ConnectionState>,
    mut local_id: ResMut<LocalPlayerId>,
) {
    let disconnected = client.is_disconnected();
    match &*connection_state {
        ConnectionState::Connecting { addr, start_time } => {
            if disconnected {
                let reason = "Connection refused".to_string();
                error!("Failed to connect to {addr}: {reason}");
                *connection_state = ConnectionState::Failed { reason };
                local_id.0 = None;
                return;
            }
            if start_time.elapsed().as_secs_f64() > CONNECTION_TIMEOUT_SECS {
                let reason = format!("Timed out after {CONNECTION_TIMEOUT_SECS}s");
                error!("Connection to {addr} {reason}");
                *connection_state = ConnectionState::Failed { reason };
                local_id.0 = None;
            }
        }
        ConnectionState::Connected => {
            if disconnected {
                let reason = "Connection lost".to_string();
                warn!("{reason}");
                *connection_state = ConnectionState::Disconnected { reason };
                local_id.0 = None;
            }
        }
        ConnectionState::Failed { .. } | ConnectionState::Disconnected { .. } => {}
    }
}

#[cfg(not(target_family = "wasm"))]
fn kickstart_client_transport(
    mut client: ResMut<RenetClient>,
    mut transport: ResMut<NetcodeClientTransport>,
    time: Res<bevy::time::Time<bevy::time::Real>>,
) {
    if let Err(e) = transport.update(time.delta(), &mut client) {
        eprintln!("CLIENT kickstart transport error: {e:?}");
    }
}

#[cfg(not(target_family = "wasm"))]
fn init_client_setup(
    mut commands: Commands,
    connect_addr: Res<ConnectAddr>,
    channels: Res<RepliconChannels>,
    mut connection_state: ResMut<ConnectionState>,
) {
    // Reset start_time — build() captured Instant::now() during plugin ctor,
    // but app init (asset loading, shader compilation) may have taken seconds.
    if let ConnectionState::Connecting { start_time, .. } = &mut *connection_state {
        *start_time = std::time::Instant::now();
    }
    use bevy_renet2::netcode::{ClientAuthentication, NativeSocket};

    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");

    let server_configs = channels.server_configs();
    let client_configs = channels.client_configs();

    let connection_config = ConnectionConfig::from_channels(server_configs, client_configs);

    let local_addr =
        std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0);
    let socket = NativeSocket::new(std::net::UdpSocket::bind(local_addr).expect("bind"))
        .expect("create socket");

    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: carcinisation_net::PROTOCOL_ID,
        socket_id: 0,
        server_addr: connect_addr.0,
        user_data: None,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket)
        .expect("create client transport");

    let client = RenetClient::new(connection_config, transport.is_reliable());

    commands.insert_resource(client);
    commands.insert_resource(transport);

    info!(
        "Client connecting to {} (UDP, client_id={})",
        connect_addr.0, client_id
    );
}

#[allow(clippy::too_many_arguments)]
/// Sync local `PlayerDead` / `PlayerHealth` from replicated `NetPlayer` state.
fn sync_player_lifecycle_state(
    net_players: Query<&NetPlayer>,
    local_player_id: Res<LocalPlayerId>,
    mut player_dead: ResMut<carcinisation_fps::plugin::PlayerDead>,
    mut player_health: ResMut<carcinisation_fps::plugin::PlayerHealth>,
    mut death_view: ResMut<carcinisation_fps::plugin::DeathViewState>,
    mut camera_shake: ResMut<CameraShakeState>,
    fps_config: Res<Config>,
) {
    let Some(my_id) = local_player_id.0 else {
        return;
    };
    let Some(local_np) = net_players.iter().find(|p| p.player_id == my_id) else {
        return;
    };
    match &local_np.state {
        carcinisation_net::PlayerNetState::Alive => {
            if player_dead.0 {
                // Respawn — reset all death/damage visual state.
                player_dead.0 = false;
                player_health.0 = fps_config.player_max_health;
                *death_view = DeathViewState::default();
                *camera_shake = CameraShakeState::default();
            }
        }
        carcinisation_net::PlayerNetState::Dead => {
            if !player_dead.0 {
                // Just died — stop camera shake so it doesn't fight the death view.
                *camera_shake = CameraShakeState::default();
            }
            player_dead.0 = true;
        }
    }
}

/// Cached player billboard atlas + per-player animation state.
#[derive(Default)]
struct SyncLocals {
    has_set_camera: bool,
    last_log_frame: u32,
    /// Directional billboard atlas for remote player rendering.
    player_billboard_atlas: Option<DirectionalBillboardAtlas>,
    /// Set after one failed load attempt so fallback rendering does not retry
    /// and warn every frame.
    player_billboard_atlas_failed: bool,
    /// Per-player animation states, keyed by `PlayerId`.
    player_anim_states: std::collections::HashMap<PlayerId, BillboardAnimationState>,
    /// Smoothed speed for walk detection (avoids flicker from tick-rate jitter).
    player_smoothed_speed: std::collections::HashMap<PlayerId, (Vec2, f32)>,
    /// Fallback placeholder sprites if atlas loading fails.
    fallback_sprites: Option<[Arc<carapace::image::CxImage>; 4]>,
    /// Remote flame billboard config (loaded once from RON).
    remote_flame_config: Option<RemoteFlameConfig>,
}

/// Smoothing alpha for walk detection. Lower = smoother but laggier.
const WALK_SMOOTH_ALPHA: f32 = 0.15;
/// Speed to START walking animation (world units/sec).
const WALK_START_THRESHOLD: f32 = 0.3;
/// Speed to STOP walking animation. Lower than start for hysteresis.
const WALK_STOP_THRESHOLD: f32 = 0.1;

fn anim_state_is_walking(state: Option<&BillboardAnimationState>) -> bool {
    state.is_some_and(|s| s.action == "walk_forward")
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn sync_camera_from_net_player(
    net_players: Query<(Entity, &NetPlayer)>,
    net_enemies: Query<(&NetEnemy, Option<&NetHealth>)>,
    net_projectiles: Query<&NetProjectile>,
    mut camera_res: ResMut<CameraRes>,
    mut extra_bbs: ResMut<carcinisation_fps::plugin::ExtraBillboards>,
    local_player_id: Res<LocalPlayerId>,
    map_res: Option<Res<MapRes>>,
    mosquiton_sprites: Option<Res<MosquitonSprites>>,
    blood_shot_sprites: Option<Res<BloodShotSprites>>,
    attack_sprites: Option<Res<PlayerAttackSprites>>,
    attack_overrides: Res<EnemyAttackOverrides>,
    damage_flickers: Res<EnemyDamageFlickers>,
    hit_impacts: Res<HitImpacts>,
    mut sync_locals: Local<SyncLocals>,
    time: Res<Time>,
) {
    let Some(my_id) = local_player_id.0 else {
        return;
    };

    let local_player = net_players.iter().find(|(_, p)| p.player_id == my_id);
    let Some((local_entity, local_np)) = local_player else {
        return;
    };

    camera_res.0.position = local_np.position;
    camera_res.0.angle = local_np.angle;

    extra_bbs.0.clear();
    let elapsed = time.elapsed_secs();
    let dt = time.delta_secs();

    // Lazy-init remote flame config from RON.
    if sync_locals.remote_flame_config.is_none() {
        const RON_PATH: &str = "assets/config/attacks/remote_flame.ron";
        sync_locals.remote_flame_config = Some(match std::fs::read_to_string(RON_PATH) {
            Ok(s) => ron::from_str(&s).unwrap_or_else(|err| {
                warn!("[NET] failed to parse {RON_PATH}: {err}, using defaults");
                RemoteFlameConfig::default()
            }),
            Err(err) => {
                warn!("[NET] failed to read {RON_PATH}: {err}, using defaults");
                RemoteFlameConfig::default()
            }
        });
    }

    // Lazy-init directional billboard atlas and fallback sprites.
    if sync_locals.player_billboard_atlas.is_none() && !sync_locals.player_billboard_atlas_failed {
        match make_player_billboard_atlas() {
            Ok(atlas) => {
                sync_locals.player_billboard_atlas = Some(atlas);
            }
            Err(err) => {
                warn!("[NET] failed to load player billboard atlas: {err}");
                sync_locals.player_billboard_atlas_failed = true;
            }
        }
    }
    if sync_locals.fallback_sprites.is_none() {
        sync_locals.fallback_sprites = Some(std::array::from_fn(|i| {
            Arc::new(make_enemy_sprite(32, i as u8 + 1))
        }));
    }

    // Collect remote player data first to avoid borrow conflicts.
    struct RemotePlayerInfo {
        player_id: PlayerId,
        position: Vec2,
        angle: f32,
        state: carcinisation_net::PlayerNetState,
        current_attack: NetAttackId,
        flame_active: bool,
    }
    let remote_players: Vec<RemotePlayerInfo> = net_players
        .iter()
        .filter(|(_, p)| p.player_id != my_id)
        .map(|(_, np)| RemotePlayerInfo {
            player_id: np.player_id,
            position: np.position,
            angle: np.angle,
            state: np.state.clone(),
            current_attack: np.current_attack,
            flame_active: np.flame_active,
        })
        .collect();

    let mut seen_player_ids: Vec<PlayerId> = Vec::new();

    for rp in &remote_players {
        seen_player_ids.push(rp.player_id);

        // Determine animation action from player state and movement.
        // Use exponentially smoothed speed to avoid walk/idle flicker from
        // network tick-rate jitter.
        let currently_walking =
            anim_state_is_walking(sync_locals.player_anim_states.get(&rp.player_id));
        let action = match rp.state {
            carcinisation_net::PlayerNetState::Dead => "death",
            carcinisation_net::PlayerNetState::Alive => {
                let (prev_pos, smoothed_speed) = sync_locals
                    .player_smoothed_speed
                    .entry(rp.player_id)
                    .or_insert((rp.position, 0.0));
                let instant_speed = if dt > 0.0 {
                    prev_pos.distance(rp.position) / dt
                } else {
                    0.0
                };
                *smoothed_speed =
                    *smoothed_speed * (1.0 - WALK_SMOOTH_ALPHA) + instant_speed * WALK_SMOOTH_ALPHA;
                *prev_pos = rp.position;
                let threshold = if currently_walking {
                    WALK_STOP_THRESHOLD
                } else {
                    WALK_START_THRESHOLD
                };
                if *smoothed_speed > threshold {
                    "walk_forward"
                } else {
                    "idle_stand"
                }
            }
        };

        // Update or create animation state for this player.
        let anim_state = sync_locals
            .player_anim_states
            .entry(rp.player_id)
            .or_insert_with(|| BillboardAnimationState::new("idle_stand"));
        anim_state.set_action(action);
        anim_state.tick(dt);

        // Snapshot animation state for billboard resolution (avoids borrow conflict).
        let anim_snapshot = anim_state.clone();

        // Try directional billboard resolution.
        let mut pushed = false;
        if let Some(atlas) = &sync_locals.player_billboard_atlas
            && let Some(resolved) = resolve_billboard(
                atlas,
                local_np.position,
                rp.position,
                rp.angle,
                &anim_snapshot,
            )
        {
            // height shifts the billboard vertically. Negative = feet toward floor.
            // Formula: -(0.5 - world_height/2) grounds feet at floor level.
            let world_height = 0.65;
            extra_bbs.0.push(Billboard {
                position: rp.position,
                height: -(0.5 - world_height / 2.0),
                world_height,
                sprite: resolved.sprite,
                flip_x: resolved.flip_x,
            });
            pushed = true;
        }

        if !pushed {
            // Fallback: use placeholder diamond sprites.
            if let Some(fallback) = &sync_locals.fallback_sprites {
                let color_idx = (rp.player_id.0.wrapping_sub(1) % 4) as usize;
                extra_bbs.0.push(Billboard {
                    position: rp.position,
                    height: 0.0,
                    world_height: 1.5,
                    sprite: Arc::clone(&fallback[color_idx]),
                    flip_x: false,
                });
            }
        }

        // Remote flame arc billboards (authoritative: replicated NetPlayer.flame_active
        // AND must be using the flamethrower weapon).
        if rp.flame_active
            && matches!(
                rp.current_attack,
                carcinisation_net::NetAttackId::Projectile
            )
        {
            let default_flame_cfg = RemoteFlameConfig::default();
            let flame_cfg = sync_locals
                .remote_flame_config
                .as_ref()
                .unwrap_or(&default_flame_cfg);
            push_remote_flame_billboards(
                &mut extra_bbs.0,
                rp.position,
                rp.angle,
                elapsed,
                attack_sprites.as_deref(),
                map_res.as_deref(),
                flame_cfg,
            );
        }
    }

    // Prune stale animation states for disconnected players.
    sync_locals
        .player_anim_states
        .retain(|id, _| seen_player_ids.contains(id));
    sync_locals
        .player_smoothed_speed
        .retain(|id, _| seen_player_ids.contains(id));

    // Enemy billboards (including dying/dead for death pose, excludes despawned).
    for (enemy, _health) in net_enemies.iter() {
        let show_invert = damage_flickers
            .0
            .get(&enemy.object_id)
            .is_some_and(|f| f.showing_invert());
        extra_bbs.0.push(net_enemy_billboard(
            enemy,
            elapsed,
            mosquiton_sprites.as_deref(),
            attack_overrides.0.get(&enemy.object_id),
            show_invert,
        ));

        // Burn flame effect for enemies killed by fire (persists through Dead until despawn).
        if matches!(
            enemy.state,
            NetEnemyState::Dying { burn: true } | NetEnemyState::Dead { burn: true }
        ) && let Some(sprites) = mosquiton_sprites.as_deref()
        {
            let corpse_sprite = sprites.0.alive_sprite_at(0.0);
            push_net_burn_flames(
                &mut extra_bbs.0,
                enemy.position,
                enemy.object_id.0,
                elapsed,
                camera_res.0.position,
                camera_res.0.direction(),
                corpse_sprite,
                attack_sprites.as_deref(),
            );
        }
    }

    // Projectile billboards (extrapolated forward by one frame for smoothness).
    // Clamp dt to 50ms so low-FPS spikes or browser stalls don't cause large visual jumps.
    let frame_dt = time.delta_secs().min(0.05);
    for proj in net_projectiles.iter() {
        let dir = Vec2::new(proj.angle.cos(), proj.angle.sin());
        let extrapolated = proj.position + dir * PROJECTILE_VISUAL_SPEED * frame_dt;
        let sprite = blood_shot_sprites.as_ref().map_or_else(
            || Arc::new(make_blood_shot_sprite(8, 3)),
            |bs| Arc::clone(&bs.0.hover),
        );
        extra_bbs.0.push(Billboard {
            position: extrapolated,
            height: 0.0,
            world_height: 0.3,
            sprite,
            flip_x: false,
        });
    }

    // Hit impact billboards (blood splats / destroy animations).
    for impact in &hit_impacts.0 {
        let (sprite, world_height) = match impact.kind {
            HitImpactKind::Hit => {
                let s = blood_shot_sprites.as_ref().map_or_else(
                    || Arc::new(make_blood_shot_sprite(8, 3)),
                    |bs| Arc::clone(&bs.0.hit),
                );
                (s, 0.42)
            }
            HitImpactKind::Destroy => {
                let s = blood_shot_sprites.as_ref().map_or_else(
                    || Arc::new(make_blood_shot_sprite(8, 3)),
                    |bs| Arc::clone(bs.0.destroy_sprite_at(impact.age)),
                );
                (s, 0.36)
            }
        };
        extra_bbs.0.push(Billboard {
            position: impact.position,
            height: 0.15,
            world_height,
            sprite,
            flip_x: false,
        });
    }

    let frame = time.elapsed_secs_f64() as u32;
    if !sync_locals.has_set_camera || frame - sync_locals.last_log_frame > 120 {
        sync_locals.last_log_frame = frame;
        let total = net_players.iter().count();
        let remote_count = extra_bbs.0.len();
        info!(
            "[NET] local={:?} entity={:?} pos={:?} angle={:.2} total_players={} remote_billboards={}",
            my_id, local_entity, local_np.position, local_np.angle, total, remote_count
        );
        for (entity, np) in net_players.iter() {
            let is_local = np.player_id == my_id;
            info!(
                "[NET]   entity={:?} PlayerId={:?} pos={:?} angle={:.2} local={}",
                entity, np.player_id, np.position, np.angle, is_local
            );
        }
    }

    if !sync_locals.has_set_camera {
        sync_locals.has_set_camera = true;
    }
}

fn net_enemy_billboard(
    enemy: &NetEnemy,
    elapsed_secs: f32,
    mosquiton_sprites: Option<&MosquitonSprites>,
    attack_override: Option<&AttackAnimOverride>,
    damage_invert: bool,
) -> Billboard {
    match enemy.enemy_type {
        NetEnemyType::Basic => Billboard {
            position: enemy.position,
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_enemy_sprite(32, 2)),
            flip_x: false,
        },
        NetEnemyType::Mosquiton => Billboard {
            position: enemy.position,
            height: 0.0,
            world_height: 0.9,
            sprite: mosquiton_sprites.map_or_else(
                || Arc::new(make_mosquiton_placeholder_sprite(32, 2)),
                |sprites| {
                    // Dying/Dead: no flicker, no attack override.
                    match enemy.state {
                        NetEnemyState::Dying { burn: false }
                        | NetEnemyState::Dead { burn: false } => {
                            return Arc::clone(&sprites.0.death);
                        }
                        NetEnemyState::Dying { burn: true }
                        | NetEnemyState::Dead { burn: true } => {
                            return Arc::new(carcinisation_fps::billboard::make_charred_sprite(
                                sprites.0.alive_sprite_at(0.0),
                            ));
                        }
                        _ => {}
                    }
                    // Select base sprite from attack override or idle state.
                    if let Some(anim) = attack_override {
                        let sprite = match anim.kind {
                            EnemyAttackKind::Melee => sprites.0.melee_sprite_at(anim.elapsed),
                            EnemyAttackKind::Ranged => sprites.0.shoot_sprite_at(anim.elapsed),
                        };
                        if damage_invert {
                            Arc::new(make_damage_invert_sprite(sprite))
                        } else {
                            Arc::clone(sprite)
                        }
                    } else {
                        let sprite = sprites.0.alive_sprite_at(elapsed_secs);
                        if damage_invert {
                            Arc::new(make_damage_invert_sprite(sprite))
                        } else {
                            Arc::clone(sprite)
                        }
                    }
                },
            ),
            flip_x: false,
        },
    }
}

/// Generate perimeter flame billboards around a burning corpse, matching the
/// singleplayer technique: sample the sprite's opaque perimeter and place
/// flame sprites along the edges with deterministic seeding.
/// Uses camera direction for lateral spread (same as SP `push_burning_corpse_flames`).
fn push_net_burn_flames(
    billboards: &mut Vec<Billboard>,
    position: Vec2,
    seed: u32,
    elapsed: f32,
    camera_pos: Vec2,
    camera_dir: Vec2,
    corpse_sprite: &carapace::image::CxImage,
    attack_sprites: Option<&PlayerAttackSprites>,
) {
    use carapace::palette::TRANSPARENT_INDEX;
    use carcinisation_fps_core::fire_death::{FireDeathConfig, perimeter_flames_from_mask};

    let config = FireDeathConfig::default();
    let flames = perimeter_flames_from_mask(
        seed,
        corpse_sprite.width(),
        corpse_sprite.height(),
        |x, y| corpse_sprite.data()[y * corpse_sprite.width() + x] != TRANSPARENT_INDEX,
        &config,
    );
    if flames.is_empty() {
        return;
    }

    // Camera-relative vectors (same as SP push_burning_corpse_flames).
    let to_corpse = position - camera_pos;
    let distance = to_corpse.length().max(0.1);
    let behind_dir = if distance > 0.001 {
        to_corpse / distance
    } else {
        camera_dir
    };
    let right = Vec2::new(-camera_dir.y, camera_dir.x);

    let base_world_height: f32 = 0.9;
    let px_to_world = base_world_height / corpse_sprite.height() as f32;

    for flame in &flames {
        let lateral_units = flame.offset_px.x * px_to_world;
        let vertical_units = flame.offset_px.y * px_to_world;
        let phase = elapsed + flame.phase_secs;
        let sprite = attack_sprites.map_or_else(
            || Arc::new(make_blood_shot_sprite(6, 3)),
            |sprites| Arc::clone(sprites.flame_frame_loop(phase)),
        );
        billboards.push(Billboard {
            position: position + behind_dir * 0.04 + right * lateral_units,
            height: vertical_units,
            world_height: base_world_height * 0.35 * flame.scale,
            sprite,
            flip_x: false,
        });
    }
}

/// Generate flame arc billboards for a remote player's active flamethrower.
///
/// Config loaded from `assets/config/attacks/remote_flame.ron`.
fn push_remote_flame_billboards(
    billboards: &mut Vec<Billboard>,
    position: Vec2,
    angle: f32,
    elapsed: f32,
    attack_sprites: Option<&PlayerAttackSprites>,
    map: Option<&MapRes>,
    cfg: &RemoteFlameConfig,
) {
    use carcinisation_fps::raycast::cast_ray;

    let dir = Vec2::new(angle.cos(), angle.sin());
    let right = Vec2::new(-dir.y, dir.x);

    let max_dist = map.map_or(cfg.flame_end, |m| {
        let hit = cast_ray(&m.0, position, dir);
        if hit.wall_id > 0 {
            hit.distance.min(cfg.flame_end)
        } else {
            cfg.flame_end
        }
    });

    let seg_max = cfg.segment_count.max(2);
    for i in 0..seg_max {
        let t = i as f32 / (seg_max - 1) as f32;
        let dist = cfg.flame_start + (cfg.flame_end - cfg.flame_start) * t;

        if dist >= max_dist {
            break;
        }

        let phase = elapsed + i as f32 * cfg.phase_step;
        let jitter = ((i as f32 * 7.31).sin() * cfg.jitter_amp) * t;

        let sprite = attack_sprites.map_or_else(
            || Arc::new(make_blood_shot_sprite(6, 3)),
            |sprites| Arc::clone(sprites.flame_frame_loop(phase)),
        );
        billboards.push(Billboard {
            position: position + dir * dist + right * jitter,
            height: cfg.flame_height,
            world_height: cfg.flame_scale,
            sprite,
            flip_x: false,
        });
    }

    // Wall impact billboard.
    if max_dist < cfg.flame_end {
        let impact_dist = (max_dist - cfg.wall_offset).max(cfg.flame_start);
        let sprite = attack_sprites.map_or_else(
            || Arc::new(make_blood_shot_sprite(8, 3)),
            |sprites| Arc::clone(sprites.flame_wall_hit_frame_loop(elapsed)),
        );
        billboards.push(Billboard {
            position: position + dir * impact_dist,
            height: cfg.flame_height,
            world_height: cfg.impact_scale,
            sprite,
            flip_x: false,
        });
    }
}

/// FPS counter — always visible, line 1.
#[cfg(not(target_family = "wasm"))]
#[derive(Component)]
struct FpsText;

/// Connection info — server + ping / connecting / failed. Line 2, toggled with Cmd+I.
#[cfg(not(target_family = "wasm"))]
#[derive(Component)]
struct ConnectionInfoText;

/// Full-screen dark overlay shown during non-Connected states.
#[cfg(not(target_family = "wasm"))]
#[derive(Component)]
struct ConnectionOverlay;

#[cfg(not(target_family = "wasm"))]
fn setup_client_info_text(mut commands: Commands) {
    // Full-screen dark overlay — covers the empty world during connecting/failed/disconnected.
    commands.spawn((
        ConnectionOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
        Visibility::Hidden,
    ));

    // FPS — top-right, toggled with Cmd+I.
    commands.spawn((
        FpsText,
        Text::new(String::new()),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(0.0, 1.0, 0.0, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(2.0),
            top: Val::Px(1.0),
            ..default()
        },
        Visibility::Hidden,
    ));

    // Connection info — top-left, toggled with Cmd+I.
    // Forced visible when connecting/failed/disconnected.
    commands.spawn((
        ConnectionInfoText,
        Text::new(String::new()),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(0.0, 1.0, 0.0, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(2.0),
            top: Val::Px(1.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// Toggle net info HUD with Cmd+I.
#[cfg(not(target_family = "wasm"))]
fn toggle_net_info(keys: Res<ButtonInput<KeyCode>>, mut visible: ResMut<NetInfoVisible>) {
    let modifier_held = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if modifier_held && keys.just_pressed(KeyCode::KeyI) {
        visible.0 = !visible.0;
    }
}

#[cfg(not(target_family = "wasm"))]
fn update_client_info_text(
    connection_state: Res<ConnectionState>,
    visible: Res<NetInfoVisible>,
    client: Option<Res<RenetClient>>,
    connect_addr: Option<Res<ConnectAddr>>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut fps_query: Query<
        (&mut Text, &mut Visibility),
        (
            With<FpsText>,
            Without<ConnectionInfoText>,
            Without<ConnectionOverlay>,
        ),
    >,
    mut conn_query: Query<
        (&mut Text, &mut TextColor, &mut Visibility),
        (
            With<ConnectionInfoText>,
            Without<FpsText>,
            Without<ConnectionOverlay>,
        ),
    >,
    mut overlay_query: Query<
        &mut Visibility,
        (
            With<ConnectionOverlay>,
            Without<FpsText>,
            Without<ConnectionInfoText>,
        ),
    >,
) {
    let user_wants = visible.0;
    // Connection overlay forced visible when not connected.
    let force_conn = !matches!(*connection_state, ConnectionState::Connected);

    // FPS — top-right, shown when toggled on.
    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .map_or_else(|| "--".to_string(), |v| format!("{v:.0}"));

    let fps_vis = if user_wants {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut text, mut vis) in &mut fps_query {
        text.0 = format!("FPS {fps}");
        *vis = fps_vis;
    }

    // Connection info — top-left, shown when toggled on OR when not connected.
    let server = connect_addr
        .as_ref()
        .map_or_else(|| "—".to_string(), |a| a.0.to_string());

    let (line, color) = match &*connection_state {
        ConnectionState::Connecting { .. } => (
            format!("{server} | connecting..."),
            Color::srgba(1.0, 1.0, 0.0, 0.7),
        ),
        ConnectionState::Connected => {
            let ping = client
                .as_ref()
                .map_or_else(|| "--".to_string(), |c| format!("{:.0}", c.rtt() * 1000.0));
            (
                format!("{server} | {ping}ms"),
                Color::srgba(0.0, 1.0, 0.0, 0.6),
            )
        }
        ConnectionState::Failed { reason } => (
            format!("{server} | {reason}"),
            Color::srgba(1.0, 0.4, 0.7, 0.9),
        ),
        ConnectionState::Disconnected { reason } => (
            format!("{server} | {reason}"),
            Color::srgba(1.0, 0.4, 0.7, 0.9),
        ),
    };

    let conn_vis = if user_wants || force_conn {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut text, mut tc, mut vis) in &mut conn_query {
        text.0 = line.clone();
        tc.0 = color;
        *vis = conn_vis;
    }

    // Dark overlay — shown during connecting/failed/disconnected, hidden when connected.
    let overlay_vis = if force_conn {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut overlay_query {
        *vis = overlay_vis;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use carcinisation_fps::camera::Camera;
    use carcinisation_fps::mosquiton::make_mosquiton_billboard_sprites;
    use carcinisation_fps::plugin::CameraRes;
    use carcinisation_net::{NetworkObjectId, PlayerNetState};

    fn init_sync_test_app(app: &mut App) {
        app.init_resource::<carcinisation_fps::plugin::ExtraBillboards>();
        app.init_resource::<EnemyAttackOverrides>();
        app.init_resource::<EnemyDamageFlickers>();
        app.init_resource::<HitImpacts>();
    }

    /// Dead enemies get death-pose billboards; alive enemies get alive billboards.
    #[test]
    fn replicated_net_enemies_populate_alive_and_dead_billboards() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalPlayerId(Some(PlayerId(1))));
        app.insert_resource(CameraRes(Camera::default()));
        app.insert_resource(Config::default());
        init_sync_test_app(&mut app);
        app.add_systems(Update, sync_camera_from_net_player);

        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(2.0, 3.0),
            angle: 0.25,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Alive,
            flame_active: false,
        });
        app.world_mut().spawn((
            NetEnemy {
                object_id: NetworkObjectId(1),
                position: Vec2::new(4.0, 5.0),
                angle: 0.0,
                state: NetEnemyState::Idle,
                enemy_type: NetEnemyType::Mosquiton,
            },
            NetHealth {
                current: 100.0,
                max: 100.0,
            },
        ));
        app.world_mut().spawn((
            NetEnemy {
                object_id: NetworkObjectId(2),
                position: Vec2::new(6.0, 7.0),
                angle: 0.0,
                state: NetEnemyState::Dead { burn: false },
                enemy_type: NetEnemyType::Mosquiton,
            },
            NetHealth {
                current: 0.0,
                max: 100.0,
            },
        ));

        app.update();

        let extra_bbs = app
            .world()
            .resource::<carcinisation_fps::plugin::ExtraBillboards>();
        // Both alive and dead enemies get billboards (dead = death pose).
        assert_eq!(extra_bbs.0.len(), 2);
        assert_eq!(extra_bbs.0[0].position, Vec2::new(4.0, 5.0));
        assert_eq!(extra_bbs.0[0].world_height, 0.9);

        let camera = app.world().resource::<CameraRes>();
        assert_eq!(camera.0.position, Vec2::new(2.0, 3.0));
        assert_eq!(camera.0.angle, 0.25);
    }

    #[test]
    fn replicated_mosquiton_billboard_uses_composed_sprites_when_loaded() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalPlayerId(Some(PlayerId(1))));
        app.insert_resource(CameraRes(Camera::default()));
        app.insert_resource(Config::default());
        app.insert_resource(MosquitonSprites(
            make_mosquiton_billboard_sprites().unwrap(),
        ));
        init_sync_test_app(&mut app);
        app.add_systems(Update, sync_camera_from_net_player);

        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(2.0, 3.0),
            angle: 0.25,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Alive,
            flame_active: false,
        });
        app.world_mut().spawn((
            NetEnemy {
                object_id: NetworkObjectId(1),
                position: Vec2::new(4.0, 5.0),
                angle: 0.0,
                state: NetEnemyState::Chase,
                enemy_type: NetEnemyType::Mosquiton,
            },
            NetHealth {
                current: 100.0,
                max: 100.0,
            },
        ));

        app.update();

        let elapsed_secs = app.world().resource::<Time>().elapsed_secs();
        let expected = app
            .world()
            .resource::<MosquitonSprites>()
            .0
            .alive_sprite_at(elapsed_secs)
            .data()
            .to_vec();
        let extra_bbs = app
            .world()
            .resource::<carcinisation_fps::plugin::ExtraBillboards>();
        assert_eq!(extra_bbs.0.len(), 1);
        assert_eq!(extra_bbs.0[0].sprite.data(), expected.as_slice());
        assert_ne!(
            extra_bbs.0[0].sprite.data(),
            make_mosquiton_placeholder_sprite(32, 2).data()
        );
    }

    #[test]
    fn attack_override_created_with_sprite_duration() {
        let sprites = make_mosquiton_billboard_sprites().unwrap();
        let shoot_dur = sprites.shoot_duration();
        let melee_dur = sprites.melee_duration();

        let mut overrides = EnemyAttackOverrides::default();
        // Simulate handle_enemy_attack_visual for ranged.
        overrides.0.insert(
            NetworkObjectId(1),
            AttackAnimOverride {
                kind: EnemyAttackKind::Ranged,
                elapsed: 0.0,
                duration: shoot_dur,
            },
        );
        let o = &overrides.0[&NetworkObjectId(1)];
        assert!(o.duration > 0.0, "shoot duration should be positive");
        assert!(
            (o.duration - shoot_dur).abs() < 0.001,
            "duration should match sprite data"
        );

        // Simulate handle_enemy_attack_visual for melee.
        overrides.0.insert(
            NetworkObjectId(2),
            AttackAnimOverride {
                kind: EnemyAttackKind::Melee,
                elapsed: 0.0,
                duration: melee_dur,
            },
        );
        let o = &overrides.0[&NetworkObjectId(2)];
        assert!(
            (o.duration - melee_dur).abs() < 0.001,
            "melee duration should match sprite data"
        );
    }

    #[test]
    fn attack_override_expires_after_duration() {
        let mut overrides = EnemyAttackOverrides::default();
        overrides.0.insert(
            NetworkObjectId(1),
            AttackAnimOverride {
                kind: EnemyAttackKind::Ranged,
                elapsed: 0.0,
                duration: 0.2,
            },
        );

        // Advance less than duration — still active.
        overrides.0.retain(|_, o| {
            o.elapsed += 0.1;
            o.elapsed < o.duration
        });
        assert_eq!(overrides.0.len(), 1, "should still be active at 0.1/0.2");

        // Advance past duration — expired.
        overrides.0.retain(|_, o| {
            o.elapsed += 0.15;
            o.elapsed < o.duration
        });
        assert_eq!(overrides.0.len(), 0, "should expire after 0.25 > 0.2");
    }

    #[test]
    fn attack_override_replaced_by_new_event() {
        let mut overrides = EnemyAttackOverrides::default();
        let id = NetworkObjectId(1);

        overrides.0.insert(
            id,
            AttackAnimOverride {
                kind: EnemyAttackKind::Ranged,
                elapsed: 0.15,
                duration: 0.2,
            },
        );

        // New event replaces with fresh elapsed.
        overrides.0.insert(
            id,
            AttackAnimOverride {
                kind: EnemyAttackKind::Melee,
                elapsed: 0.0,
                duration: 0.3,
            },
        );

        let o = &overrides.0[&id];
        assert_eq!(o.elapsed, 0.0, "new override should reset elapsed");
        assert!(
            matches!(o.kind, EnemyAttackKind::Melee),
            "new override should be melee"
        );
    }

    #[test]
    fn attack_override_fallback_duration_when_no_sprites() {
        // Without MosquitonSprites, handle_enemy_attack_visual uses fallback constants.
        let shoot_fallback = SHOOT_ANIM_DURATION_FALLBACK;
        let melee_fallback = MELEE_ANIM_DURATION_FALLBACK;
        assert!(shoot_fallback > 0.0);
        assert!(melee_fallback > 0.0);
    }

    /// Remote flame billboards are generated when `NetPlayer.flame_active` is true.
    #[test]
    fn remote_flame_active_generates_flame_billboards() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalPlayerId(Some(PlayerId(1))));
        app.insert_resource(CameraRes(Camera::default()));
        app.insert_resource(Config::default());
        init_sync_test_app(&mut app);
        app.add_systems(Update, sync_camera_from_net_player);

        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(2.0, 3.0),
            angle: 0.0,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Alive,
            flame_active: false,
        });

        // Remote player with flame_active = true (replicated authoritative state).
        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(2),
            position: Vec2::new(4.0, 5.0),
            angle: 0.0,
            current_attack: NetAttackId::Projectile,
            state: PlayerNetState::Alive,
            flame_active: true,
        });

        app.update();

        let extra_bbs = app
            .world()
            .resource::<carcinisation_fps::plugin::ExtraBillboards>();
        // Should have: 1 player billboard + flame arc billboards (12 segments + wall impact).
        assert!(
            extra_bbs.0.len() > 1,
            "flame_active=true should generate flame arc billboards, got {}",
            extra_bbs.0.len()
        );
    }

    /// Remote flame billboards are NOT generated when `flame_active` is false.
    #[test]
    fn remote_flame_inactive_no_flame_billboards() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalPlayerId(Some(PlayerId(1))));
        app.insert_resource(CameraRes(Camera::default()));
        app.insert_resource(Config::default());
        init_sync_test_app(&mut app);
        app.add_systems(Update, sync_camera_from_net_player);

        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(2.0, 3.0),
            angle: 0.0,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Alive,
            flame_active: false,
        });

        // Remote player with flamethrower equipped but NOT firing.
        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(2),
            position: Vec2::new(4.0, 5.0),
            angle: 0.0,
            current_attack: NetAttackId::Projectile,
            state: PlayerNetState::Alive,
            flame_active: false,
        });

        app.update();

        let extra_bbs = app
            .world()
            .resource::<carcinisation_fps::plugin::ExtraBillboards>();
        // Should have exactly 1 billboard (remote player sprite only, no flames).
        assert_eq!(
            extra_bbs.0.len(),
            1,
            "flame_active=false should not generate flame billboards"
        );
    }
}
