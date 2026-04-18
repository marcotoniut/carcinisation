#![allow(clippy::wrong_self_convention)]

//! Serialized stage definitions: spawns, pickups, objects, and scripted steps.

use super::{
    components::{CinematicStageStep, StopStageStep, TweenStageStep, placement::Depth},
    destructible::data::DestructibleSpawn,
    enemy::{data::steps::EnemyStep, entity::EnemyType},
    projection::ProjectionProfile,
};
use crate::{
    globals::{SCREEN_RESOLUTION, SCREEN_RESOLUTION_F32_H},
    transitions::data::TransitionRequest,
};
use bevy::{asset::Asset, prelude::*, reflect::Reflect};
use derive_more::From;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{DurationSecondsWithFrac, serde_as};
use std::{collections::VecDeque, time::Duration};

/// Convenience spawn position centred on the gameplay viewport.
pub static DEFAULT_COORDINATES: std::sync::LazyLock<Vec2> =
    std::sync::LazyLock::new(|| *SCREEN_RESOLUTION_F32_H);

pub const GAME_BASE_SPEED: f32 = 15.0;

/// Common methods for spawn containers that may hold pickups or enemies.
pub trait Contains {
    fn with_contains(&mut self, value: Option<Box<ContainerSpawn>>);
    fn drops(&mut self, value: ContainerSpawn);
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Metadata required to load an animated skybox.
pub struct SkyboxData {
    pub path: String,
    pub frames: usize,
}

// deriving Default for simplicity's sake in defining the stage data
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
/// Static props that can populate the stage background.
pub enum ObjectType {
    BenchBig,
    BenchSmall,
    Fibertree,
    RugparkSign,
}

impl ObjectType {
    /// Returns the sprite base name for this object type
    #[must_use]
    pub fn sprite_base_name(&self) -> &'static str {
        match self {
            ObjectType::BenchBig => "bench_big",
            ObjectType::BenchSmall => "bench_small",
            ObjectType::Fibertree => "fiber_tree",
            ObjectType::RugparkSign => "rugpark_sign",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
/// Pickup categories available for drop spawns.
pub enum PickupType {
    SmallHealthpack,
    BigHealthpack,
    // TODO
    // Weapon,
    // Ammo,
    // Shield,
}

impl PickupType {
    /// Returns the sprite base name for this pickup type
    #[must_use]
    pub fn sprite_base_name(&self) -> &'static str {
        match self {
            PickupType::SmallHealthpack => "health_4",
            PickupType::BigHealthpack => "health_6",
        }
    }
}

#[derive(Clone, Debug, Deserialize, From, Reflect, Serialize)]
/// Container that either spawns a pickup or an enemy payload.
pub enum ContainerSpawn {
    Pickup(PickupDropSpawn),
    Enemy(EnemyDropSpawn),
}

// TODO move pickup data under its own module?
#[serde_as]
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Runtime spawn instruction for a pickup entity.
pub struct PickupSpawn {
    pub pickup_type: PickupType,
    pub coordinates: Vec2,
    #[serde(default)]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub elapsed: Duration,
    pub depth: Depth,
    /// Visible depths with hand-made visuals. When `None`, defaults to just
    /// the spawn's own `depth`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_depths: Option<Vec<Depth>>,
}

impl PickupSpawn {
    /// Builds a Bevy name for debugging.
    #[must_use]
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }
    /// Display string describing the pickup type.
    #[must_use]
    pub fn show_type(&self) -> String {
        format!("Pickup<{:?}>", self.pickup_type)
    }
    #[must_use]
    pub fn with_elapsed_f32(mut self, value: f32) -> Self {
        self.elapsed = Duration::from_secs_f32(value);
        self
    }
    #[must_use]
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }
    #[must_use]
    pub fn with_depth(mut self, value: Depth) -> Self {
        self.depth = value;
        self
    }
    #[must_use]
    pub fn big_healthpack_base() -> Self {
        Self {
            pickup_type: PickupType::BigHealthpack,
            coordinates: Vec2::ZERO,
            elapsed: Duration::ZERO,
            depth: Depth::Six,
            authored_depths: None,
        }
    }
    #[must_use]
    pub fn small_healthpack_base() -> Self {
        Self {
            pickup_type: PickupType::SmallHealthpack,
            coordinates: Vec2::ZERO,
            elapsed: Duration::ZERO,
            depth: Depth::Six,
            authored_depths: None,
        }
    }
}

// TODO move pickup data under its own module?
#[derive(new, Clone, Debug, Deserialize, Reflect, Serialize)]
/// Template describing which pickup type should drop.
pub struct PickupDropSpawn {
    pub pickup_type: PickupType,
}

impl PickupDropSpawn {
    /// Creates a concrete spawn from this template at the provided location/depth.
    #[must_use]
    pub fn from_spawn(&self, coordinates: Vec2, depth: Depth) -> PickupSpawn {
        PickupSpawn {
            pickup_type: self.pickup_type,
            coordinates,
            depth,
            elapsed: Duration::ZERO,
            authored_depths: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Static prop placement within the stage map.
pub struct ObjectSpawn {
    pub object_type: ObjectType,
    pub coordinates: Vec2,
    pub depth: Depth,
    /// Visible depths with hand-made visuals. When `None`, defaults to just
    /// the spawn's own `depth`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_depths: Option<Vec<Depth>>,
}

impl ObjectSpawn {
    /// Builds a Bevy name for debugging.
    #[must_use]
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }
    /// Display string describing the object type.
    #[must_use]
    pub fn show_type(&self) -> String {
        format!("Object<{:?}>", self.object_type)
    }

    #[must_use]
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    #[must_use]
    pub fn with_depth(mut self, value: Depth) -> Self {
        self.depth = value;
        self
    }

    #[must_use]
    pub fn bench_big_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::BenchBig,
            coordinates: Vec2::new(x, y),
            depth: Depth::Six,
            authored_depths: None,
        }
    }

    #[must_use]
    pub fn bench_small_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::BenchSmall,
            coordinates: Vec2::new(x, y),
            depth: Depth::Six,
            authored_depths: None,
        }
    }

    #[must_use]
    pub fn fibertree_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::Fibertree,
            coordinates: Vec2::new(x, y),
            depth: Depth::Two,
            authored_depths: None,
        }
    }

    #[must_use]
    pub fn rugpark_sign_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::RugparkSign,
            coordinates: Vec2::new(x, y),
            depth: Depth::Three,
            authored_depths: None,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Default, Deserialize, Reflect, Serialize)]
pub struct EnemySpawn {
    pub enemy_type: EnemyType,
    #[serde(default)]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub elapsed: Duration,
    #[reflect(ignore)]
    #[serde(default)]
    pub contains: Option<Box<ContainerSpawn>>,
    pub coordinates: Vec2,
    pub speed: f32,
    #[serde(default)]
    pub health: Option<u32>,
    #[serde(default)]
    pub steps: VecDeque<EnemyStep>,
    pub depth: Depth,
    /// Visible depths with hand-made visuals. When `None`, defaults to just
    /// the spawn's own `depth`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_depths: Option<Vec<Depth>>,
}

#[derive(Clone, Debug, Default, Deserialize, Reflect, Serialize)]
pub struct EnemyDropSpawn {
    pub enemy_type: EnemyType,
    #[reflect(ignore)]
    #[serde(default)]
    pub contains: Option<Box<ContainerSpawn>>,
    pub speed: f32,
    #[serde(default)]
    pub health: Option<u32>,
    #[serde(default)]
    pub steps: VecDeque<EnemyStep>,
}

impl EnemyDropSpawn {
    #[must_use]
    pub fn from_spawn(&self, coordinates: Vec2, depth: Depth) -> EnemySpawn {
        EnemySpawn {
            enemy_type: self.enemy_type,
            coordinates,
            depth,
            speed: self.speed,
            health: self.health,
            steps: self.steps.clone(),
            contains: self.contains.clone(),
            elapsed: Duration::ZERO,
            authored_depths: None,
        }
    }
}

impl EnemySpawn {
    #[must_use]
    pub fn with_elapsed_f32(mut self, value: f32) -> Self {
        self.elapsed = Duration::from_secs_f32(value);
        self
    }
    #[must_use]
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }
    #[must_use]
    pub fn with_x(mut self, value: f32) -> Self {
        self.coordinates.x = value;
        self
    }
    #[must_use]
    pub fn with_y(mut self, value: f32) -> Self {
        self.coordinates.y = value;
        self
    }
    #[must_use]
    pub fn with_speed(mut self, value: f32) -> Self {
        self.speed = value;
        self
    }
    #[must_use]
    pub fn with_health(mut self, value: u32) -> Self {
        self.health = Some(value);
        self
    }
    #[must_use]
    pub fn with_steps(mut self, value: VecDeque<EnemyStep>) -> Self {
        self.steps = value;
        self
    }
    #[must_use]
    pub fn with_steps_vec(mut self, value: Vec<EnemyStep>) -> Self {
        self.steps = value.into();
        self
    }
    #[must_use]
    pub fn with_depth(mut self, value: Depth) -> Self {
        self.depth = value;
        self
    }
    #[must_use]
    pub fn with_enemy_type(mut self, value: EnemyType) -> Self {
        self.enemy_type = value;
        self
    }
    /** TODO should I implement these as a trait Contains */
    #[must_use]
    pub fn with_contains(mut self, value: Option<Box<ContainerSpawn>>) -> Self {
        self.contains = value;
        self
    }
    #[must_use]
    pub fn drops(mut self, value: ContainerSpawn) -> Self {
        self.contains = Some(Box::new(value));
        self
    }

    #[must_use]
    pub fn add_step(mut self, value: EnemyStep) -> Self {
        self.steps.extend(vec![value]);
        self
    }

    // Enemies
    #[must_use]
    pub fn tardigrade_base() -> Self {
        Self {
            enemy_type: EnemyType::Tardigrade,
            coordinates: *DEFAULT_COORDINATES,
            depth: Depth::Six,
            elapsed: Duration::ZERO,
            speed: 0.5,
            health: None,
            steps: vec![].into(),
            contains: None,
            authored_depths: None,
        }
    }
    #[must_use]
    pub fn mosquito_base() -> Self {
        Self {
            enemy_type: EnemyType::Mosquito,
            coordinates: *DEFAULT_COORDINATES,
            depth: Depth::Five,
            elapsed: Duration::ZERO,
            speed: 2.0,
            health: None,
            steps: vec![].into(),
            contains: None,
            authored_depths: None,
        }
    }
    #[must_use]
    pub fn mosquiton_base() -> Self {
        Self {
            enemy_type: EnemyType::Mosquiton,
            depth: Depth::Three,
            ..Self::mosquito_base()
        }
    }
    #[must_use]
    pub fn mosquito_variant_circle() -> Self {
        Self::mosquito_base().with_steps_vec(vec![
            EnemyStep::circle_around_base().with_radius(12.).into(),
        ])
    }
    #[must_use]
    pub fn mosquito_variant_approacher() -> Self {
        Self::mosquito_base()
            .with_depth(Depth::Eight)
            .with_x(SCREEN_RESOLUTION.x as f32 + 10.)
            .with_steps_vec(vec![
                EnemyStep::linear_movement_base()
                    .with_direction(-1., -0.1)
                    .with_trayectory(100.)
                    .depth_advance(1)
                    .into(),
                EnemyStep::linear_movement_base()
                    .depth_advance(2)
                    .with_direction(0.3, -0.2)
                    .with_trayectory(80.)
                    .into(),
                EnemyStep::idle_base().into(),
            ])
    }
    #[must_use]
    pub fn mosquito_variant_linear() -> Self {
        Self::mosquito_base()
            .with_x(SCREEN_RESOLUTION.x as f32 + 10.)
            .with_steps_vec(vec![
                EnemyStep::linear_movement_base().into(),
                EnemyStep::linear_movement_base()
                    .opposite_direction()
                    .into(),
            ])
    }
    #[must_use]
    pub fn mosquito_variant_linear_opposite() -> Self {
        Self::mosquito_base().with_x(-10.).with_steps_vec(vec![
            EnemyStep::linear_movement_base()
                .opposite_direction()
                .into(),
            EnemyStep::linear_movement_base().into(),
        ])
    }
    #[must_use]
    pub fn spidey_base(speed_multiplier: f32, coordinates: Vec2) -> Self {
        Self {
            enemy_type: EnemyType::Spidey,
            coordinates,
            depth: Depth::Six,
            elapsed: Duration::ZERO,
            steps: vec![EnemyStep::circle_around_base().opposite_direction().into()].into(),
            speed: speed_multiplier,
            health: None,
            contains: None,
            authored_depths: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, From, Reflect, Serialize)]
pub enum StageSpawn {
    Object(ObjectSpawn),
    Destructible(DestructibleSpawn),
    Pickup(PickupSpawn),
    Enemy(EnemySpawn),
}

impl StageSpawn {
    #[must_use]
    pub fn get_coordinates(&self) -> &Vec2 {
        match self {
            StageSpawn::Destructible(s) => &s.coordinates,
            StageSpawn::Enemy(s) => &s.coordinates,
            StageSpawn::Object(s) => &s.coordinates,
            StageSpawn::Pickup(s) => &s.coordinates,
        }
    }

    #[must_use]
    pub fn get_elapsed(&self) -> Duration {
        match self {
            StageSpawn::Destructible(_) | StageSpawn::Object(_) => Duration::ZERO,
            StageSpawn::Enemy(s) => s.elapsed.div_f32(GAME_BASE_SPEED),
            StageSpawn::Pickup(s) => s.elapsed.div_f32(GAME_BASE_SPEED),
        }
    }

    #[must_use]
    pub fn get_depth(&self) -> Depth {
        #[allow(clippy::match_same_arms)]
        match self {
            StageSpawn::Destructible(DestructibleSpawn { depth, .. }) => *depth,
            StageSpawn::Enemy(EnemySpawn { depth, .. }) => *depth,
            StageSpawn::Object(ObjectSpawn { depth, .. }) => *depth,
            StageSpawn::Pickup(PickupSpawn { depth, .. }) => *depth,
        }
    }

    /// Returns the authored depths for this spawn, or `None` if not specified
    /// (meaning the spawn's own `depth` is the only authored depth).
    #[must_use]
    pub fn get_authored_depths(&self) -> Option<&Vec<Depth>> {
        match self {
            StageSpawn::Destructible(s) => s.authored_depths.as_ref(),
            StageSpawn::Enemy(s) => s.authored_depths.as_ref(),
            StageSpawn::Object(s) => s.authored_depths.as_ref(),
            StageSpawn::Pickup(s) => s.authored_depths.as_ref(),
        }
    }

    pub fn set_coordinates(&mut self, position: Vec2) {
        match self {
            StageSpawn::Destructible(s) => s.coordinates = position,
            StageSpawn::Enemy(s) => s.coordinates = position,
            StageSpawn::Object(s) => s.coordinates = position,
            StageSpawn::Pickup(s) => s.coordinates = position,
        }
    }

    #[must_use]
    pub fn show_type(&self) -> String {
        match self {
            StageSpawn::Destructible(s) => s.show_type(),
            StageSpawn::Enemy(s) => s.enemy_type.show_type(),
            StageSpawn::Object(s) => s.show_type(),
            StageSpawn::Pickup(s) => s.show_type(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StageActionResumeCondition {
    MaxDuration(u32),
    KillAll,
    KillBoss,
}

#[derive(Clone, Debug, Deserialize, From, Reflect, Serialize)]
pub enum StageStep {
    Cinematic(CinematicStageStep),
    Tween(TweenStageStep),
    Stop(StopStageStep),
}

/// Authored mid-stage checkpoint for continue-after-death.
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct StageCheckpoint {
    /// Which step index to resume from on continue.
    pub step_index: usize,
    /// Camera position when restarting from this checkpoint.
    pub start_coordinates: Vec2,
}

#[derive(Asset, Clone, Debug, Deserialize, Reflect, Resource, Serialize)]
pub struct StageData {
    pub name: String,
    pub background_path: String,
    pub music_path: String,
    pub skybox: SkyboxData,
    pub start_coordinates: Vec2,
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
    #[serde(default)]
    pub on_start_transition_o: Option<TransitionRequest>,
    #[serde(default)]
    pub on_end_transition_o: Option<TransitionRequest>,
    /// Optional gravity override for this stage in pixels per second squared.
    /// If not specified, defaults to standard gravity (800.0).
    #[serde(default)]
    pub gravity: Option<f32>,
    /// Default projection profile for this stage.  Individual steps can
    /// override with their own projection.  When `None`, falls back to
    /// [`ProjectionProfile::default()`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection: Option<ProjectionProfile>,
    /// Optional mid-stage checkpoint for continue-after-death.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<StageCheckpoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_data_without_checkpoint_deserializes() {
        let ron = r#"
            #![enable(unwrap_newtypes)]
            #![enable(implicit_some)]
            #![enable(unwrap_variant_newtypes)]
            #![enable(explicit_struct_names)]
            StageData(
                name: "test",
                background_path: "",
                music_path: "",
                skybox: SkyboxData(path: "", frames: 1),
                start_coordinates: Vec2(0.0, 0.0),
                spawns: [],
                steps: [],
            )
        "#;
        let data: StageData = ron::from_str(ron).expect("should parse without checkpoint");
        assert!(data.checkpoint.is_none());
    }

    #[test]
    fn stage_data_with_checkpoint_deserializes() {
        let ron = r#"
            #![enable(unwrap_newtypes)]
            #![enable(implicit_some)]
            #![enable(unwrap_variant_newtypes)]
            #![enable(explicit_struct_names)]
            StageData(
                name: "test",
                background_path: "",
                music_path: "",
                skybox: SkyboxData(path: "", frames: 1),
                start_coordinates: Vec2(0.0, 0.0),
                spawns: [],
                steps: [
                    Stop(max_duration: 5.0, kill_all: false, kill_boss: false, spawns: [], floor_depths: None),
                    Stop(max_duration: 5.0, kill_all: false, kill_boss: false, spawns: [], floor_depths: None),
                ],
                checkpoint: StageCheckpoint(
                    step_index: 1,
                    start_coordinates: Vec2(100.0, 50.0),
                ),
            )
        "#;
        let data: StageData = ron::from_str(ron).expect("should parse with checkpoint");
        let cp = data.checkpoint.expect("checkpoint should be Some");
        assert_eq!(cp.step_index, 1);
        assert_eq!(cp.start_coordinates, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn park_stage_ron_deserializes() {
        let ron_bytes = include_str!("../../../../assets/stages/park.sg.ron");
        let data: StageData =
            ron::from_str(ron_bytes).expect("park.sg.ron should deserialize successfully");
        assert_eq!(data.name, "Park");
        let cp = data.checkpoint.expect("park should have a checkpoint");
        assert_eq!(cp.step_index, 4);
    }
}
