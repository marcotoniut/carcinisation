#![allow(
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::items_after_statements,
    clippy::redundant_closure_for_method_calls,
    clippy::match_same_arms,
    clippy::assigning_clones
)]

use std::sync::Arc;

use bevy::prelude::*;

#[cfg(not(target_family = "wasm"))]
use bevy_renet2::netcode::NetcodeClientTransport;
use bevy_replicon::prelude::*;
#[cfg(not(target_family = "wasm"))]
use bevy_replicon_renet2::RenetChannelsExt;
#[cfg(not(target_family = "wasm"))]
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient};
use carcinisation_fps::billboard::{
    Billboard, PickupBillboardSprites, make_blood_shot_sprite, make_damage_invert_sprite,
    make_enemy_sprite, make_health_pickup_sprite, make_mosquiton_placeholder_sprite,
};
use carcinisation_fps::directional_billboard::{
    BillboardAnimationState, DirectionalBillboardAtlas, make_player_billboard_atlas,
    resolve_billboard,
};
use carcinisation_fps::player_attack::{
    AttackInput, AttackLoadout, PlayerAttackSprites, PlayerAttackState, PlayerFlamethrower3pConfig,
};
use carcinisation_fps::plugin::CharDecals;
use carcinisation_fps::plugin::{
    Active, BloodShotSprites, CameraRes, CameraShakeState, Config, DeathViewState, MapRes,
    MosquitonSprites, SpiderShotSprites, SpideySprites, Systems, request_camera_shake,
};
use carcinisation_fps::raycast::{HitSide, WallSurfaceId};
use carcinisation_fps::render::CharDecal;
use carcinisation_fps::screen_particles::FpsScreenParticles;
use carcinisation_fps_core::ScreenParticleConfig;
use carcinisation_net::components::{NetEnemy, NetPickup};
use carcinisation_net::protocol::PickupEffect;
use carcinisation_net::{
    DamageEffect, DeathEffect, EnemyAttackKind, EnemyAttackVisual, FlameActive, FlameCharMark,
    HitConfirm, MuzzleFlash, NetAttackId, NetBurning, NetEnemyState, NetEnemyType, NetGroundFire,
    NetHealth, NetPickupKind, NetPlayer, NetProjectile, NetProjectileType, NetProtocolPlugin,
    NetworkObjectId, PlayerId, PlayerIdAssigned, register_net_all,
};
use std::net::SocketAddr;
#[cfg(not(target_family = "wasm"))]
use std::time::{SystemTime, UNIX_EPOCH};

pub mod input;
pub mod interpolation;
pub mod prediction;
pub use input::{ClientInputSequence, collect_and_send_intent};
use interpolation::{RemoteAngleInterpolation, RemotePositionInterpolation};
use prediction::{PendingInput, PredictedPlayerState};

/// Tracks which `PlayerId` belongs to this client.
#[derive(Resource, Debug, Default)]
pub struct LocalPlayerId(pub Option<PlayerId>);

/// Client connection state machine.
///
/// Transitions:
///   Connecting ──(`PlayerIdAssigned`)──→ Connected
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

/// Client-side pickup effect billboards (feedback at pickup location).
#[derive(Debug, Clone)]
struct PickupImpact {
    position: Vec2,
    age: f32,
    lifetime: f32,
}

/// Active pickup feedback impacts.
#[derive(Resource, Debug, Default)]
struct PickupImpacts(Vec<PickupImpact>);

/// Local-only health pickup screen feedback deduplication state.
#[derive(Resource, Debug, Default)]
struct HealthPickupScreenFeedback {
    last_local_health: Option<f32>,
    last_burst_at_secs: Option<f64>,
}

const PICKUP_BILLBOARD_WORLD_HEIGHT: f32 = 0.25;
const CONNECTION_TIMEOUT_SECS: f64 = 15.0;
const HEALTH_PICKUP_SCREEN_FEEDBACK_SUPPRESSION_SECS: f64 = 0.20;

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
    projectile_type: Option<NetProjectileType>,
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
            .init_resource::<PickupImpacts>()
            .init_resource::<FpsScreenParticles>()
            .init_resource::<HealthPickupScreenFeedback>()
            .insert_resource(prediction::PredictionEnabled::from_env())
            .init_resource::<PredictedPlayerState>()
            .init_resource::<prediction::PredictionDiagnostics>()
            .init_resource::<PendingInput>()
            .init_resource::<prediction::PredictedRenderState>()
            .init_resource::<prediction::StaleInput>()
            .init_resource::<carcinisation_net::prediction::PredictionHistory>()
            .register_type::<PredictedPlayerState>()
            .register_type::<prediction::PredictionDiagnostics>()
            .register_type::<prediction::PredictedRenderState>()
            .register_type::<prediction::PredictionEnabled>();

        // Simulated latency for testing prediction drift.
        if let Some(latency) = input::SimulatedLatency::from_env() {
            app.insert_resource(latency)
                .add_systems(Update, input::flush_delayed_intents);
        }

        app.insert_resource(carcinisation_fps_core::FpsVisualConfig::load())
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
            .add_observer(handle_pickup_effect)
            .add_observer(prediction::handle_input_ack)
            .add_systems(Update, collect_and_send_intent.run_if(is_connected))
            .add_systems(
                Update,
                prediction::tick_predicted_render
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            .add_systems(Update, prediction::reset_prediction_diagnostics)
            .add_systems(
                Update,
                (
                    attach_interpolation_to_players,
                    attach_interpolation_to_enemies,
                )
                    .run_if(is_connected),
            )
            .add_systems(
                Update,
                (
                    tick_attack_overrides,
                    tick_damage_flickers,
                    tick_hit_impacts,
                    tick_player_interpolation,
                    tick_enemy_interpolation,
                    sync_player_lifecycle_state,
                )
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            .add_systems(
                Update,
                sync_local_player_health_from_net_health
                    .before(Systems)
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            .add_systems(
                Update,
                sync_camera_from_net_player
                    .after(tick_player_interpolation)
                    .after(tick_enemy_interpolation)
                    .after(prediction::tick_predicted_render)
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            .add_systems(
                Update,
                tick_pickup_impacts
                    .after(sync_camera_from_net_player)
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            .add_systems(
                Update,
                queue_pickup_billboards
                    .after(tick_pickup_impacts)
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            .add_systems(
                Update,
                sync_weapon_hud_and_flame_visual
                    .before(Systems)
                    .run_if(resource_exists::<Active>)
                    .run_if(is_connected),
            )
            // Prediction init systems (Update, conditional).
            .add_systems(
                Update,
                (
                    prediction::init_client_map.run_if(resource_exists::<MapRes>.and(not(
                        resource_exists::<carcinisation_net::prediction::ClientMap>,
                    ))),
                    prediction::sync_client_map_from_map_res
                        .run_if(resource_exists::<MapRes>)
                        .run_if(resource_exists::<carcinisation_net::prediction::ClientMap>),
                    prediction::init_predicted_state.run_if(resource_exists::<Active>),
                    prediction::clear_prediction_on_death.run_if(resource_exists::<Active>),
                ),
            )
            // Prediction movement (FixedUpdate, 30 Hz) — in MovementSet to
            // match the server's scheduling and ordering constraints.
            .add_systems(
                FixedUpdate,
                prediction::apply_predicted_movement.in_set(carcinisation_net::MovementSet),
            );

        info!("FpsClientPlugin: connect addr = {:?}", self.connect_addr);

        #[cfg(not(target_family = "wasm"))]
        {
            let _ = dotenvy::dotenv_override();
            app.insert_resource(ConnectAddr(self.connect_addr));
            app.init_resource::<NetInfoVisible>();
        }
        app.add_systems(Startup, init_pickup_sprites);

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
        projectile_type: confirm.projectile_type,
    });
}

/// Pickup effect — pushes world feedback and local screen feedback.
fn handle_pickup_effect(
    trigger: On<PickupEffect>,
    mut impacts: ResMut<PickupImpacts>,
    local_id: Res<LocalPlayerId>,
    config: Res<Config>,
    time: Res<Time>,
    mut screen_particles: ResMut<FpsScreenParticles>,
    mut screen_feedback: ResMut<HealthPickupScreenFeedback>,
    particle_config: Res<ScreenParticleConfig>,
) {
    let event = trigger.event();
    impacts.0.push(PickupImpact {
        position: event.position,
        age: 0.0,
        lifetime: 0.5,
    });

    if event.kind == NetPickupKind::Health && local_id.0 == Some(event.player_id) {
        try_spawn_health_pickup_screen_feedback(
            &mut screen_particles,
            &mut screen_feedback,
            config.screen_width,
            config.screen_height,
            &particle_config,
            time.elapsed_secs_f64(),
        );
    }
}

fn try_spawn_health_pickup_screen_feedback(
    screen_particles: &mut FpsScreenParticles,
    screen_feedback: &mut HealthPickupScreenFeedback,
    screen_width: u32,
    screen_height: u32,
    particle_config: &ScreenParticleConfig,
    now_secs: f64,
) {
    if screen_feedback
        .last_burst_at_secs
        .is_some_and(|last| now_secs - last < HEALTH_PICKUP_SCREEN_FEEDBACK_SUPPRESSION_SECS)
    {
        return;
    }

    screen_particles.spawn_health_pickup_burst(screen_width, screen_height, particle_config);
    screen_feedback.last_burst_at_secs = Some(now_secs);
}

fn handle_damage_effect(
    trigger: On<DamageEffect>,
    local_id: Res<LocalPlayerId>,
    mut camera_shake: ResMut<CameraShakeState>,
    config: Res<Config>,
    visual_config: Res<carcinisation_fps_core::FpsVisualConfig>,
    mut flickers: ResMut<EnemyDamageFlickers>,
    mut health: ResMut<carcinisation_fps::plugin::PlayerHealth>,
) {
    let effect = trigger.event();
    debug!(
        "DamageEffect: target={:?} damage={:.0} remaining={:.0}",
        effect.target_id, effect.damage, effect.remaining_health
    );

    // Trigger screen shake and sync health for the local player.
    let is_local = effect.was_player && local_id.0.is_some_and(|pid| effect.target_id.0 == pid.0);
    if is_local {
        request_camera_shake(&mut camera_shake, &config);
        health.0 = effect.remaining_health.ceil() as u32;
    }

    // Trigger damage flicker for enemies (not players).
    // Only start a new flicker if none is active — avoids resetting mid-cycle
    // when burn damage sends frequent DamageEffect events.
    if !effect.was_player {
        let vc = *visual_config;
        flickers
            .0
            .entry(effect.target_id)
            .or_insert_with(|| carcinisation_fps_core::enemy::DamageFlicker::from_config(&vc));
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
    mosquiton_sprites: Option<Res<MosquitonSprites>>,
    spidey_sprites: Option<Res<SpideySprites>>,
    enemies: Query<&NetEnemy>,
) {
    let event = trigger.event();
    // Determine animation duration from the correct enemy's sprites.
    let is_spidey = enemies
        .iter()
        .any(|e| e.object_id == event.object_id && e.enemy_type == NetEnemyType::Spidey);
    let duration = if is_spidey {
        match event.kind {
            EnemyAttackKind::Ranged => spidey_sprites
                .as_ref()
                .map_or(SHOOT_ANIM_DURATION_FALLBACK, |s| {
                    s.0.shoot.iter().map(|f| f.duration).sum::<f32>().max(0.1)
                }),
            EnemyAttackKind::Melee => spidey_sprites
                .as_ref()
                .map_or(MELEE_ANIM_DURATION_FALLBACK, |s| {
                    s.0.lunge_total_duration().max(0.1)
                }),
        }
    } else {
        match event.kind {
            EnemyAttackKind::Ranged => mosquiton_sprites
                .as_ref()
                .map_or(SHOOT_ANIM_DURATION_FALLBACK, |s| s.0.shoot_duration()),
            EnemyAttackKind::Melee => mosquiton_sprites
                .as_ref()
                .map_or(MELEE_ANIM_DURATION_FALLBACK, |s| s.0.melee_duration()),
        }
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

/// Tick pickup effect lifetimes and push active billboards into extra_bbs.
fn tick_pickup_impacts(
    mut impacts: ResMut<PickupImpacts>,
    time: Res<Time>,
    mut extra_bbs: ResMut<carcinisation_fps::plugin::ExtraBillboards>,
    pickup_sprites: Option<Res<PickupBillboardSprites>>,
) {
    let dt = time.delta_secs();
    let world_height = PICKUP_BILLBOARD_WORLD_HEIGHT;
    let height = carcinisation_fps::spidey::FLOOR_OFFSET + world_height / 2.0;
    let sprite = pickup_sprites.as_ref().map_or_else(
        || Arc::new(make_health_pickup_sprite(12)),
        |s| Arc::clone(&s.health),
    );
    for impact in &mut impacts.0 {
        impact.age += dt;
        extra_bbs.0.push(Billboard {
            position: impact.position,
            height,
            world_height,
            sprite: Arc::clone(&sprite),
            flip_x: false,
            palette_variant: None,
        });
    }
    impacts.0.retain(|i| i.age < i.lifetime);
}

/// Populate persistent pickup billboards from replicated `NetPickup` entities.
/// Only renders pickups that are `.available == true`.
fn queue_pickup_billboards(
    net_pickups: Query<&NetPickup>,
    mut extra_bbs: ResMut<carcinisation_fps::plugin::ExtraBillboards>,
    pickup_sprites: Option<Res<PickupBillboardSprites>>,
) {
    let Some(sprites) = pickup_sprites else {
        return;
    };
    let world_height = PICKUP_BILLBOARD_WORLD_HEIGHT;
    let height = carcinisation_fps::spidey::FLOOR_OFFSET + world_height / 2.0;
    for pickup in &net_pickups {
        if !pickup.available {
            continue;
        }
        extra_bbs.0.push(Billboard {
            position: pickup.position,
            height,
            world_height,
            sprite: Arc::clone(sprites.sprite_for_kind(pickup.kind)),
            flip_x: false,
            palette_variant: None,
        });
    }
}

fn sync_local_player_health_from_net_health(
    net_players: Query<(&NetPlayer, &NetHealth), Changed<NetHealth>>,
    local_player_id: Res<LocalPlayerId>,
    mut player_health: ResMut<carcinisation_fps::plugin::PlayerHealth>,
    time: Res<Time>,
    config: Option<Res<Config>>,
    screen_particles: Option<ResMut<FpsScreenParticles>>,
    screen_feedback: Option<ResMut<HealthPickupScreenFeedback>>,
    particle_config: Option<Res<ScreenParticleConfig>>,
) {
    let Some(my_id) = local_player_id.0 else {
        return;
    };
    let Some((_player, health)) = net_players
        .iter()
        .find(|(player, _)| player.player_id == my_id)
    else {
        return;
    };

    let current = health.current.ceil().clamp(0.0, health.max) as u32;
    if let Some(mut screen_feedback) = screen_feedback {
        let previous = screen_feedback.last_local_health.replace(health.current);
        if let (Some(previous), Some(config), Some(mut screen_particles), Some(particle_config)) =
            (previous, config, screen_particles, particle_config)
            && previous > 0.0
            && health.current > previous + f32::EPSILON
        {
            try_spawn_health_pickup_screen_feedback(
                &mut screen_particles,
                &mut screen_feedback,
                config.screen_width,
                config.screen_height,
                &particle_config,
                time.elapsed_secs_f64(),
            );
        }
    }

    player_health.0 = current;
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

/// Attach interpolation components to newly replicated remote players.
///
/// Skips the local player — interpolation is only for remote entities.
fn attach_interpolation_to_players(
    mut commands: Commands,
    new_players: Query<(Entity, &NetPlayer), Added<NetPlayer>>,
    local_id: Res<LocalPlayerId>,
) {
    for (entity, np) in &new_players {
        if local_id.0 == Some(np.player_id) {
            continue;
        }
        commands.entity(entity).insert((
            RemotePositionInterpolation::new(np.position),
            RemoteAngleInterpolation::new(np.angle),
        ));
    }
}

/// Attach interpolation components to newly replicated enemies.
fn attach_interpolation_to_enemies(
    mut commands: Commands,
    new_enemies: Query<(Entity, &NetEnemy), Added<NetEnemy>>,
) {
    for (entity, ne) in &new_enemies {
        commands.entity(entity).insert((
            RemotePositionInterpolation::new(ne.position),
            RemoteAngleInterpolation::new(ne.angle),
        ));
    }
}

/// Detect replication changes on remote players and advance interpolation.
fn tick_player_interpolation(
    mut query: Query<(
        &NetPlayer,
        &mut RemotePositionInterpolation,
        &mut RemoteAngleInterpolation,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (np, mut pos_interp, mut angle_interp) in &mut query {
        pos_interp.update_if_changed(np.position);
        angle_interp.update_if_changed(np.angle);
        pos_interp.tick(dt);
        angle_interp.tick(dt);
    }
}

/// Detect replication changes on enemies and advance interpolation.
fn tick_enemy_interpolation(
    mut query: Query<(
        &NetEnemy,
        &mut RemotePositionInterpolation,
        &mut RemoteAngleInterpolation,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (ne, mut pos_interp, mut angle_interp) in &mut query {
        pos_interp.update_if_changed(ne.position);
        angle_interp.update_if_changed(ne.angle);
        pos_interp.tick(dt);
        angle_interp.tick(dt);
    }
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
    mut prev_angle: Local<f32>,
    time: Res<Time>,
    action: Res<leafwing_input_manager::prelude::ActionState<carcinisation_input::GBInput>>,
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

    // Derive aim_turn_velocity from replicated angle delta so the
    // flamethrower chain bends (whip effect) during turns.
    attack_input.aim_turn_velocity = carcinisation_fps_core::angular_velocity_clamped(
        player.angle,
        *prev_angle,
        time.delta_secs(),
    );
    *prev_angle = player.angle;

    // Drive weapon walk-bob from local movement input.
    attack_input.moving_forward_back = action.pressed(&carcinisation_input::GBInput::Up)
        || action.pressed(&carcinisation_input::GBInput::Down);

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

/// Initialise pickup billboard sprites from the embedded composed atlas.
/// Logs a warning if loading fails (non-fatal — systems fall back gracefully).
fn init_pickup_sprites(mut commands: Commands) {
    match carcinisation_fps::billboard::make_pickup_billboard_sprites() {
        Ok(sprites) => {
            commands.insert_resource(sprites);
            info!("[FPS] Pickup billboard sprites loaded");
        }
        Err(err) => {
            warn!("[FPS] Failed to load pickup billboard sprites: {err}");
        }
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
    use bevy_renet2::netcode::{ClientAuthentication, NativeSocket};
    // Reset start_time — build() captured Instant::now() during plugin ctor,
    // but app init (asset loading, shader compilation) may have taken seconds.
    if let ConnectionState::Connecting { start_time, .. } = &mut *connection_state {
        *start_time = std::time::Instant::now();
    }

    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos() as u64;

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
    /// Smoothed speed + previous angle for walk detection and flame bend.
    /// `(prev_position, smoothed_speed, prev_angle)`.
    player_smoothed_speed: std::collections::HashMap<PlayerId, (Vec2, f32, f32)>,
    /// Fallback placeholder sprites if atlas loading fails.
    fallback_sprites: Option<[Arc<carapace::image::CxImage>; 4]>,
    /// Remote flame billboard config (loaded once from RON).
    remote_flame_config: Option<PlayerFlamethrower3pConfig>,
    /// Cached flamethrower visual params (loaded once from RON).
    /// `(bend_strength, bend_power, world_speed, spawn_interval, max_segments, world_range, bend_return_speed)`.
    flame_visual_params: Option<FlameVisualParams>,
    /// Per-remote-player flame chain visual state.
    remote_flame_states: std::collections::HashMap<PlayerId, RemoteFlameVisual>,
    /// Tracks when each ground fire entity first appeared (`elapsed_secs` at spawn).
    ground_fire_spawn_times: std::collections::HashMap<bevy::prelude::Entity, f32>,
    /// Cached ground fire visual config (loaded once from RON to avoid per-frame filesystem access).
    ground_fire_visual_config: Option<carcinisation_fps::player_attack::GroundFireVisualConfig>,
    /// Cached ground fire fade start time from `FpsCombatConfig` (avoids per-frame `Default::default()`).
    ground_fire_fade_start_secs: Option<f32>,
    /// Cached projectile speed from `FpsCombatConfig` (avoids per-frame `Default::default()`).
    projectile_visual_speed: Option<f32>,
}

/// Cached flamethrower visual parameters derived from `PlayerFlamethrower1pConfig`.
#[derive(Clone, Copy, Debug)]
struct FlameVisualParams {
    speed: f32,
    emit_interval: f32,
}

/// Lightweight per-remote-player flame stream simulation (visual only).
#[derive(Clone, Debug, Default)]
struct RemoteFlameVisual {
    samples: Vec<RemoteFlameStreamSample>,
    spawn_cooldown: f32,
    sample_counter: u32,
    spawning: bool,
}

#[derive(Clone, Debug)]
struct RemoteFlameStreamSample {
    emit_position: Vec2,
    emit_direction: Vec2,
    age: f32,
    seed: u32,
}

impl RemoteFlameStreamSample {
    fn world_position(&self, speed: f32) -> Vec2 {
        self.emit_position + self.emit_direction * speed * self.age
    }
}

fn screen_right_from_direction(dir: Vec2) -> Vec2 {
    Vec2::new(dir.y, -dir.x)
}

impl RemoteFlameVisual {
    fn tick(
        &mut self,
        dt: f32,
        active: bool,
        params: &FlameVisualParams,
        world_range: f32,
        player_position: Vec2,
        player_angle: f32,
        nozzle_forward: f32,
        nozzle_lateral: f32,
    ) {
        let max_age = world_range / params.speed;

        if !active && self.samples.is_empty() {
            self.spawn_cooldown = 0.0;
            self.sample_counter = 0;
            self.spawning = false;
            return;
        }

        if !active && self.spawning {
            self.spawning = false;
        }
        if active && !self.spawning && self.samples.is_empty() {
            self.spawning = true;
            self.sample_counter = 0;
        }

        // Age existing samples and expire old ones.
        for sample in &mut self.samples {
            sample.age += dt;
        }
        self.samples.retain(|s| s.age < max_age);

        // Emit new samples while firing.
        if self.spawning {
            self.spawn_cooldown -= dt;
            let dir = Vec2::new(player_angle.cos(), player_angle.sin());
            let right = screen_right_from_direction(dir);
            let nozzle_pos = player_position + dir * nozzle_forward + right * nozzle_lateral;
            while self.spawn_cooldown <= 0.0 {
                let seed = self.sample_counter.wrapping_mul(0x9E37_79B9) ^ 0xC2B2_AE35;
                self.samples.push(RemoteFlameStreamSample {
                    emit_position: nozzle_pos,
                    emit_direction: dir,
                    age: 0.0,
                    seed,
                });
                self.sample_counter = self.sample_counter.wrapping_add(1);
                self.spawn_cooldown += params.emit_interval;
            }
        }
    }
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

/// Bundled optional sprite resources for `sync_camera_from_net_player`.
#[derive(bevy::ecs::system::SystemParam)]
struct SyncSpriteResources<'w> {
    mosquiton: Option<Res<'w, MosquitonSprites>>,
    spidey: Option<Res<'w, SpideySprites>>,
    spider_shot: Option<Res<'w, SpiderShotSprites>>,
    blood_shot: Option<Res<'w, BloodShotSprites>>,
    attack: Option<Res<'w, PlayerAttackSprites>>,
    burn_config: Res<'w, carcinisation_fps_core::BurnConfig>,
}

/// Bundled prediction visual state for `sync_camera_from_net_player`.
#[derive(bevy::ecs::system::SystemParam)]
struct PredictionVisualParams<'w> {
    predicted_state: Option<Res<'w, PredictedPlayerState>>,
    render_state: Option<Res<'w, prediction::PredictedRenderState>>,
    diag: Option<ResMut<'w, prediction::PredictionDiagnostics>>,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn sync_camera_from_net_player(
    net_players: Query<(
        Entity,
        &NetPlayer,
        Option<&RemotePositionInterpolation>,
        Option<&RemoteAngleInterpolation>,
    )>,
    net_enemies: Query<(
        &NetEnemy,
        Option<&NetHealth>,
        Option<&NetBurning>,
        Option<&RemotePositionInterpolation>,
        Option<&RemoteAngleInterpolation>,
    )>,
    net_projectiles: Query<&NetProjectile>,
    net_ground_fires: Query<(Entity, &NetGroundFire)>,
    mut camera_res: ResMut<CameraRes>,
    mut extra_bbs: ResMut<carcinisation_fps::plugin::ExtraBillboards>,
    local_player_id: Res<LocalPlayerId>,
    map_res: Option<Res<MapRes>>,
    sprites: SyncSpriteResources,
    attack_overrides: Res<EnemyAttackOverrides>,
    damage_flickers: Res<EnemyDamageFlickers>,
    hit_impacts: Res<HitImpacts>,
    mut sync_locals: Local<SyncLocals>,
    time: Res<Time>,
    flame_cfg: Res<carcinisation_fps_core::PlayerFlamethrowerConfig>,
    mut prediction_visual: PredictionVisualParams,
) {
    let mosquiton_sprites = &sprites.mosquiton;
    let spidey_sprites = &sprites.spidey;
    let spider_shot_sprites = &sprites.spider_shot;
    let blood_shot_sprites = &sprites.blood_shot;
    let attack_sprites = &sprites.attack;
    let burn_config = &sprites.burn_config;

    let Some(my_id) = local_player_id.0 else {
        return;
    };

    let local_player = net_players.iter().find(|(_, p, _, _)| p.player_id == my_id);
    let Some((local_entity, local_np, _, _)) = local_player else {
        return;
    };

    // Use render-interpolated predicted state for the local camera.
    // PredictedPlayerState updates at 30Hz (FixedUpdate), but the camera
    // renders at 60Hz+. PredictedRenderState lerps between the last two
    // predicted snapshots based on elapsed time, eliminating visual stepping.
    if let Some(ref predicted) = prediction_visual.predicted_state {
        if predicted.initialised {
            if let Some(ref render) = prediction_visual.render_state {
                camera_res.0.position = render.interpolated_position();
                camera_res.0.angle = render.interpolated_angle();
            } else {
                camera_res.0.position = predicted.position;
                camera_res.0.angle = predicted.angle;
            }
        } else {
            camera_res.0.position = local_np.position;
            camera_res.0.angle = local_np.angle;
        }
    } else {
        camera_res.0.position = local_np.position;
        camera_res.0.angle = local_np.angle;
    }

    // Record diagnostics.
    if let Some(ref mut diag) = prediction_visual.diag {
        diag.camera_position = camera_res.0.position;
        diag.camera_angle = camera_res.0.angle;
        diag.replicated_position = local_np.position;
        diag.replicated_angle = local_np.angle;
        diag.prediction_active = prediction_visual
            .predicted_state
            .as_ref()
            .is_some_and(|p| p.initialised);
        diag.render_alpha = prediction_visual.render_state.as_ref().map_or(0.0, |r| {
            if r.interval > 0.0 {
                (r.elapsed / r.interval).min(1.0)
            } else {
                1.0
            }
        });
        diag.camera_writes_this_frame += 1;
    }

    extra_bbs.0.clear();
    let elapsed = time.elapsed_secs();
    let dt = time.delta_secs();

    // Lazy-init remote flame config from RON.
    if sync_locals.remote_flame_config.is_none() {
        sync_locals.remote_flame_config = Some(carcinisation_core::ron_config!(
            "assets/config/attacks/player_flamethrower_3p.ron"
        ));
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

    // Lazy-init flame visual params from flamethrower config.
    if sync_locals.flame_visual_params.is_none() {
        sync_locals.flame_visual_params = Some(FlameVisualParams {
            speed: flame_cfg.speed,
            emit_interval: flame_cfg.emit_interval_ms as f32 / 1000.0,
        });
    }
    let flame_params = sync_locals.flame_visual_params.unwrap();

    struct RemotePlayerInfo {
        player_id: PlayerId,
        position: Vec2,
        angle: f32,
        visual_pos: Vec2,
        visual_angle: f32,
        state: carcinisation_net::PlayerNetState,
        current_attack: NetAttackId,
        flame_active: bool,
        avatar_palette_variant: Option<carcinisation_net::AvatarPaletteVariant>,
    }
    // Collect remote player data first to avoid borrow conflicts.
    // Interpolation components (ticked earlier this frame) provide smooth
    // visual position/angle; fall back to raw replicated values.
    let remote_players: Vec<RemotePlayerInfo> = net_players
        .iter()
        .filter(|(_, p, _, _)| p.player_id != my_id)
        .map(|(_, np, pos_interp, angle_interp)| {
            let visual_pos = pos_interp.map_or(np.position, |i| i.interpolated());
            let visual_angle = angle_interp.map_or(np.angle, |i| i.interpolated());
            RemotePlayerInfo {
                player_id: np.player_id,
                position: np.position,
                angle: np.angle,
                visual_pos,
                visual_angle,
                state: np.state.clone(),
                current_attack: np.current_attack,
                flame_active: np.flame_active,
                avatar_palette_variant: np.avatar_palette_variant,
            }
        })
        .collect();

    let mut seen_player_ids: Vec<PlayerId> = Vec::new();

    for rp in &remote_players {
        seen_player_ids.push(rp.player_id);

        let visual_pos = rp.visual_pos;
        let visual_angle = rp.visual_angle;

        // Determine animation action from player state and movement.
        // Use exponentially smoothed speed to avoid walk/idle flicker from
        // network tick-rate jitter.
        let currently_walking =
            anim_state_is_walking(sync_locals.player_anim_states.get(&rp.player_id));
        let action = match rp.state {
            carcinisation_net::PlayerNetState::Dead => "death",
            carcinisation_net::PlayerNetState::Alive => {
                let (prev_pos, smoothed_speed, _prev_angle) = sync_locals
                    .player_smoothed_speed
                    .entry(rp.player_id)
                    .or_insert((rp.position, 0.0, rp.angle));
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
        // Use interpolated visual_pos/visual_angle for smooth rendering.
        let mut pushed = false;
        if let Some(atlas) = &sync_locals.player_billboard_atlas
            && let Some(resolved) = resolve_billboard(
                atlas,
                local_np.position,
                visual_pos,
                visual_angle,
                &anim_snapshot,
            )
        {
            // height shifts the billboard vertically. Negative = feet toward floor.
            // Formula: -(0.5 - world_height/2) grounds feet at floor level.
            let world_height = 0.65;
            extra_bbs.0.push(Billboard {
                position: visual_pos,
                height: -(0.5 - world_height / 2.0),
                world_height,
                sprite: resolved.sprite,
                flip_x: resolved.flip_x,
                palette_variant: rp.avatar_palette_variant,
            });
            pushed = true;
        }

        if !pushed {
            // Fallback: use placeholder diamond sprites.
            if let Some(fallback) = &sync_locals.fallback_sprites {
                let color_idx = (rp.player_id.0.wrapping_sub(1) % 4) as usize;
                extra_bbs.0.push(Billboard {
                    position: visual_pos,
                    height: 0.0,
                    world_height: 1.5,
                    sprite: Arc::clone(&fallback[color_idx]),
                    flip_x: false,
                    palette_variant: rp.avatar_palette_variant,
                });
            }
        }

        // Remote flame visual simulation — persistent world-space stream.
        // Flame uses interpolated position/angle for smooth nozzle tracking.
        {
            let flame_active = rp.flame_active
                && matches!(
                    rp.current_attack,
                    carcinisation_net::NetAttackId::Projectile
                );

            let flame_3p_cfg = sync_locals.remote_flame_config.clone().unwrap_or_default();
            let world_range = flame_cfg.range;

            let vis = sync_locals
                .remote_flame_states
                .entry(rp.player_id)
                .or_default();
            vis.tick(
                dt,
                flame_active,
                &flame_params,
                world_range,
                visual_pos,
                visual_angle,
                flame_3p_cfg.nozzle_forward,
                flame_3p_cfg.nozzle_lateral,
            );

            if !vis.samples.is_empty() {
                push_remote_flame_billboards(
                    &mut extra_bbs.0,
                    elapsed,
                    attack_sprites.as_deref(),
                    map_res.as_deref(),
                    &flame_3p_cfg,
                    &flame_params,
                    &vis.samples,
                    visual_pos,
                    visual_angle,
                    &flame_cfg,
                );
            }
        }

        // Update prev_angle for next frame's turn velocity derivation.
        if let Some((_, _, prev_angle)) = sync_locals.player_smoothed_speed.get_mut(&rp.player_id) {
            *prev_angle = rp.angle;
        }
    }

    // Prune stale animation states for disconnected players.
    sync_locals
        .player_anim_states
        .retain(|id, _| seen_player_ids.contains(id));
    sync_locals
        .player_smoothed_speed
        .retain(|id, _| seen_player_ids.contains(id));
    sync_locals
        .remote_flame_states
        .retain(|id, _| seen_player_ids.contains(id));

    // Enemy billboards (including dying/dead for death pose, excludes despawned).
    for (enemy, _health, net_burning, enemy_pos_interp, _enemy_angle_interp) in net_enemies.iter() {
        let visual_pos = enemy_pos_interp.map_or(enemy.position, |i| i.interpolated());
        let show_invert = damage_flickers
            .0
            .get(&enemy.object_id)
            .is_some_and(|f| f.showing_invert());
        extra_bbs.0.push(net_enemy_billboard(
            enemy,
            visual_pos,
            elapsed,
            mosquiton_sprites.as_deref(),
            spidey_sprites.as_deref(),
            attack_overrides.0.get(&enemy.object_id),
            show_invert,
        ));

        // Burn flame effect for enemies killed by fire (persists through Dead until despawn).
        // Resolve the corpse sprite for the correct enemy type.
        let burn_corpse_sprite: Option<&carapace::image::CxImage> = match enemy.enemy_type {
            NetEnemyType::Spidey => spidey_sprites
                .as_deref()
                .map(|s| s.0.alive_sprite_at(0.0).as_ref()),
            _ => mosquiton_sprites
                .as_deref()
                .map(|s| s.0.alive_sprite_at(0.0).as_ref()),
        };
        if matches!(
            enemy.state,
            NetEnemyState::Dying { burn: true } | NetEnemyState::Dead { burn: true }
        ) && let Some(corpse_sprite) = burn_corpse_sprite
        {
            push_net_burn_flames(
                &mut extra_bbs.0,
                visual_pos,
                enemy.object_id.0,
                elapsed,
                camera_res.0.position,
                camera_res.0.direction(),
                corpse_sprite,
                attack_sprites.as_deref(),
            );
        }

        // Living enemy burn flames — driven by NetBurning intensity.
        let burn_intensity = net_burning.map_or(0.0, |b| b.intensity);
        if burn_intensity > 0.0
            && !matches!(
                enemy.state,
                NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
            )
            && let Some(corpse_sprite) = burn_corpse_sprite
        {
            push_alive_burn_flames(
                &mut extra_bbs.0,
                visual_pos,
                enemy.object_id.0,
                burn_intensity,
                elapsed,
                camera_res.0.position,
                camera_res.0.direction(),
                corpse_sprite,
                attack_sprites.as_deref(),
                burn_config,
            );
        }
    }

    // Ground fire billboards (replicated from server).
    {
        let cam_dir = camera_res.0.direction();
        let right = Vec2::new(-cam_dir.y, cam_dir.x);
        // Cached on first access to avoid per-frame filesystem/allocation cost.
        // Clone values out of sync_locals to release the mutable borrow before
        // accessing ground_fire_spawn_times below.
        let vis = sync_locals
            .ground_fire_visual_config
            .get_or_insert_with(carcinisation_fps::player_attack::GroundFireVisualConfig::load)
            .clone();
        let fade_start = *sync_locals
            .ground_fire_fade_start_secs
            .get_or_insert_with(|| {
                carcinisation_fps_core::FpsCombatConfig::default().ground_fire_fade_start_secs
            });

        // Track spawn times and prune despawned fires.
        let live_entities: std::collections::HashSet<Entity> =
            net_ground_fires.iter().map(|(e, _)| e).collect();
        sync_locals
            .ground_fire_spawn_times
            .retain(|e, _| live_entities.contains(e));

        for (entity, fire) in net_ground_fires.iter() {
            let spawn_time = *sync_locals
                .ground_fire_spawn_times
                .entry(entity)
                .or_insert(elapsed);
            let fire_elapsed = elapsed - spawn_time;
            let intensity: f32 = if fire_elapsed >= fade_start { 0.5 } else { 1.0 };

            let flames = carcinisation_fps_core::ground_fire::ground_fire_flame_layout(
                fire.seed,
                vis.flame_count,
                vis.visual_radius,
            );
            if let Some(sprites) = attack_sprites.as_deref() {
                for (offset, scale, phase) in &flames {
                    let full_sprite = sprites.flame_frame_loop(elapsed + phase);
                    let cropped =
                        carcinisation_fps::plugin::crop_bottom(full_sprite, vis.crop_bottom_px);
                    let world_height = vis.flame_world_height * scale * intensity;
                    let height = -0.5 + world_height * 0.5;
                    extra_bbs.0.push(Billboard {
                        position: fire.position + right * offset.x + cam_dir * offset.y,
                        height,
                        world_height,
                        sprite: std::sync::Arc::new(cropped),
                        flip_x: false,
                        palette_variant: None,
                    });
                }
            }
        }
    }

    // Projectile billboards (extrapolated forward by one frame for smoothness).
    // Clamp dt to 50ms so low-FPS spikes or browser stalls don't cause large visual jumps.
    let frame_dt = time.delta_secs().min(0.05);
    // Cached on first access to avoid per-frame `Default::default()` allocation.
    let projectile_speed = *sync_locals
        .projectile_visual_speed
        .get_or_insert_with(|| carcinisation_fps_core::FpsCombatConfig::default().projectile_speed);
    for proj in net_projectiles.iter() {
        let dir = Vec2::new(proj.angle.cos(), proj.angle.sin());
        let extrapolated = proj.position + dir * projectile_speed * frame_dt;
        let sprite = match proj.projectile_type {
            NetProjectileType::BloodShot => blood_shot_sprites.as_ref().map_or_else(
                || Arc::new(make_blood_shot_sprite(8, 3)),
                |bs| Arc::clone(&bs.0.hover),
            ),
            NetProjectileType::WebShot => spider_shot_sprites.as_ref().map_or_else(
                || Arc::new(make_blood_shot_sprite(8, 3)),
                |ss| Arc::clone(&ss.0.hover),
            ),
        };
        extra_bbs.0.push(Billboard {
            position: extrapolated,
            height: 0.0,
            world_height: 0.3,
            sprite,
            flip_x: false,
            palette_variant: None,
        });
    }

    // Hit impact billboards (blood splats / destroy animations).
    let is_web_shot =
        |pt: Option<NetProjectileType>| -> bool { matches!(pt, Some(NetProjectileType::WebShot)) };
    for impact in &hit_impacts.0 {
        let (sprite, world_height) = match impact.kind {
            HitImpactKind::Hit => {
                let s = if is_web_shot(impact.projectile_type) {
                    spider_shot_sprites.as_ref().map_or_else(
                        || Arc::new(make_blood_shot_sprite(8, 3)),
                        |ss| Arc::clone(&ss.0.hit),
                    )
                } else {
                    blood_shot_sprites.as_ref().map_or_else(
                        || Arc::new(make_blood_shot_sprite(8, 3)),
                        |bs| Arc::clone(&bs.0.hit),
                    )
                };
                (s, 0.42)
            }
            HitImpactKind::Destroy => {
                let s = if is_web_shot(impact.projectile_type) {
                    spider_shot_sprites.as_ref().map_or_else(
                        || Arc::new(make_blood_shot_sprite(8, 3)),
                        |ss| Arc::clone(ss.0.destroy_sprite_at(impact.age)),
                    )
                } else {
                    blood_shot_sprites.as_ref().map_or_else(
                        || Arc::new(make_blood_shot_sprite(8, 3)),
                        |bs| Arc::clone(bs.0.destroy_sprite_at(impact.age)),
                    )
                };
                (s, 0.36)
            }
        };
        extra_bbs.0.push(Billboard {
            position: impact.position,
            height: 0.15,
            world_height,
            sprite,
            flip_x: false,
            palette_variant: None,
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
        for (entity, np, _, _) in net_players.iter() {
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

/// MP adapter: convert `NetEnemyState` + replicated `visual_height` +
/// client-side `AttackAnimOverride` into the shared `EnemyPresentationState`.
///
/// Lives in the app crate because it needs both `carcinisation_net` and
/// `carcinisation_fps_core` types, and `fps` does not depend on `net`.
fn spidey_net_presentation_state(
    net_state: &NetEnemyState,
    visual_height: f32,
    visual_phase: f32,
    attack_override: Option<&AttackAnimOverride>,
) -> carcinisation_fps_core::EnemyPresentationState {
    use carcinisation_fps_core::presentation::{AttackPresentationKind, EnemyPresentationState};

    // If a one-shot attack animation is playing, it takes priority.
    if let Some(anim) = attack_override {
        let atk = match anim.kind {
            EnemyAttackKind::Melee => AttackPresentationKind::Melee,
            EnemyAttackKind::Ranged => AttackPresentationKind::Ranged,
        };
        return EnemyPresentationState::Attacking {
            attack: atk,
            phase: anim.elapsed,
        };
    }

    match net_state {
        NetEnemyState::Idle => EnemyPresentationState::Idle,
        NetEnemyState::Chase => {
            if visual_height > carcinisation_fps::spidey::SPIDEY_HOP_DETECTION_THRESHOLD {
                EnemyPresentationState::Hopping {
                    phase: visual_phase.clamp(0.0, 1.0),
                    visual_height,
                }
            } else {
                EnemyPresentationState::Moving
            }
        }
        NetEnemyState::HoldingRange => EnemyPresentationState::Recover,
        NetEnemyState::Dying { burn } => EnemyPresentationState::Dying {
            burn: *burn,
            phase: visual_phase,
        },
        NetEnemyState::Dead { burn } => EnemyPresentationState::Dead { burn: *burn },
    }
}

fn net_enemy_billboard(
    enemy: &NetEnemy,
    visual_pos: Vec2,
    elapsed_secs: f32,
    mosquiton_sprites: Option<&MosquitonSprites>,
    spidey_sprites: Option<&SpideySprites>,
    attack_override: Option<&AttackAnimOverride>,
    damage_invert: bool,
) -> Billboard {
    match enemy.enemy_type {
        NetEnemyType::Basic => Billboard {
            position: visual_pos,
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_enemy_sprite(32, 2)),
            flip_x: false,
            palette_variant: None,
        },
        NetEnemyType::Mosquiton => Billboard {
            position: visual_pos,
            height: 0.0,
            world_height: 0.9,
            palette_variant: None,
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
        NetEnemyType::Spidey => {
            use carcinisation_fps::spidey::{FLOOR_OFFSET, SPIDEY_BILLBOARD_HEIGHT};
            let pres = spidey_net_presentation_state(
                &enemy.state,
                enemy.visual_height,
                enemy.visual_phase,
                attack_override,
            );
            // Use replicated visual_height for billboard positioning (hop/leap arc).
            let grounded_height =
                FLOOR_OFFSET + SPIDEY_BILLBOARD_HEIGHT / 2.0 + enemy.visual_height;
            Billboard {
                position: visual_pos,
                height: grounded_height,
                world_height: SPIDEY_BILLBOARD_HEIGHT,
                palette_variant: None,
                sprite: spidey_sprites.map_or_else(
                    || Arc::new(make_enemy_sprite(32, 2)),
                    |sprites| {
                        carcinisation_fps::billboard::spidey_sprite_for_presentation(
                            &pres,
                            &sprites.0,
                            damage_invert,
                            elapsed_secs,
                        )
                    },
                ),
                flip_x: false,
            }
        }
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
            palette_variant: None,
        });
    }
}

/// Generate perimeter flame billboards on a living burning enemy.
/// Flame count scales with burn intensity via `burn_flame_count`.
fn push_alive_burn_flames(
    billboards: &mut Vec<Billboard>,
    position: Vec2,
    seed: u32,
    intensity: f32,
    elapsed: f32,
    camera_pos: Vec2,
    camera_dir: Vec2,
    sprite: &carapace::image::CxImage,
    attack_sprites: Option<&PlayerAttackSprites>,
    burn_config: &carcinisation_fps_core::BurnConfig,
) {
    use carapace::palette::TRANSPARENT_INDEX;

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

    let to_enemy = position - camera_pos;
    let distance = to_enemy.length().max(0.1);
    let behind_dir = if distance > 0.001 {
        to_enemy / distance
    } else {
        camera_dir
    };
    let right = Vec2::new(-camera_dir.y, camera_dir.x);

    let base_world_height: f32 = 0.9;
    let px_to_world = base_world_height / sprite.height() as f32;

    let flame_size = carcinisation_fps_core::burn_flame_scale(intensity, burn_config);
    for flame in all_flames.iter().take(visible) {
        let lateral_units = flame.offset_px.x * px_to_world;
        let vertical_units = flame.offset_px.y * px_to_world;
        let phase = elapsed + flame.phase_secs;
        let flame_sprite = attack_sprites.map_or_else(
            || Arc::new(make_blood_shot_sprite(6, 3)),
            |sprites| Arc::clone(sprites.flame_frame_loop(phase)),
        );
        billboards.push(Billboard {
            position: position - behind_dir * 0.04 + right * lateral_units,
            height: vertical_units,
            world_height: base_world_height * flame_size * flame.scale,
            sprite: flame_sprite,
            flip_x: false,
            palette_variant: None,
        });
    }
}

/// Generate flame billboards from stream samples for a remote player.
fn push_remote_flame_billboards(
    billboards: &mut Vec<Billboard>,
    elapsed: f32,
    attack_sprites: Option<&PlayerAttackSprites>,
    map: Option<&MapRes>,
    flame_3p_cfg: &PlayerFlamethrower3pConfig,
    params: &FlameVisualParams,
    samples: &[RemoteFlameStreamSample],
    player_position: Vec2,
    player_angle: f32,
    flame_cfg: &carcinisation_fps_core::PlayerFlamethrowerConfig,
) {
    use carcinisation_fps::raycast::cast_ray;

    let dir = Vec2::new(player_angle.cos(), player_angle.sin());
    let range_plus_nozzle = flame_cfg.range + flame_3p_cfg.nozzle_forward;

    // Wall-hit distance measured from player origin along current facing.
    let max_dist = map.map_or(range_plus_nozzle, |m| {
        let hit = cast_ray(&m.0, player_position, dir);
        if hit.wall_id > 0 {
            hit.distance.min(range_plus_nozzle)
        } else {
            range_plus_nozzle
        }
    });

    let max_age = flame_cfg.range / params.speed;
    let mut hit_wall = false;

    for sample in samples {
        let pos = sample.world_position(params.speed);
        // Wall clipping: distance from player origin along current facing.
        let dist_from_player = (pos - player_position).dot(dir);
        if dist_from_player >= max_dist {
            hit_wall = true;
            continue;
        }
        let t = (sample.age / max_age).clamp(0.0, 1.0);
        let phase = elapsed + sample.age * flame_3p_cfg.phase_step;
        let jitter = ((sample.seed as f32 * 7.31).sin() * flame_3p_cfg.jitter_amp) * t;
        let right = screen_right_from_direction(sample.emit_direction);
        let sprite = attack_sprites.map_or_else(
            || Arc::new(make_blood_shot_sprite(6, 3)),
            |sprites| Arc::clone(sprites.flame_frame_loop(phase)),
        );
        let world_height = flame_3p_cfg.flame_scale_near
            + (flame_3p_cfg.flame_scale_far - flame_3p_cfg.flame_scale_near) * t;
        billboards.push(Billboard {
            position: pos + right * jitter,
            height: flame_3p_cfg.nozzle_height,
            world_height,
            sprite,
            flip_x: false,
            palette_variant: None,
        });
    }

    // Wall impact billboard (placed on aim line, no nozzle lateral).
    if hit_wall && max_dist < range_plus_nozzle {
        let impact_dist = (max_dist - flame_3p_cfg.wall_offset).max(flame_3p_cfg.nozzle_forward);
        let sprite = attack_sprites.map_or_else(
            || Arc::new(make_blood_shot_sprite(8, 3)),
            |sprites| Arc::clone(sprites.flame_wall_hit_frame_loop(elapsed)),
        );
        billboards.push(Billboard {
            position: player_position + dir * impact_dist,
            height: flame_3p_cfg.nozzle_height,
            world_height: flame_3p_cfg.impact_scale,
            sprite,
            flip_x: false,
            palette_variant: None,
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
    use carcinisation_fps::plugin::{CameraRes, PlayerHealth};
    use carcinisation_fps::spidey::make_spidey_billboard_sprites;
    use carcinisation_fps_core::presentation::{AttackPresentationKind, EnemyPresentationState};
    use carcinisation_net::{NetPickupKind, NetworkObjectId, PlayerNetState};

    fn init_sync_test_app(app: &mut App) {
        app.init_resource::<carcinisation_fps::plugin::ExtraBillboards>();
        app.init_resource::<FpsScreenParticles>();
        app.init_resource::<EnemyAttackOverrides>();
        app.init_resource::<EnemyDamageFlickers>();
        app.init_resource::<HitImpacts>();
        app.init_resource::<PickupImpacts>();
        app.init_resource::<HealthPickupScreenFeedback>();
        app.insert_resource(PlayerFlamethrower3pConfig::default());
        app.insert_resource(carcinisation_fps_core::PlayerFlamethrowerConfig::load());
        app.insert_resource(carcinisation_fps_core::BurnConfig::default());
        app.insert_resource(ScreenParticleConfig::default());
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
            avatar_palette_variant: None,
        });
        app.world_mut().spawn((
            NetEnemy {
                object_id: NetworkObjectId(1),
                position: Vec2::new(4.0, 5.0),
                angle: 0.0,
                state: NetEnemyState::Idle,
                enemy_type: NetEnemyType::Mosquiton,
                visual_height: 0.0,
                visual_phase: 0.0,
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
                visual_height: 0.0,
                visual_phase: 0.0,
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
            avatar_palette_variant: None,
        });
        app.world_mut().spawn((
            NetEnemy {
                object_id: NetworkObjectId(1),
                position: Vec2::new(4.0, 5.0),
                angle: 0.0,
                state: NetEnemyState::Chase,
                enemy_type: NetEnemyType::Mosquiton,
                visual_height: 0.0,
                visual_phase: 0.0,
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
    fn pickup_billboards_use_small_ground_anchored_prop_scale() {
        let mut app = App::new();
        app.init_resource::<carcinisation_fps::plugin::ExtraBillboards>();
        app.insert_resource(carcinisation_fps::billboard::make_pickup_billboard_sprites().unwrap());
        app.add_systems(Update, queue_pickup_billboards);

        app.world_mut().spawn(NetPickup {
            object_id: NetworkObjectId(10),
            position: Vec2::new(4.0, 5.0),
            kind: NetPickupKind::Health,
            available: true,
            respawn_remaining: None,
            respawnable: true,
        });

        app.update();

        let extra_bbs = app
            .world()
            .resource::<carcinisation_fps::plugin::ExtraBillboards>();
        assert_eq!(extra_bbs.0.len(), 1);
        assert_eq!(extra_bbs.0[0].world_height, PICKUP_BILLBOARD_WORLD_HEIGHT);
        assert_eq!(
            extra_bbs.0[0].height,
            carcinisation_fps::spidey::FLOOR_OFFSET + PICKUP_BILLBOARD_WORLD_HEIGHT / 2.0
        );
    }

    #[test]
    fn replicated_net_health_updates_local_health_hud() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalPlayerId(Some(PlayerId(1))));
        app.insert_resource(PlayerHealth(25));
        app.add_systems(Update, sync_local_player_health_from_net_health);

        let player = app
            .world_mut()
            .spawn((
                NetPlayer {
                    player_id: PlayerId(1),
                    position: Vec2::new(2.0, 3.0),
                    angle: 0.25,
                    current_attack: NetAttackId::None,
                    state: PlayerNetState::Alive,
                    flame_active: false,
                    avatar_palette_variant: None,
                },
                NetHealth {
                    current: 75.0,
                    max: 100.0,
                },
            ))
            .id();

        app.update();
        assert_eq!(app.world().resource::<PlayerHealth>().0, 75);

        app.world_mut()
            .get_mut::<NetHealth>(player)
            .unwrap()
            .current = 90.0;
        app.update();
        assert_eq!(app.world().resource::<PlayerHealth>().0, 90);
    }

    fn setup_pickup_feedback_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalPlayerId(Some(PlayerId(1))));
        app.insert_resource(Config::default());
        app.insert_resource(ScreenParticleConfig::default());
        app.init_resource::<PickupImpacts>();
        app.init_resource::<FpsScreenParticles>();
        app.init_resource::<HealthPickupScreenFeedback>();
        app.add_observer(handle_pickup_effect);
        app
    }

    #[test]
    fn local_health_pickup_effect_triggers_screen_particles() {
        let mut app = setup_pickup_feedback_test_app();

        app.world_mut().trigger(PickupEffect {
            player_id: PlayerId(1),
            pickup_id: NetworkObjectId(10),
            kind: NetPickupKind::Health,
            position: Vec2::new(4.0, 5.0),
        });

        assert_eq!(app.world().resource::<PickupImpacts>().0.len(), 1);
        assert_eq!(
            app.world().resource::<FpsScreenParticles>().len(),
            16,
            "local health pickup should spawn one SUBTLE screen burst"
        );
    }

    #[test]
    fn remote_health_pickup_effect_does_not_trigger_screen_particles() {
        let mut app = setup_pickup_feedback_test_app();

        app.world_mut().trigger(PickupEffect {
            player_id: PlayerId(2),
            pickup_id: NetworkObjectId(10),
            kind: NetPickupKind::Health,
            position: Vec2::new(4.0, 5.0),
        });

        assert_eq!(app.world().resource::<PickupImpacts>().0.len(), 1);
        assert_eq!(app.world().resource::<FpsScreenParticles>().len(), 0);
    }

    #[test]
    fn non_health_pickup_effect_does_not_trigger_screen_particles() {
        let mut app = setup_pickup_feedback_test_app();

        app.world_mut().trigger(PickupEffect {
            player_id: PlayerId(1),
            pickup_id: NetworkObjectId(10),
            kind: NetPickupKind::Ammo,
            position: Vec2::new(4.0, 5.0),
        });

        assert_eq!(app.world().resource::<PickupImpacts>().0.len(), 1);
        assert_eq!(app.world().resource::<FpsScreenParticles>().len(), 0);
    }

    #[test]
    fn positive_local_net_health_delta_triggers_fallback_screen_particles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalPlayerId(Some(PlayerId(1))));
        app.insert_resource(PlayerHealth(50));
        app.insert_resource(Config::default());
        app.insert_resource(ScreenParticleConfig::default());
        app.init_resource::<FpsScreenParticles>();
        app.init_resource::<HealthPickupScreenFeedback>();
        app.add_systems(Update, sync_local_player_health_from_net_health);

        let player = app
            .world_mut()
            .spawn((
                NetPlayer {
                    player_id: PlayerId(1),
                    position: Vec2::new(2.0, 3.0),
                    angle: 0.25,
                    current_attack: NetAttackId::None,
                    state: PlayerNetState::Alive,
                    flame_active: false,
                    avatar_palette_variant: None,
                },
                NetHealth {
                    current: 50.0,
                    max: 100.0,
                },
            ))
            .id();

        app.update();
        assert_eq!(app.world().resource::<FpsScreenParticles>().len(), 0);

        app.world_mut()
            .get_mut::<NetHealth>(player)
            .unwrap()
            .current = 75.0;
        app.update();

        assert_eq!(app.world().resource::<PlayerHealth>().0, 75);
        assert_eq!(app.world().resource::<FpsScreenParticles>().len(), 16);
    }

    #[test]
    fn pickup_effect_suppresses_followup_net_health_fallback_burst() {
        let mut app = setup_pickup_feedback_test_app();
        app.insert_resource(PlayerHealth(50));
        app.add_systems(Update, sync_local_player_health_from_net_health);

        let player = app
            .world_mut()
            .spawn((
                NetPlayer {
                    player_id: PlayerId(1),
                    position: Vec2::new(2.0, 3.0),
                    angle: 0.25,
                    current_attack: NetAttackId::None,
                    state: PlayerNetState::Alive,
                    flame_active: false,
                    avatar_palette_variant: None,
                },
                NetHealth {
                    current: 50.0,
                    max: 100.0,
                },
            ))
            .id();

        app.update();
        app.world_mut().trigger(PickupEffect {
            player_id: PlayerId(1),
            pickup_id: NetworkObjectId(10),
            kind: NetPickupKind::Health,
            position: Vec2::new(4.0, 5.0),
        });
        assert_eq!(app.world().resource::<FpsScreenParticles>().len(), 16);

        app.world_mut()
            .get_mut::<NetHealth>(player)
            .unwrap()
            .current = 80.0;
        app.update();

        assert_eq!(
            app.world().resource::<FpsScreenParticles>().len(),
            16,
            "NetHealth fallback should not duplicate the recent PickupEffect burst"
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
    fn spidey_net_presentation_maps_all_states() {
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::Idle, 0.0, 0.0, None),
            EnemyPresentationState::Idle
        );
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::Chase, 0.0, 0.0, None),
            EnemyPresentationState::Moving
        );
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::Chase, 0.3, 0.75, None),
            EnemyPresentationState::Hopping {
                phase: 0.75,
                visual_height: 0.3,
            }
        );
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::HoldingRange, 0.0, 0.0, None),
            EnemyPresentationState::Recover
        );
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::Dying { burn: true }, 0.0, 0.0, None,),
            EnemyPresentationState::Dying {
                burn: true,
                phase: 0.0,
            }
        );
        // Non-zero death phase from server-replicated visual_phase.
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::Dying { burn: false }, 0.0, 0.75, None,),
            EnemyPresentationState::Dying {
                burn: false,
                phase: 0.75,
            }
        );
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::Dead { burn: false }, 0.0, 0.0, None,),
            EnemyPresentationState::Dead { burn: false }
        );
    }

    #[test]
    fn spidey_net_attack_override_takes_priority() {
        let override_anim = AttackAnimOverride {
            kind: EnemyAttackKind::Ranged,
            elapsed: 0.25,
            duration: 1.0,
        };
        assert_eq!(
            spidey_net_presentation_state(&NetEnemyState::Chase, 0.3, 0.75, Some(&override_anim),),
            EnemyPresentationState::Attacking {
                attack: AttackPresentationKind::Ranged,
                phase: 0.25,
            }
        );
    }

    #[test]
    fn spidey_sprite_for_each_net_presentation_is_non_empty() {
        let sprites = make_spidey_billboard_sprites().unwrap();
        let states = [
            EnemyPresentationState::Idle,
            EnemyPresentationState::Moving,
            EnemyPresentationState::Hopping {
                phase: 0.5,
                visual_height: 0.3,
            },
            EnemyPresentationState::Windup {
                attack: AttackPresentationKind::Ranged,
                phase: 0.1,
            },
            EnemyPresentationState::Attacking {
                attack: AttackPresentationKind::Melee,
                phase: 0.1,
            },
            EnemyPresentationState::Recover,
            EnemyPresentationState::Dying {
                burn: false,
                phase: 0.0,
            },
            EnemyPresentationState::Dead { burn: true },
        ];
        for state in states {
            let sprite = carcinisation_fps::billboard::spidey_sprite_for_presentation(
                &state, &sprites, false, 0.0,
            );
            assert!(
                !sprite.data().is_empty(),
                "state {state:?} should resolve a sprite"
            );
        }
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
            avatar_palette_variant: None,
        });

        // Remote player with flame_active = true (replicated authoritative state).
        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(2),
            position: Vec2::new(4.0, 5.0),
            angle: 0.0,
            current_attack: NetAttackId::Projectile,
            state: PlayerNetState::Alive,
            flame_active: true,
            avatar_palette_variant: None,
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
            avatar_palette_variant: None,
        });

        // Remote player with flamethrower equipped but NOT firing.
        app.world_mut().spawn(NetPlayer {
            player_id: PlayerId(2),
            position: Vec2::new(4.0, 5.0),
            angle: 0.0,
            current_attack: NetAttackId::Projectile,
            state: PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
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
