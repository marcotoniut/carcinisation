use crate::layer::Layer;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxAnimationFinishBehavior, PxCanvas, PxFrameTransition};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum AttackId {
    Pincer,
    Gun,
    Bomb,
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

#[derive(Clone, Copy, Debug)]
pub enum AttackCategory {
    Melee,
    Ranged,
}

#[derive(Clone, Copy, Debug)]
pub enum AttackCollisionMode {
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
    pub input_policy: AttackInputPolicy,
    pub hit_policy: AttackHitPolicy,
    pub sprite: AttackSpriteDefinition,
    pub sfx_path: &'static str,
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
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Repeat {
                    cooldown_secs: 0.18,
                    repeat_damage: 35,
                },
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
                sfx_path: assert_assets_path!("audio/sfx/player_melee.ogg"),
                effects: AttackEffects {
                    screen_shake: false,
                },
            },
        );
        defs.insert(
            AttackId::Gun,
            AttackDefinition {
                id: AttackId::Gun,
                name: "Gun",
                category: AttackCategory::Ranged,
                damage: 30,
                duration_secs: 0.08,
                collision: AttackCollisionMode::Point,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Single,
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
                sfx_path: assert_assets_path!("audio/sfx/player_shot.ogg"),
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
                damage: 60,
                duration_secs: 0.9,
                collision: AttackCollisionMode::SpriteMask,
                input_policy: AttackInputPolicy::Release,
                hit_policy: AttackHitPolicy::Repeat {
                    cooldown_secs: 0.25,
                    repeat_damage: 25,
                },
                sprite: AttackSpriteDefinition {
                    sprite_path: assert_assets_path!("sprites/pickups/bomb_6.px_sprite.png"),
                    frames: 1,
                    speed_ms: 200,
                    finish_behavior: PxAnimationFinishBehavior::Despawn,
                    frame_transition: PxFrameTransition::None,
                    anchor: PxAnchor::Center,
                    canvas: PxCanvas::Camera,
                    layer: Layer::Attack,
                },
                sfx_path: assert_assets_path!("audio/sfx/bomb_explode.ogg"),
                effects: AttackEffects { screen_shake: true },
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
            b: AttackCycle::new(vec![AttackId::Gun]),
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct AttackInputState {
    pub active_button: Option<AttackButton>,
    pub pressed_at: f32,
    pub last_hold_fire_at: Option<f32>,
    pub hold_fired: bool,
}

impl AttackInputState {
    pub fn arm(&mut self, button: AttackButton, now: f32) {
        self.active_button = Some(button);
        self.pressed_at = now;
        self.last_hold_fire_at = None;
        self.hold_fired = false;
    }

    pub fn clear(&mut self) {
        self.active_button = None;
        self.last_hold_fire_at = None;
        self.hold_fired = false;
        self.pressed_at = 0.0;
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
                .map_or(true, |record| record.cooldown <= 0.0),
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
}
