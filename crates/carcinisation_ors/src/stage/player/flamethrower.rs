use crate::stage::{
    components::{
        interactive::{ColliderData, Hittable},
        placement::Depth,
    },
    messages::DamageMessage,
    player::{
        components::{Player, PlayerAttack},
        intent::PlayerIntent,
    },
    resources::StageTimeDomain,
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxAnimationBundle, CxAnimationDirection, CxAnimationDuration,
    CxAnimationFinishBehavior, CxAtlasSprite, CxCamera, CxFrameTransition, CxPosition,
    CxPresentationTransform, CxRenderSpace, CxSpriteAtlasAsset, WorldPos,
};
use carcinisation_base::fire_death::FireDeathConfig;
use carcinisation_base::layer::FlameDepth;
use carcinisation_base::layer::Layer;
use carcinisation_base::layer::OrsLayer;
use carcinisation_core::globals::SCREEN_RESOLUTION;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const CONFIG_PATH: &str = "assets/config/attacks/player_flamethrower_ors.ron";
const EMBEDDED_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/config/attacks/player_flamethrower_ors.ron"
));

#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct FlamethrowerConfig {
    pub atlas_path: String,
    pub animation_tag: String,
    /// Maximum number of flame particles alive at once.
    pub max_flames: u8,
    /// Total chain range (px). Replaces uniform spacing.
    pub chain_range: f32,
    /// Spacing curve exponent. <1 = wider near origin, tighter at end.
    /// 1.0 = uniform spacing.
    pub spacing_curve: f32,
    /// How fast each flame particle travels along the chain (px/s).
    pub flame_speed: f32,
    /// Speed multiplier when draining (A released). >1 = faster disappearance.
    pub drain_speed_multiplier: f32,
    /// Time between spawning new flame particles (ms).
    pub spawn_interval_ms: u64,
    pub damage_per_tick: u32,
    pub tick_ms: u64,
    pub max_ammo: f32,
    pub ammo_drain_per_ms: f32,
    pub origin_camera_px: (f32, f32),
    /// Hit radius per flame particle for damage checks (px).
    pub hit_radius: f32,
    /// Base angular follow speed for the nearest particle (units/s).
    pub angular_follow_speed: f32,
    /// Per-slot drag on angular follow. Each slot index divides the follow
    /// speed by `(1 + slot * angular_drag)`, creating a whip-like trail.
    pub angular_drag: f32,
    /// Scale at the origin end of the chain (nearest particle).
    pub scale_near: f32,
    /// Scale at the far end of the chain (farthest particle).
    pub scale_far: f32,
    /// Maximum depth that the flamethrower can damage (inclusive).
    pub max_damage_depth: i8,
    pub burning_corpse_duration_secs: f32,
    pub burning_flame_count: usize,
    pub burning_flame_perimeter_padding_px: f32,
    pub burning_flame_jitter_px: f32,
    pub burning_flame_scale_min: f32,
    pub burning_flame_scale_max: f32,
}

impl FlamethrowerConfig {
    #[must_use]
    pub fn load() -> Self {
        #[cfg(not(target_family = "wasm"))]
        if let Ok(body) = std::fs::read_to_string(CONFIG_PATH) {
            return Self::parse_and_validate(&body, CONFIG_PATH);
        }

        Self::parse_and_validate(EMBEDDED_CONFIG, "embedded player_flamethrower.ron")
    }

    fn parse_and_validate(ron_str: &str, source: &str) -> Self {
        let config: Self = ron::from_str(ron_str).unwrap_or_else(|e| {
            panic!("{source}: failed to parse FlamethrowerConfig: {e}");
        });
        config.validate(source);
        config
    }

    fn validate(&self, source: &str) {
        assert!(self.max_flames > 0, "{source}: max_flames must be > 0");
        assert!(
            self.chain_range > 0.0,
            "{source}: chain_range must be positive",
        );
        assert!(
            self.spacing_curve > 0.0,
            "{source}: spacing_curve must be positive",
        );
        assert!(
            self.flame_speed > 0.0,
            "{source}: flame_speed must be positive",
        );
        assert!(
            self.spawn_interval_ms > 0,
            "{source}: spawn_interval_ms must be > 0",
        );
        assert!(self.max_ammo > 0.0, "{source}: max_ammo must be positive");
        assert!(self.tick_ms > 0, "{source}: tick_ms must be positive");
        assert!(
            self.hit_radius > 0.0,
            "{source}: hit_radius must be positive",
        );
        assert!(
            self.drain_speed_multiplier > 0.0,
            "{source}: drain_speed_multiplier must be positive",
        );
        assert!(
            self.angular_follow_speed > 0.0,
            "{source}: angular_follow_speed must be positive",
        );
        assert!(
            self.scale_near > 0.0 && self.scale_far > 0.0,
            "{source}: scale_near and scale_far must be positive",
        );
        assert!(
            self.burning_corpse_duration_secs >= 0.0,
            "{source}: burning_corpse_duration_secs must be non-negative",
        );
        assert!(
            self.burning_flame_scale_min > 0.0 && self.burning_flame_scale_max > 0.0,
            "{source}: burning flame scales must be positive",
        );
    }

    #[must_use]
    pub fn tick_duration(&self) -> Duration {
        Duration::from_millis(self.tick_ms)
    }

    #[must_use]
    pub fn spawn_interval(&self) -> Duration {
        Duration::from_millis(self.spawn_interval_ms)
    }

    #[must_use]
    pub fn origin(&self) -> Vec2 {
        Vec2::new(self.origin_camera_px.0, self.origin_camera_px.1)
    }

    /// Maximum distance a flame can travel before despawning.
    #[must_use]
    pub fn max_range(&self) -> f32 {
        self.chain_range
    }

    /// Target distance for a given slot index, applying the spacing curve.
    /// Slot 0 = nearest, max_flames-1 = farthest.
    #[must_use]
    pub fn slot_target(&self, slot: u8) -> f32 {
        let t = (f32::from(slot) + 1.0) / f32::from(self.max_flames);
        self.chain_range * t.powf(self.spacing_curve)
    }

    #[must_use]
    pub fn fire_death_config(&self) -> FireDeathConfig {
        FireDeathConfig {
            burning_corpse_duration_secs: self.burning_corpse_duration_secs,
            burning_flame_count: self.burning_flame_count,
            burning_flame_perimeter_padding_px: self.burning_flame_perimeter_padding_px,
            burning_flame_jitter_px: self.burning_flame_jitter_px,
            burning_flame_scale_min: self.burning_flame_scale_min,
            burning_flame_scale_max: self.burning_flame_scale_max,
        }
    }
}

/// Compute flame chain direction from cursor screen-space X position.
///
/// Maps the cursor's horizontal position across the viewport to an angle
/// in the range \[-90°, +90°\] from vertical. `x=0` → -90° (pointing left),
/// `x=viewport_width` → +90° (pointing right).
fn flame_direction(cursor_screen_x: f32) -> Vec2 {
    let screen_w = SCREEN_RESOLUTION.x as f32;
    let normalized = (cursor_screen_x / screen_w).clamp(0.0, 1.0);
    let angle = (normalized * 2.0 - 1.0) * std::f32::consts::FRAC_PI_2;
    Vec2::new(angle.sin(), angle.cos())
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Root entity for the flamethrower. Exists from first press until all
/// particles have drained away after release.
#[derive(Component, Debug)]
pub struct ActiveFlamethrower {
    pub ammo: f32,
    /// Whether new flames are being spawned (true while A held, false after release).
    pub spawning: bool,
    /// Next time a flame particle should be emitted.
    pub next_spawn_at: Duration,
    /// Next slot index to assign to a newly spawned particle.
    pub next_slot: u8,
    /// Cached atlas handle + region for spawning particles.
    pub atlas_handle: Handle<CxSpriteAtlasAsset>,
    pub region_id: carapace::prelude::AtlasRegionId,
    pub anim_duration_ms: u64,
}

/// Tick-based damage state per enemy target.
#[derive(Component, Debug, Default)]
pub struct FlamethrowerTickState {
    last_tick: HashMap<Entity, Duration>,
}

impl FlamethrowerTickState {
    #[must_use]
    pub fn can_tick(&self, target: Entity, now: Duration, interval: Duration) -> bool {
        self.last_tick
            .get(&target)
            .is_none_or(|&last| now.saturating_sub(last) >= interval)
    }

    pub fn register_tick(&mut self, target: Entity, now: Duration) {
        self.last_tick.insert(target, now);
    }

    pub fn retain_alive<D: bevy::ecs::query::QueryData>(
        &mut self,
        alive: &Query<D, With<Hittable>>,
    ) {
        self.last_tick.retain(|e, _| alive.contains(*e));
    }
}

/// A single flame particle travelling along the chain.
#[derive(Component, Debug)]
pub struct FlameParticle {
    /// Distance travelled from origin along the chain direction (px).
    pub progress: f32,
    /// Target distance — the slot this particle fills (spacing * (slot + 1)).
    pub target: f32,
    /// This particle's current direction (lerps toward the chain direction).
    pub direction: Vec2,
    /// Slot index (0 = nearest, used for angular drag calculation).
    pub slot: u8,
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Manages the flamethrower lifecycle: spawn root on press, stop spawning
/// on release, despawn root when all particles are gone.
#[allow(clippy::too_many_arguments)]
pub fn manage_flamethrower(
    mut commands: Commands,
    intent: Res<PlayerIntent>,
    config: Res<FlamethrowerConfig>,
    asset_server: Res<AssetServer>,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    mut active_query: Query<(Entity, &mut ActiveFlamethrower)>,
    particle_query: Query<Entity, With<FlameParticle>>,
    loadout: Res<crate::stage::player::attacks::AttackLoadout>,
    time: Res<Time<StageTimeDomain>>,
) {
    let is_flamethrower =
        loadout.current() == crate::stage::player::attacks::AttackId::Flamethrower;

    // Handle existing flamethrower.
    if let Ok((root_entity, mut flamethrower)) = active_query.single_mut() {
        // Stop spawning on release, loadout change, or ammo depletion.
        if flamethrower.spawning
            && (!is_flamethrower
                || intent.shoot_just_released
                || (!intent.shoot_held && !intent.shoot_just_pressed)
                || flamethrower.ammo <= 0.0)
        {
            flamethrower.spawning = false;
        }

        // Despawn root when no longer spawning AND all particles are gone.
        if !flamethrower.spawning && particle_query.is_empty() {
            commands.entity(root_entity).despawn();
        }
        return;
    }

    // Spawn root if not active, shoot pressed, and correct loadout.
    if is_flamethrower && intent.shoot_just_pressed {
        let atlas_handle: Handle<CxSpriteAtlasAsset> = asset_server.load(config.atlas_path.clone());
        let region_id = atlas_assets
            .get(&atlas_handle)
            .and_then(|a| a.region_id(&config.animation_tag))
            .unwrap_or_default();
        let anim_duration_ms = atlas_assets
            .get(&atlas_handle)
            .and_then(|a| a.animation(&config.animation_tag))
            .map_or(400, |a| a.duration_ms);

        let origin = config.origin();
        let now = time.elapsed();

        commands.spawn((
            PlayerAttack {
                attack_id: crate::stage::player::attacks::AttackId::Flamethrower,
                position: origin,
            },
            ActiveFlamethrower {
                ammo: config.max_ammo,
                spawning: true,
                next_spawn_at: now,
                next_slot: 0,
                atlas_handle,
                region_id,
                anim_duration_ms,
            },
            FlamethrowerTickState::default(),
            WorldPos::from(origin),
            CxPosition::from(origin.round().as_ivec2()),
            CxRenderSpace::Camera,
            CxAnchor::Center,
            Layer::Ors(OrsLayer::Attack),
            Name::new("Flamethrower"),
        ));
    }
}

/// Spawns new flame particles, advances existing ones, and despawns those
/// that exceed max range. Also drains ammo.
#[allow(clippy::too_many_arguments)]
pub fn update_flamethrower(
    mut commands: Commands,
    config: Res<FlamethrowerConfig>,
    time: Res<Time<StageTimeDomain>>,
    player_query: Query<
        &WorldPos,
        (
            With<Player>,
            Without<ActiveFlamethrower>,
            Without<FlameParticle>,
        ),
    >,
    mut active_query: Query<&mut ActiveFlamethrower, (Without<Player>, Without<FlameParticle>)>,
    mut particle_query: Query<
        (
            Entity,
            &mut FlameParticle,
            &mut WorldPos,
            &mut CxPosition,
            &mut Layer,
            &mut CxPresentationTransform,
            &mut ColliderData,
        ),
        (Without<ActiveFlamethrower>, Without<Player>),
    >,
) {
    let Ok(mut flamethrower) = active_query.single_mut() else {
        return;
    };
    let Some(player_screen) = player_query.single().ok() else {
        return;
    };

    let dt = time.delta_secs();
    let now = time.elapsed();
    let origin = config.origin();
    let direction = flame_direction(player_screen.0.x);
    let max_range = config.max_range();

    // Drain ammo while spawning.
    if flamethrower.spawning {
        let dt_ms = dt * 1000.0;
        flamethrower.ammo -= dt_ms * config.ammo_drain_per_ms;
    }

    // Advance existing particles.
    let speed = if flamethrower.spawning {
        config.flame_speed
    } else {
        config.flame_speed * config.drain_speed_multiplier
    };

    for (
        entity,
        mut particle,
        mut world_pos,
        mut cx_pos,
        mut layer,
        mut presentation,
        mut collider,
    ) in &mut particle_query
    {
        particle.progress += speed * dt;

        if flamethrower.spawning {
            // While chain is forming/steady: clamp at slot target.
            particle.progress = particle.progress.min(particle.target);
        } else {
            // Draining: flame continues past its slot, despawn at max range.
            if particle.progress >= max_range {
                commands.entity(entity).despawn();
                continue;
            }
        }

        // Angular lag: further slots follow the current direction more slowly.
        let follow =
            config.angular_follow_speed / (1.0 + f32::from(particle.slot) * config.angular_drag);
        let t = (follow * dt).min(1.0);
        let lerped = particle.direction.lerp(direction, t);
        // Avoid zero-length collapse when directions are nearly opposite.
        particle.direction = if lerped.length_squared() > 1e-6 {
            lerped.normalize()
        } else {
            particle.direction
        };

        let pos = origin + particle.direction * particle.progress;
        world_pos.0 = pos;
        cx_pos.0 = pos.round().as_ivec2();

        // Closer to origin = higher render priority (renders on top).
        let progress_t = (particle.progress / max_range).clamp(0.0, 1.0);
        let depth = ((1.0 - progress_t) * 15.0) as u8;
        *layer = Layer::Ors(OrsLayer::FlameSegment(FlameDepth(depth)));

        // Interpolate scale from near (origin) to far (max range).
        let scale = config.scale_near + (config.scale_far - config.scale_near) * progress_t;
        presentation.scale = Vec2::splat(scale);

        // Scale collider to match visual size.
        *collider = ColliderData::from_one(carcinisation_collision::Collider::new_circle(
            config.hit_radius * scale,
        ));
    }

    // Spawn new particles at the configured interval while chain isn't full.
    if flamethrower.spawning
        && now >= flamethrower.next_spawn_at
        && flamethrower.next_slot < config.max_flames
    {
        let slot = flamethrower.next_slot;
        let target = config.slot_target(slot);

        let anim_bundle = CxAnimationBundle::from_parts(
            CxAnimationDirection::Forward,
            CxAnimationDuration::millis_per_animation(flamethrower.anim_duration_ms),
            CxAnimationFinishBehavior::Loop,
            CxFrameTransition::None,
        );

        commands.spawn((
            FlameParticle {
                progress: 0.0,
                target,
                direction,
                slot,
            },
            CxAtlasSprite::new(flamethrower.atlas_handle.clone(), flamethrower.region_id),
            anim_bundle,
            WorldPos::from(origin),
            CxPosition::from(origin.round().as_ivec2()),
            CxAnchor::Center,
            ColliderData::from_one(carcinisation_collision::Collider::new_circle(
                config.hit_radius,
            )),
            CxPresentationTransform {
                scale: Vec2::splat(config.scale_near),
                ..default()
            },
            CxRenderSpace::Camera,
            Layer::Ors(OrsLayer::FlameSegment(FlameDepth(15))), // starts at highest (closest to origin)
            Name::new("FlameParticle"),
        ));

        flamethrower.next_slot += 1;
        flamethrower.next_spawn_at = now + config.spawn_interval();
    }
}

/// Applies tick-based damage from flame particles to enemies in range.
/// Converts particle camera-space positions to world-space for comparison.
#[allow(clippy::too_many_arguments)]
pub fn flamethrower_damage(
    config: Res<FlamethrowerConfig>,
    time: Res<Time<StageTimeDomain>>,
    camera: Res<CxCamera>,
    mut event_writer: MessageWriter<DamageMessage>,
    mut active_query: Query<&mut FlamethrowerTickState, With<ActiveFlamethrower>>,
    particle_query: Query<(&WorldPos, &FlameParticle)>,
    hittable_query: Query<(Entity, &WorldPos, Option<&Depth>), With<Hittable>>,
) {
    let Ok(mut tick_state) = active_query.single_mut() else {
        return;
    };

    let now = time.elapsed();
    let tick_interval = config.tick_duration();
    let base_radius = config.hit_radius;
    let max_range = config.max_range();
    let camera_offset = camera.0.as_vec2();

    // Collect particle world positions with per-particle scaled hit radius.
    let particles: Vec<(Vec2, f32)> = particle_query
        .iter()
        .map(|(pos, particle)| {
            let world_pos = pos.0 + camera_offset;
            // Scale hit radius to match visual scale at this progress.
            let progress_t = (particle.progress / max_range).clamp(0.0, 1.0);
            let scale = config.scale_near + (config.scale_far - config.scale_near) * progress_t;
            (world_pos, base_radius * scale)
        })
        .collect();

    tick_state.retain_alive(&hittable_query);

    for (target_entity, target_pos, target_depth) in hittable_query.iter() {
        // Skip entities deeper than the configured max damage depth.
        if let Some(depth) = target_depth
            && depth.to_i8() > config.max_damage_depth
        {
            continue;
        }

        if !tick_state.can_tick(target_entity, now, tick_interval) {
            continue;
        }

        let hit = particles
            .iter()
            .find(|(seg_pos, radius)| seg_pos.distance(target_pos.0) <= *radius);

        if hit.is_some() {
            tick_state.register_tick(target_entity, now);
            event_writer.write(DamageMessage::fire(target_entity, config.damage_per_tick));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_parses_and_validates() {
        let config: FlamethrowerConfig = ron::from_str(EMBEDDED_CONFIG)
            .expect("embedded player_flamethrower.ron must parse into FlamethrowerConfig");
        config.validate("embedded player_flamethrower.ron");
    }

    #[test]
    fn tick_state_respects_interval() {
        let mut state = FlamethrowerTickState::default();
        let target = Entity::from_bits(1);
        let interval = Duration::from_millis(100);

        assert!(state.can_tick(target, Duration::ZERO, interval));
        state.register_tick(target, Duration::ZERO);
        assert!(!state.can_tick(target, Duration::from_millis(50), interval));
        assert!(state.can_tick(target, Duration::from_millis(100), interval));
    }

    #[test]
    fn config_origin_converts() {
        let config = FlamethrowerConfig::load();
        let origin = config.origin();
        assert!((origin.x - 80.0).abs() < f32::EPSILON);
        assert!((origin.y - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn flame_direction_center_points_up() {
        let dir = flame_direction(80.0); // center of 160px screen
        assert!(dir.x.abs() < 0.01, "x should be ~0, got {}", dir.x);
        assert!((dir.y - 1.0).abs() < 0.01, "y should be ~1, got {}", dir.y);
    }

    #[test]
    fn flame_direction_edges() {
        let left = flame_direction(0.0);
        assert!(
            (left.x - (-1.0)).abs() < 0.01,
            "left edge x should be ~-1, got {}",
            left.x
        );

        let right = flame_direction(160.0);
        assert!(
            (right.x - 1.0).abs() < 0.01,
            "right edge x should be ~1, got {}",
            right.x
        );
    }

    #[test]
    fn max_range_matches_chain_range() {
        let config = FlamethrowerConfig::load();
        assert!((config.max_range() - config.chain_range).abs() < f32::EPSILON);
    }

    #[test]
    fn slot_targets_are_monotonically_increasing() {
        let config = FlamethrowerConfig::load();
        let mut prev = 0.0;
        for slot in 0..config.max_flames {
            let target = config.slot_target(slot);
            assert!(target > prev, "slot {slot}: {target} should be > {prev}");
            prev = target;
        }
        // Last slot should reach chain_range.
        assert!(
            (prev - config.chain_range).abs() < 0.01,
            "last slot should reach chain_range, got {prev}"
        );
    }
}
