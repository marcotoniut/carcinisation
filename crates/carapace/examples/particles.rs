#![allow(clippy::needless_pass_by_value)]
// In this program, a particle emitter is spawned

use std::time::Duration;

use bevy::{ecs::system::EntityCommands, prelude::*};
use carapace::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: UVec2::splat(512).into(),
                    ..default()
                }),
                ..default()
            }),
            CxPlugin::<Layer>::new(UVec2::splat(32), "palette/palette_1.palette.png"),
            CxAnimationPlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .run();
}

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    // Spawn a particle emitter
    commands
        .spawn((
            CxEmitter {
                sprites: vec![
                    assets.load("sprite/snow_1.px_sprite.png"),
                    assets.load("sprite/snow_2.px_sprite.png"),
                ],
                // Range where the particles can spawn
                range: IRect::new(-4, 36, 36, 36),
                // Range of how often the particles spawn
                frequency: CxEmitterFrequency::new(
                    Duration::from_millis(800),
                    Duration::from_millis(1500),
                ),
                // `CxEmitterSimulation::Simulate` spawns particles
                // as soon as the `CxEmitter` is spawned, with values as if they had been spawned
                // earlier. This is useful when an emitter comes into view,
                // and you want it to look like it had been emitting particles all along.
                simulation: CxEmitterSimulation::Simulate,
                // This function is run on each particle that spawns. It is run
                // after all of the other components are added, so you can use this to override components.
                on_spawn: Box::new(|particle: &mut EntityCommands| {
                    // Let's make each particle animated
                    particle.insert(CxAnimation {
                        on_finish: CxAnimationFinishBehavior::Loop,
                        ..default()
                    });
                }),
            },
            // Particle lifetime
            ParticleLifetime(Duration::from_secs(30)),
            // Particle starting velocity
            CxVelocity(Vec2::new(0., -2.5)),
        ))
        .log_components();
}

#[px_layer]
struct Layer;
