use crate::layer::Layer;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxAnimationFinishBehavior, PxCanvas, PxFrameTransition};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum AttackId {
    Pincer,
    Pistol,
    MachineGun,
    Bomb,
    BombExplosion,
}

#[derive(Clone, Copy, Debug)]
pub enum AttackButton {
    A,
    B,
}

impl AttackButton {
    pub fn gb_input(self) -> crate::input::GBInput {
        match self {
            AttackButton::A => crate::input::GBInput::A,
            AttackButton::B => crate::input::GBInput::B,
        }
    }
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
    pub finish_behavior: PxAnimationFinishBehavior,
    pub frame_transition: PxFrameTransition,
    pub anchor: PxAnchor,
    pub canvas: PxCanvas,
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
    pub fn get(&self, id: AttackId) -> &AttackDefinition {
        self.defs
            .get(&id)
            .unwrap_or_else(|| panic!("missing attack definition for {id:?}"))
    }
}

impl Default for AttackDefinitions {
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
                    finish_behavior: PxAnimationFinishBehavior::Despawn,
                    frame_transition: PxFrameTransition::None,
                    anchor: PxAnchor::Center,
                    canvas: PxCanvas::Camera,
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
                spawn_on_expire: None,
                detonates_on_hit: false,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Single,
                aim_spread: 0.0,
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/bullet_particles.px_sprite.png"),
                    frames: 4,
                    speed_ms: 80,
                    finish_behavior: PxAnimationFinishBehavior::Despawn,
                    frame_transition: PxFrameTransition::None,
                    anchor: PxAnchor::Center,
                    canvas: PxCanvas::Camera,
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
                    finish_behavior: PxAnimationFinishBehavior::Despawn,
                    frame_transition: PxFrameTransition::None,
                    anchor: PxAnchor::Center,
                    canvas: PxCanvas::Camera,
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
                spawn_on_expire: Some(AttackId::BombExplosion),
                detonates_on_hit: true,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Single,
                aim_spread: 0.0,
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/pickups/bomb_6.px_sprite.png"),
                    frames: 1,
                    speed_ms: 200,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    frame_transition: PxFrameTransition::None,
                    anchor: PxAnchor::Center,
                    canvas: PxCanvas::World,
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
                collision: AttackCollisionMode::SpriteMask,
                spawn_on_expire: None,
                detonates_on_hit: false,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Repeat {
                    cooldown_secs: 0.25,
                    repeat_damage: 25,
                },
                aim_spread: 0.0,
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!(
                        "sprites/attacks/blood_attack_hovering_1.px_sprite.png"
                    ),
                    frames: 4,
                    speed_ms: 500,
                    finish_behavior: PxAnimationFinishBehavior::Despawn,
                    frame_transition: PxFrameTransition::None,
                    anchor: PxAnchor::Center,
                    canvas: PxCanvas::World,
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
    pub fn new(options: Vec<AttackId>) -> Self {
        Self { options, index: 0 }
    }

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
    pub a: AttackCycle,
    pub b: AttackCycle,
}

impl AttackLoadout {
    pub fn current(&self, button: AttackButton) -> AttackId {
        match button {
            AttackButton::A => self.a.current(),
            AttackButton::B => self.b.current(),
        }
    }

    pub fn cycle(&mut self, button: AttackButton) -> AttackId {
        match button {
            AttackButton::A => self.a.cycle(),
            AttackButton::B => self.b.cycle(),
        }
    }
}

impl Default for AttackLoadout {
    fn default() -> Self {
        Self {
            a: AttackCycle::new(vec![AttackId::Pincer, AttackId::Bomb]),
            b: AttackCycle::new(vec![AttackId::Pistol]),
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct AttackInputState {
    pub active_button: Option<AttackButton>,
    pub pressed_at: f32,
    pub pressed_world_position: Option<Vec2>,
    pub last_hold_fire_at: Option<f32>,
    pub hold_fired: bool,
    pub cycled: bool,
}

impl AttackInputState {
    pub fn arm(&mut self, button: AttackButton, now: f32, world_position: Option<Vec2>) {
        self.active_button = Some(button);
        self.pressed_at = now;
        self.pressed_world_position = world_position;
        self.last_hold_fire_at = None;
        self.hold_fired = false;
        self.cycled = false;
    }

    pub fn clear(&mut self) {
        self.active_button = None;
        self.last_hold_fire_at = None;
        self.hold_fired = false;
        self.pressed_at = 0.0;
        self.pressed_world_position = None;
        self.cycled = false;
    }

    pub fn mark_hold_fired(&mut self, now: f32) {
        self.hold_fired = true;
        self.last_hold_fire_at = Some(now);
    }

    pub fn mark_cycled(&mut self) {
        self.cycled = true;
    }
}

#[derive(Component, Debug)]
pub struct AttackLifetime {
    pub timer: Timer,
}

impl AttackLifetime {
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

    pub fn can_hit(&self, entity: Entity, policy: AttackHitPolicy) -> bool {
        match policy {
            AttackHitPolicy::Single => !self.records.contains_key(&entity),
            AttackHitPolicy::Repeat { .. } => self
                .records
                .get(&entity)
                .is_none_or(|record| record.cooldown <= 0.0),
        }
    }

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
        let mut cycle = AttackCycle::new(vec![AttackId::Pincer, AttackId::Bomb]);

        assert_eq!(cycle.current(), AttackId::Pincer);
        assert_eq!(cycle.cycle(), AttackId::Bomb);
        assert_eq!(cycle.cycle(), AttackId::Pincer);
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
}
