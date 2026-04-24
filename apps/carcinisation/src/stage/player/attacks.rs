use crate::layer::Layer;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::{CxAnchor, CxAnimationFinishBehavior, CxFrameTransition, CxRenderSpace};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum AttackId {
    Pincer,
    Pistol,
    MachineGun,
    Bomb,
    BombExplosion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackCategory {
    Melee,
    Ranged,
}

#[derive(Clone, Copy, Debug)]
pub enum AttackCollisionMode {
    None,
    Point,
    SpriteMask,
    Radial { radius: f32 },
}

#[derive(Clone, Copy, Debug)]
pub enum AttackInputPolicy {
    Release,
    Hold {
        warmup_secs: f32,
        interval_secs: f32,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum AttackHitPolicy {
    Single,
    Repeat {
        cooldown_secs: f32,
        repeat_damage: u32,
    },
}

#[derive(Clone, Debug)]
pub struct AttackSpriteDefinition {
    pub sprite_path: &'static str,
    pub frames: usize,
    pub speed_ms: u64,
    pub finish_behavior: CxAnimationFinishBehavior,
    pub frame_transition: CxFrameTransition,
    pub anchor: CxAnchor,
    pub canvas: CxRenderSpace,
    pub layer: Layer,
}

#[derive(Clone, Copy, Debug)]
pub struct AttackEffects {
    pub screen_shake: bool,
}

#[derive(Clone, Debug)]
pub struct AttackDefinition {
    pub id: AttackId,
    pub name: &'static str,
    pub category: AttackCategory,
    pub damage: u32,
    pub duration_secs: f32,
    pub collision: AttackCollisionMode,
    /// Pixel offsets from the attack centre to test for `Point` collision.
    ///
    /// Each offset is tested in order; the first opaque-pixel hit wins.
    /// An empty list means "centre only" (single-point, legacy behaviour).
    /// A cross pattern `[(0,0),(1,0),(-1,0),(0,1),(0,-1)]` covers the
    /// centre plus four cardinal neighbours.
    pub hit_offsets: Vec<IVec2>,
    pub spawn_on_expire: Option<AttackId>,
    pub detonates_on_hit: bool,
    pub input_policy: AttackInputPolicy,
    pub hit_policy: AttackHitPolicy,
    pub aim_spread: f32,
    pub sprite: AttackSpriteDefinition,
    pub sfx_path: Option<&'static str>,
    pub effects: AttackEffects,
}

#[derive(Resource)]
pub struct AttackDefinitions {
    defs: HashMap<AttackId, AttackDefinition>,
}

impl AttackDefinitions {
    /// Returns the [`AttackDefinition`] for the given [`AttackId`].
    ///
    /// # Panics
    ///
    /// Panics if no definition has been registered for `id`.
    #[must_use]
    pub fn get(&self, id: AttackId) -> &AttackDefinition {
        self.defs
            .get(&id)
            .unwrap_or_else(|| panic!("missing attack definition for {id:?}"))
    }
}

impl Default for AttackDefinitions {
    #[allow(clippy::too_many_lines)]
    fn default() -> Self {
        let mut defs = HashMap::new();
        defs.insert(
            AttackId::Pincer,
            AttackDefinition {
                id: AttackId::Pincer,
                name: "Pincer",
                category: AttackCategory::Melee,
                damage: 70,
                duration_secs: 0.6,
                collision: AttackCollisionMode::SpriteMask,
                hit_offsets: vec![],
                spawn_on_expire: None,
                detonates_on_hit: false,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Repeat {
                    cooldown_secs: 0.18,
                    repeat_damage: 35,
                },
                aim_spread: 0.0,
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/melee_slash.px_sprite.png"),
                    frames: 9,
                    speed_ms: 500,
                    finish_behavior: CxAnimationFinishBehavior::Despawn,
                    frame_transition: CxFrameTransition::None,
                    anchor: CxAnchor::Center,
                    canvas: CxRenderSpace::Camera,
                    layer: Layer::Attack,
                },
                sfx_path: Some(assert_assets_path!("audio/sfx/player_melee.ogg")),
                effects: AttackEffects {
                    screen_shake: false,
                },
            },
        );
        defs.insert(
            AttackId::Pistol,
            AttackDefinition {
                id: AttackId::Pistol,
                name: "Pistol",
                category: AttackCategory::Ranged,
                damage: 30,
                duration_secs: 0.06,
                collision: AttackCollisionMode::Point,
                // 5-point cross: centre + 4 cardinal neighbours.
                hit_offsets: vec![IVec2::ZERO, IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y],
                spawn_on_expire: None,
                detonates_on_hit: false,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Single,
                aim_spread: 0.0,
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/bullet_particles.px_sprite.png"),
                    frames: 4,
                    speed_ms: 80,
                    finish_behavior: CxAnimationFinishBehavior::Despawn,
                    frame_transition: CxFrameTransition::None,
                    anchor: CxAnchor::Center,
                    canvas: CxRenderSpace::Camera,
                    layer: Layer::Attack,
                },
                sfx_path: Some(assert_assets_path!("audio/sfx/player_shot.ogg")),
                effects: AttackEffects {
                    screen_shake: false,
                },
            },
        );
        defs.insert(
            AttackId::MachineGun,
            AttackDefinition {
                id: AttackId::MachineGun,
                name: "Machine Gun",
                category: AttackCategory::Ranged,
                damage: 20,
                duration_secs: 0.06,
                collision: AttackCollisionMode::Point,
                hit_offsets: vec![],
                spawn_on_expire: None,
                detonates_on_hit: false,
                input_policy: AttackInputPolicy::Hold {
                    warmup_secs: 0.18,
                    interval_secs: 0.08,
                },
                hit_policy: AttackHitPolicy::Single,
                aim_spread: 2.0,
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/bullet_particles.px_sprite.png"),
                    frames: 4,
                    speed_ms: 80,
                    finish_behavior: CxAnimationFinishBehavior::Despawn,
                    frame_transition: CxFrameTransition::None,
                    anchor: CxAnchor::Center,
                    canvas: CxRenderSpace::Camera,
                    layer: Layer::Attack,
                },
                sfx_path: Some(assert_assets_path!("audio/sfx/player_shot.ogg")),
                effects: AttackEffects {
                    screen_shake: false,
                },
            },
        );
        defs.insert(
            AttackId::Bomb,
            AttackDefinition {
                id: AttackId::Bomb,
                name: "Bomb",
                category: AttackCategory::Ranged,
                damage: 0,
                duration_secs: 0.9,
                collision: AttackCollisionMode::SpriteMask,
                hit_offsets: vec![],
                spawn_on_expire: Some(AttackId::BombExplosion),
                detonates_on_hit: true,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Single,
                aim_spread: 0.0,
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/pickups/bomb_6.px_sprite.png"),
                    frames: 1,
                    speed_ms: 200,
                    finish_behavior: CxAnimationFinishBehavior::Loop,
                    frame_transition: CxFrameTransition::None,
                    anchor: CxAnchor::Center,
                    canvas: CxRenderSpace::World,
                    layer: Layer::Attack,
                },
                sfx_path: None,
                effects: AttackEffects { screen_shake: true },
            },
        );
        defs.insert(
            AttackId::BombExplosion,
            AttackDefinition {
                id: AttackId::BombExplosion,
                name: "Bomb Explosion",
                category: AttackCategory::Ranged,
                damage: 60,
                duration_secs: 0.5,
                collision: AttackCollisionMode::Radial { radius: 5.0 },
                hit_offsets: vec![],
                spawn_on_expire: None,
                detonates_on_hit: false,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Repeat {
                    cooldown_secs: 0.25,
                    repeat_damage: 25,
                },
                aim_spread: 0.0,
                // TODO replace with dedicated explosion sprite
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/bullet_particles.px_sprite.png"),
                    frames: 4,
                    speed_ms: 500,
                    finish_behavior: CxAnimationFinishBehavior::Despawn,
                    frame_transition: CxFrameTransition::None,
                    anchor: CxAnchor::Center,
                    canvas: CxRenderSpace::World,
                    layer: Layer::Attack,
                },
                sfx_path: Some(assert_assets_path!("audio/sfx/bomb_explode.ogg")),
                effects: AttackEffects {
                    screen_shake: false,
                },
            },
        );
        Self { defs }
    }
}

#[derive(Clone, Debug)]
pub struct AttackCycle {
    options: Vec<AttackId>,
    index: usize,
}

impl AttackCycle {
    #[must_use]
    pub fn new(options: Vec<AttackId>) -> Self {
        Self { options, index: 0 }
    }

    #[must_use]
    pub fn current(&self) -> AttackId {
        self.options[self.index]
    }

    pub fn cycle(&mut self) -> AttackId {
        if self.options.is_empty() {
            return AttackId::Pincer;
        }
        self.index = (self.index + 1) % self.options.len();
        self.current()
    }
}

#[derive(Resource, Debug)]
pub struct AttackLoadout {
    cycle: AttackCycle,
}

impl AttackLoadout {
    #[must_use]
    pub fn current(&self) -> AttackId {
        self.cycle.current()
    }

    pub fn cycle(&mut self) -> AttackId {
        self.cycle.cycle()
    }
}

impl Default for AttackLoadout {
    fn default() -> Self {
        Self {
            cycle: AttackCycle::new(vec![AttackId::Pistol, AttackId::Bomb]),
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct AttackInputState {
    pub armed: bool,
    pub pressed_at: f32,
    pub pressed_world_position: Option<Vec2>,
    pub last_hold_fire_at: Option<f32>,
    pub hold_fired: bool,
}

impl AttackInputState {
    pub fn arm(&mut self, now: f32, world_position: Option<Vec2>) {
        self.armed = true;
        self.pressed_at = now;
        self.pressed_world_position = world_position;
        self.last_hold_fire_at = None;
        self.hold_fired = false;
    }

    pub fn clear(&mut self) {
        self.armed = false;
        self.last_hold_fire_at = None;
        self.hold_fired = false;
        self.pressed_at = 0.0;
        self.pressed_world_position = None;
    }

    pub fn mark_hold_fired(&mut self, now: f32) {
        self.hold_fired = true;
        self.last_hold_fire_at = Some(now);
    }
}

#[derive(Component, Debug)]
pub struct AttackLifetime {
    pub timer: Timer,
}

impl AttackLifetime {
    #[must_use]
    pub fn new(duration_secs: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct HitRecord {
    cooldown: f32,
    has_hit: bool,
}

#[derive(Component, Default, Debug)]
pub struct AttackHitTracker {
    records: HashMap<Entity, HitRecord>,
}

impl AttackHitTracker {
    pub fn tick(&mut self, delta_secs: f32) {
        for record in self.records.values_mut() {
            if record.cooldown > 0.0 {
                record.cooldown = (record.cooldown - delta_secs).max(0.0);
            }
        }
    }

    #[must_use]
    pub fn can_hit(&self, entity: Entity, policy: AttackHitPolicy) -> bool {
        match policy {
            AttackHitPolicy::Single => !self.records.contains_key(&entity),
            AttackHitPolicy::Repeat { .. } => self
                .records
                .get(&entity)
                .is_none_or(|record| record.cooldown <= 0.0),
        }
    }

    #[must_use]
    pub fn has_hit(&self, entity: Entity) -> bool {
        self.records
            .get(&entity)
            .is_some_and(|record| record.has_hit)
    }

    pub fn register_hit(&mut self, entity: Entity, policy: AttackHitPolicy) {
        match policy {
            AttackHitPolicy::Single => {
                self.records.insert(
                    entity,
                    HitRecord {
                        cooldown: f32::INFINITY,
                        has_hit: true,
                    },
                );
            }
            AttackHitPolicy::Repeat { cooldown_secs, .. } => {
                self.records.insert(
                    entity,
                    HitRecord {
                        cooldown: cooldown_secs,
                        has_hit: true,
                    },
                );
            }
        }
    }

    pub fn inherit_hit(&mut self, from: Entity, to: Entity) {
        if let Some(record) = self.records.get(&from).copied() {
            self.records.insert(to, record);
        }
    }
}

#[derive(Component, Default, Debug)]
pub struct AttackEffectState {
    pub screen_shake_triggered: bool,
    pub follow_up_spawned: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_cycle_wraps() {
        let mut cycle = AttackCycle::new(vec![AttackId::Pistol, AttackId::Bomb]);

        assert_eq!(cycle.current(), AttackId::Pistol);
        assert_eq!(cycle.cycle(), AttackId::Bomb);
        assert_eq!(cycle.cycle(), AttackId::Pistol);
    }

    #[test]
    fn hit_tracker_single_hits_once() {
        let mut tracker = AttackHitTracker::default();
        let target = Entity::from_bits(1);

        assert!(tracker.can_hit(target, AttackHitPolicy::Single));
        tracker.register_hit(target, AttackHitPolicy::Single);
        assert!(!tracker.can_hit(target, AttackHitPolicy::Single));
    }

    #[test]
    fn hit_tracker_repeat_respects_cooldown() {
        let mut tracker = AttackHitTracker::default();
        let target = Entity::from_bits(2);
        let policy = AttackHitPolicy::Repeat {
            cooldown_secs: 0.5,
            repeat_damage: 1,
        };

        assert!(tracker.can_hit(target, policy));
        tracker.register_hit(target, policy);
        assert!(!tracker.can_hit(target, policy));
        tracker.tick(0.49);
        assert!(!tracker.can_hit(target, policy));
        tracker.tick(0.02);
        assert!(tracker.can_hit(target, policy));
    }

    #[test]
    fn pistol_has_5_point_cross_hit_offsets() {
        let defs = AttackDefinitions::default();
        let pistol = defs.get(AttackId::Pistol);
        assert_eq!(
            pistol.hit_offsets.len(),
            5,
            "pistol should have 5 hit points"
        );
        assert!(
            pistol.hit_offsets.contains(&IVec2::ZERO),
            "centre point must be included"
        );
        assert!(pistol.hit_offsets.contains(&IVec2::X), "right");
        assert!(pistol.hit_offsets.contains(&IVec2::NEG_X), "left");
        assert!(pistol.hit_offsets.contains(&IVec2::Y), "up");
        assert!(pistol.hit_offsets.contains(&IVec2::NEG_Y), "down");
    }

    #[test]
    fn non_pistol_attacks_have_empty_offsets() {
        let defs = AttackDefinitions::default();
        for &id in &[
            AttackId::Pincer,
            AttackId::MachineGun,
            AttackId::Bomb,
            AttackId::BombExplosion,
        ] {
            let def = defs.get(id);
            assert!(
                def.hit_offsets.is_empty(),
                "{id:?} should have empty hit_offsets",
            );
        }
    }

    #[test]
    fn pistol_cross_offsets_are_adjacent_pixels() {
        let defs = AttackDefinitions::default();
        let pistol = defs.get(AttackId::Pistol);
        for offset in &pistol.hit_offsets {
            let manhattan = offset.x.abs() + offset.y.abs();
            assert!(
                manhattan <= 1,
                "offset {offset:?} should be within 1 Manhattan distance"
            );
        }
    }
}
