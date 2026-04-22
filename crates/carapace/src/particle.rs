// TODO Remove
//! Particles and particle emitters

use std::{
    fmt::{Debug, Formatter, Result},
    time::Duration,
};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::EntityCommands;
use bevy_platform::time::Instant;

use crate::{
    position::{CxLayer, DefaultLayer},
    prelude::*,
    set::CxSet,
};

// https://github.com/bevyengine/bevy/issues/8483
// In wasm, time starts at 0, so it needs an offset to represent an instant before the app started.
// If a day isn't sufficient for your use case, file an issue!
const TIME_OFFSET: Duration = Duration::from_secs(60 * 60 * 24);

pub(crate) fn plug<L: CxLayer>(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            (
                validate_emitters,
                ApplyDeferred,
                (simulate_emitters::<L>, insert_emitter_time),
                (ApplyDeferred, update_emitters::<L>)
                    .chain()
                    .in_set(CxSet::UpdateEmitters),
            )
                .chain(),
            despawn_particles,
        ),
    );
}

fn validate_emitters(
    mut commands: Commands,
    emitters: Query<(Entity, &CxEmitter), Or<(Added<CxEmitter>, Changed<CxEmitter>)>>,
) {
    for (entity, emitter) in &emitters {
        if emitter.sprites.is_empty() {
            error!(
                "`CxEmitter` on entity {entity:?} has no sprites; removing invalid `CxEmitter` component"
            );
            commands.entity(entity).remove::<CxEmitter>();
        }
    }
}

/// A particle's lifetime
#[derive(Clone, Component, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct ParticleLifetime(pub Duration);

impl Default for ParticleLifetime {
    fn default() -> Self {
        Self(Duration::from_secs(1))
    }
}

impl From<Duration> for ParticleLifetime {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

/// Spawn frequency range for an emitter
#[derive(Debug)]
pub struct CxEmitterFrequency {
    min: Duration,
    max: Duration,
    next: Option<Duration>,
}

impl Default for CxEmitterFrequency {
    fn default() -> Self {
        Self::single(Duration::from_secs(1))
    }
}

impl CxEmitterFrequency {
    /// Create a new [`CxEmitterFrequency`] with frequency bounds
    #[must_use]
    pub fn new(min: Duration, max: Duration) -> Self {
        Self {
            min,
            max,
            next: None,
        }
    }

    /// Create a [`CxEmitterFrequency`] with a certain frequency
    #[must_use]
    pub fn single(duration: Duration) -> Self {
        Self {
            min: duration,
            max: duration,
            next: None,
        }
    }

    fn next(&mut self, rng: &mut Rng) -> Duration {
        if let Some(duration) = self.next {
            duration
        } else {
            let duration = self.max.saturating_sub(self.min).mul_f32(rng.f32()) + self.min;
            self.next = Some(duration);
            duration
        }
    }

    fn update_next(&mut self, rng: &mut Rng) -> Duration {
        let duration = self.next(rng);
        self.next = None;
        self.next(rng);
        duration
    }
}

/// Determines whether the emitter is pre-simulated
#[derive(Debug, Default, Eq, PartialEq)]
pub enum CxEmitterSimulation {
    /// The emitter is not pre-simulated
    #[default]
    None,
    /// The emitter is pre-simulated. This means that the emitter will spawn particles
    /// as soon as the `CxEmitter` is spawned, with values as if they had been spawned
    /// earlier. This is useful when an emitter comes into view,
    /// and you want it to look like it had been emitting particles all along.
    Simulate,
}

/// Creates a particle emitter
#[derive(Component)]
#[require(CxAnchor, DefaultLayer, CxRenderSpace, ParticleLifetime, CxVelocity)]
pub struct CxEmitter {
    /// Possible sprites for an emitter's particles
    pub sprites: Vec<Handle<CxSpriteAsset>>,
    /// Location range for an emitter's particles
    pub range: IRect,
    /// A [`CxEmitterFrequency`]
    pub frequency: CxEmitterFrequency,
    /// A [`CxEmitterSimulation`]
    pub simulation: CxEmitterSimulation,
    /// This function is run on each particle that spawns. It is run
    /// after all of the other components are added, so you can use this to override components.
    pub on_spawn: Box<dyn Fn(&mut EntityCommands) + Send + Sync>,
}

impl Default for CxEmitter {
    fn default() -> Self {
        Self {
            sprites: Vec::new(),
            range: default(),
            frequency: default(),
            simulation: default(),
            on_spawn: Box::new(|_| ()),
        }
    }
}

impl Debug for CxEmitter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("CxEmitter")
            .field("sprites", &self.sprites)
            .field("range", &self.range)
            .field("frequency", &self.frequency)
            .field("simulation", &self.simulation)
            .field("on_spawn", &())
            .finish()
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
struct CxEmitterStart(Instant);

#[derive(Component, Debug, Deref, DerefMut)]
struct CxParticleStart(Instant);

impl Default for CxParticleStart {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl From<Instant> for CxParticleStart {
    fn from(duration: Instant) -> Self {
        Self(duration)
    }
}

#[allow(dead_code)]
#[derive(Bundle, Default)]
struct CxParticleBundle {
    position: WorldPos,
    velocity: CxVelocity,
    start: CxParticleStart,
    lifetime: ParticleLifetime,
}

fn simulate_emitters<L: CxLayer>(
    mut commands: Commands,
    emitters: Query<
        (
            &CxEmitter,
            &CxAnchor,
            &L,
            &CxRenderSpace,
            &ParticleLifetime,
            &CxVelocity,
        ),
        Added<CxEmitter>,
    >,
    time: Res<Time<Real>>,
    mut rng: ResMut<GlobalRng>,
) {
    for (emitter, anchor, layer, canvas, lifetime, velocity) in &emitters {
        if emitter.simulation != CxEmitterSimulation::Simulate {
            continue;
        }

        let current_time = time.last_update().unwrap_or_else(|| time.startup()) + TIME_OFFSET;
        let mut simulated_time = current_time;

        while simulated_time + **lifetime >= current_time {
            let position = IVec2::new(
                rng.i32(emitter.range.min.x..=emitter.range.max.x),
                rng.i32(emitter.range.min.y..=emitter.range.max.y),
            )
            .as_vec2()
                + **velocity * (current_time - simulated_time).as_secs_f32();

            (emitter.on_spawn)(&mut commands.spawn((
                CxSprite(rng.sample(&emitter.sprites).unwrap().clone()),
                CxPosition::from(IVec2::new(
                    position.x.round() as i32,
                    position.y.round() as i32,
                )),
                *anchor,
                layer.clone(),
                *canvas,
                WorldPos::from(position),
                *velocity,
                CxParticleStart::from(simulated_time),
                *lifetime,
                Name::new("Particle"),
            )));

            // In wasm, the beginning of time is the start of the program, so we `checked_sub`
            let Some(new_time) = simulated_time.checked_sub(
                emitter
                    .frequency
                    .max
                    .saturating_sub(emitter.frequency.min)
                    .mul_f32(rng.f32())
                    + emitter.frequency.min,
            ) else {
                break;
            };
            simulated_time = new_time;
        }
    }
}

fn insert_emitter_time(
    mut commands: Commands,
    emitters: Query<Entity, Added<CxEmitter>>,
    time: Res<Time<Real>>,
    mut rng: ResMut<GlobalRng>,
) {
    for emitter in &emitters {
        commands.entity(emitter).insert((
            CxEmitterStart(time.last_update().unwrap_or_else(|| time.startup()) + TIME_OFFSET),
            RngComponent::from(&mut rng),
        ));
    }
}

fn update_emitters<L: CxLayer>(
    mut commands: Commands,
    mut emitters: Query<(
        &mut CxEmitter,
        &CxAnchor,
        &L,
        &CxRenderSpace,
        &ParticleLifetime,
        &CxVelocity,
        &mut CxEmitterStart,
        &mut RngComponent,
    )>,
    time: Res<Time<Real>>,
) {
    for (mut emitter, anchor, layer, canvas, lifetime, velocity, mut start, mut rng) in
        &mut emitters
    {
        if time.last_update().unwrap_or_else(|| time.startup()) + TIME_OFFSET - **start
            < emitter.frequency.next(rng.get_mut())
        {
            continue;
        }

        **start += emitter.frequency.update_next(rng.get_mut());
        let position = IVec2::new(
            rng.i32(emitter.range.min.x..=emitter.range.max.x),
            rng.i32(emitter.range.min.y..=emitter.range.max.y),
        );

        (emitter.on_spawn)(&mut commands.spawn((
            CxSprite(rng.sample(&emitter.sprites).unwrap().clone()),
            CxPosition::from(position),
            *anchor,
            layer.clone(),
            *canvas,
            WorldPos::from(position.as_vec2()),
            *velocity,
            CxParticleStart::from(
                time.last_update().unwrap_or_else(|| time.startup()) + TIME_OFFSET,
            ),
            *lifetime,
            Name::new("Particle"),
        )));
    }
}

fn despawn_particles(
    mut commands: Commands,
    particles: Query<(Entity, &ParticleLifetime, &CxParticleStart)>,
    time: Res<Time<Real>>,
) {
    for (particle, lifetime, start) in &particles {
        if time.last_update().unwrap_or_else(|| time.startup()) + TIME_OFFSET - **start
            >= **lifetime
        {
            commands.entity(particle).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::schedule::Schedule;

    #[cfg_attr(
        feature = "headed",
        derive(bevy_render::extract_component::ExtractComponent)
    )]
    #[derive(Component, next::Next, Ord, PartialOrd, Eq, PartialEq, Clone, Default, Debug)]
    #[next(path = next::Next)]
    enum TestLayer {
        #[default]
        Test,
    }

    fn test_world() -> World {
        let mut world = World::new();
        // `CxEmitter` requires `DefaultLayer`; tests set layer explicitly.
        world.insert_resource(crate::position::InsertDefaultLayer::noop());
        world.init_resource::<Time<Real>>();
        world.init_resource::<GlobalRng>();
        world
    }

    fn run_emitter_step(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                validate_emitters,
                ApplyDeferred,
                simulate_emitters::<TestLayer>,
                insert_emitter_time,
                ApplyDeferred,
                update_emitters::<TestLayer>,
            )
                .chain(),
        );
        schedule.run(world);
    }

    #[test]
    fn invalid_emitter_with_empty_sprites_produces_no_particles() {
        let mut world = test_world();

        let emitter = world.spawn(CxEmitter::default()).id();
        world.entity_mut(emitter).insert(TestLayer::default());

        run_emitter_step(&mut world);

        let particles = world.query::<&CxParticleStart>().iter(&world).count();
        assert_eq!(particles, 0);
    }

    #[test]
    fn valid_emitter_can_spawn_particles() {
        let mut world = test_world();

        let emitter = world
            .spawn(CxEmitter {
                sprites: vec![default()],
                frequency: CxEmitterFrequency::single(Duration::ZERO),
                simulation: CxEmitterSimulation::None,
                ..default()
            })
            .id();
        world.entity_mut(emitter).insert(TestLayer::default());

        run_emitter_step(&mut world);

        let particles = world.query::<&CxParticleStart>().iter(&world).count();
        assert!(particles >= 1);
    }
}
