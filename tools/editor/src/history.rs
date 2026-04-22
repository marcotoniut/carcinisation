use carcinisation::stage::data::StageSpawn;

use crate::components::StageSpawnRef;

/// A location within `StageData` that a spawn occupies.
#[derive(Clone, Debug, PartialEq)]
pub enum SpawnLocation {
    Static {
        index: usize,
    },
    Step {
        step_index: usize,
        spawn_index: usize,
    },
}

impl SpawnLocation {
    pub fn from_ref(r: &StageSpawnRef) -> Self {
        match *r {
            StageSpawnRef::Static { index } => SpawnLocation::Static { index },
            StageSpawnRef::Step {
                step_index,
                spawn_index,
                ..
            } => SpawnLocation::Step {
                step_index,
                spawn_index,
            },
        }
    }
}

/// Resolve a mutable reference to a spawn in `StageData`.
pub fn resolve_spawn_mut<'a>(
    stage_data: &'a mut carcinisation::stage::data::StageData,
    location: &SpawnLocation,
) -> Option<&'a mut StageSpawn> {
    use carcinisation::stage::data::StageStep;
    match location {
        SpawnLocation::Static { index } => stage_data.spawns.get_mut(*index),
        SpawnLocation::Step {
            step_index,
            spawn_index,
        } => match stage_data.steps.get_mut(*step_index)? {
            StageStep::Tween(step) => step.spawns.get_mut(*spawn_index),
            StageStep::Stop(step) => step.spawns.get_mut(*spawn_index),
            StageStep::Cinematic(_) => None,
        },
    }
}

/// Resolve an immutable reference to a spawn in `StageData`.
pub fn resolve_spawn<'a>(
    stage_data: &'a carcinisation::stage::data::StageData,
    location: &SpawnLocation,
) -> Option<&'a StageSpawn> {
    use carcinisation::stage::data::StageStep;
    match location {
        SpawnLocation::Static { index } => stage_data.spawns.get(*index),
        SpawnLocation::Step {
            step_index,
            spawn_index,
        } => match stage_data.steps.get(*step_index)? {
            StageStep::Tween(step) => step.spawns.get(*spawn_index),
            StageStep::Stop(step) => step.spawns.get(*spawn_index),
            StageStep::Cinematic(_) => None,
        },
    }
}

/// Remove a spawn at the given location, returning the removed spawn.
pub fn remove_spawn(
    stage_data: &mut carcinisation::stage::data::StageData,
    location: &SpawnLocation,
) -> Option<StageSpawn> {
    use carcinisation::stage::data::StageStep;
    match location {
        SpawnLocation::Static { index } => {
            if *index < stage_data.spawns.len() {
                Some(stage_data.spawns.remove(*index))
            } else {
                None
            }
        }
        SpawnLocation::Step {
            step_index,
            spawn_index,
        } => {
            let step = stage_data.steps.get_mut(*step_index)?;
            match step {
                StageStep::Tween(step) => {
                    if *spawn_index < step.spawns.len() {
                        Some(step.spawns.remove(*spawn_index))
                    } else {
                        None
                    }
                }
                StageStep::Stop(step) => {
                    if *spawn_index < step.spawns.len() {
                        Some(step.spawns.remove(*spawn_index))
                    } else {
                        None
                    }
                }
                StageStep::Cinematic(_) => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use carcinisation::stage::{
        components::{TweenStageStep, placement::Depth},
        data::*,
    };

    fn test_stage_data() -> StageData {
        StageData {
            name: "Test".to_string(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: vec![
                StageSpawn::Object(ObjectSpawn {
                    object_type: ObjectType::BenchBig,
                    coordinates: Vec2::new(10.0, 20.0),
                    depth: Depth::Three,
                    authored_depths: None,
                }),
                StageSpawn::Object(ObjectSpawn {
                    object_type: ObjectType::Fibertree,
                    coordinates: Vec2::new(50.0, 60.0),
                    depth: Depth::Two,
                    authored_depths: None,
                }),
            ],
            steps: vec![StageStep::Tween(
                TweenStageStep::base(100.0, 0.0)
                    .add_spawns(vec![StageSpawn::Enemy(EnemySpawn::mosquito_base())]),
            )],
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
            checkpoint: None,
            parallax_attenuation: None,
        }
    }

    #[test]
    fn resolve_spawn_mut_works() {
        let mut stage_data = test_stage_data();
        let loc = SpawnLocation::Static { index: 0 };
        let spawn = resolve_spawn_mut(&mut stage_data, &loc).unwrap();
        spawn.set_coordinates(Vec2::new(999.0, 888.0));
        assert_eq!(
            *stage_data.spawns[0].get_coordinates(),
            Vec2::new(999.0, 888.0)
        );

        let step_loc = SpawnLocation::Step {
            step_index: 0,
            spawn_index: 0,
        };
        let spawn = resolve_spawn_mut(&mut stage_data, &step_loc).unwrap();
        spawn.set_coordinates(Vec2::new(777.0, 666.0));
        if let StageStep::Tween(ref s) = stage_data.steps[0] {
            assert_eq!(*s.spawns[0].get_coordinates(), Vec2::new(777.0, 666.0));
        }
    }

    #[test]
    fn remove_spawn_static() {
        let mut stage_data = test_stage_data();
        assert_eq!(stage_data.spawns.len(), 2);
        let removed = remove_spawn(&mut stage_data, &SpawnLocation::Static { index: 0 });
        assert!(removed.is_some());
        assert_eq!(stage_data.spawns.len(), 1);
    }

    #[test]
    fn remove_spawn_out_of_bounds_is_none() {
        let mut stage_data = test_stage_data();
        let removed = remove_spawn(&mut stage_data, &SpawnLocation::Static { index: 99 });
        assert!(removed.is_none());
        assert_eq!(stage_data.spawns.len(), 2);
    }
}
