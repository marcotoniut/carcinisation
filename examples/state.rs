#![allow(clippy::needless_pass_by_value)]
// In this game, you can press space to cast a spell.
// This example uses local ECS components plus PxAnimationFinished as a small state machine.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
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
            InputManagerPlugin::<Action>::default(),
            PxPlugin::<Layer>::new(UVec2::splat(16), "palette/palette_1.palette.png"),
            PxAnimationPlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .add_systems(Update, begin_cast)
        .add_systems(PostUpdate, finish_cast)
        .run();
}

#[derive(Clone, Component)]
#[component(storage = "SparseSet")]
struct Idle;

#[derive(Clone, Component)]
#[component(storage = "SparseSet")]
struct Cast;

#[derive(Resource, Clone)]
struct Sprites {
    idle: Handle<PxSpriteAsset>,
    cast: Handle<PxSpriteAsset>,
}

type CastFinishQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static mut PxSprite), (With<Cast>, With<PxAnimationFinished>)>;

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let sprites = Sprites {
        idle: assets.load("sprite/mage.px_sprite.png"),
        cast: assets.load("sprite/mage_cast.px_sprite.png"),
    };

    commands.spawn((
        PxSprite(sprites.idle.clone()),
        PxPosition(IVec2::splat(8)),
        InputMap::default().with(Action::Cast, KeyCode::Space),
        Idle,
    ));

    commands.insert_resource(sprites);
}

fn begin_cast(
    sprites: Res<Sprites>,
    mut commands: Commands,
    query: Query<(Entity, &ActionState<Action>), With<Idle>>,
) {
    for (entity, action_state) in &query {
        if !action_state.just_pressed(&Action::Cast) {
            continue;
        }

        commands.entity(entity).insert((
            Cast,
            PxSprite(sprites.cast.clone()),
            PxAnimation {
                duration: PxAnimationDuration::millis_per_animation(2000),
                on_finish: PxAnimationFinishBehavior::Done,
                ..default()
            },
        ));
        commands.entity(entity).remove::<Idle>();
    }
}

fn finish_cast(sprites: Res<Sprites>, mut commands: Commands, mut query: CastFinishQuery) {
    for (entity, mut sprite) in &mut query {
        *sprite = PxSprite(sprites.idle.clone());
        commands
            .entity(entity)
            .remove::<(Cast, PxAnimation, PxAnimationFinished)>()
            .insert(Idle);
    }
}

#[derive(Actionlike, Clone, Eq, Hash, PartialEq, Reflect, Debug)]
enum Action {
    Cast,
}

#[px_layer]
struct Layer;
