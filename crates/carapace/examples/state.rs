#![allow(clippy::needless_pass_by_value)]
// In this game, you can press space to cast a spell.
// This example uses local ECS components plus CxAnimationFinished as a small state machine.

use bevy::prelude::*;
use carapace::prelude::*;
use leafwing_input_manager::prelude::*;

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
            CxPlugin::<Layer>::new(UVec2::splat(16), "palette/palette_1.palette.png"),
            CxAnimationPlugin,
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
    idle: Handle<CxSpriteAsset>,
    cast: Handle<CxSpriteAsset>,
}

type CastFinishQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static mut CxSprite), (With<Cast>, With<CxAnimationFinished>)>;

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let sprites = Sprites {
        idle: assets.load("sprite/mage.px_sprite.png"),
        cast: assets.load("sprite/mage_cast.px_sprite.png"),
    };

    commands.spawn((
        CxSprite(sprites.idle.clone()),
        CxPosition(IVec2::splat(8)),
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
            CxSprite(sprites.cast.clone()),
            CxAnimation {
                duration: CxAnimationDuration::millis_per_animation(2000),
                on_finish: CxAnimationFinishBehavior::Done,
                ..default()
            },
        ));
        commands.entity(entity).remove::<Idle>();
    }
}

fn finish_cast(sprites: Res<Sprites>, mut commands: Commands, mut query: CastFinishQuery) {
    for (entity, mut sprite) in &mut query {
        *sprite = CxSprite(sprites.idle.clone());
        commands
            .entity(entity)
            .remove::<(Cast, CxAnimation, CxAnimationFinished)>()
            .insert(Idle);
    }
}

#[derive(Actionlike, Clone, Eq, Hash, PartialEq, Reflect, Debug)]
enum Action {
    Cast,
}

#[px_layer]
struct Layer;
