#![allow(clippy::needless_pass_by_value)]
// In this game, you can move the camera with the arrow keys, and switch the mage's render space
// by pressing space

use bevy::prelude::*;
use carapace::prelude::*;
use rand::{RngExt, rng};

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
            CxPlugin::<Layer>::new(UVec2::splat(64), "palette/palette_1.palette.png"),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, init)
        .add_systems(Update, (move_mage, move_camera, switch_render_space))
        .run();
}

const OK: Result = Ok(());

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    // `WorldPos` contains a `Vec2` — the authoritative world-space position.
    // `CxPosition` (integer cache) is derived by rounding each frame.
    commands.spawn((WorldPos::default(), CameraPos));

    // By default, the mage is on the World render space, which means you see it in different positions
    // based on where the camera is
    commands.spawn((
        CxSprite(assets.load("sprite/mage.px_sprite.png")),
        CxPosition(IVec2::splat(32)),
        Mage,
    ));
}

#[derive(Component)]
struct CameraPos;

const CAMERA_SPEED: f32 = 10.;

// Move the camera based on the arrow keys
fn move_camera(
    mut camera_poses: Query<&mut WorldPos, With<CameraPos>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera: ResMut<CxCamera>,
) -> Result {
    let mut camera_pos = camera_poses.single_mut()?;
    **camera_pos += IVec2::new(
        i32::from(keys.pressed(KeyCode::ArrowRight)) - i32::from(keys.pressed(KeyCode::ArrowLeft)),
        i32::from(keys.pressed(KeyCode::ArrowUp)) - i32::from(keys.pressed(KeyCode::ArrowDown)),
    )
    .as_vec2()
    .normalize_or_zero()
        * time.delta_secs()
        * CAMERA_SPEED;

    **camera = camera_pos.round().as_ivec2();

    OK
}

#[derive(Component)]
struct Mage;

// Jitter the mage around randomly. This function is framerate-sensitive, which is not good
// for a game, but it's fine for this example.
fn move_mage(mut mages: Query<&mut CxPosition, With<Mage>>) -> Result {
    if let Some(delta) = [IVec2::X, -IVec2::X, IVec2::Y, -IVec2::Y].get(rng().random_range(0..50)) {
        **mages.single_mut()? += *delta;
    }

    OK
}

// Toggle render space when you press space
fn switch_render_space(
    mut mages: Query<&mut CxRenderSpace>,
    keys: Res<ButtonInput<KeyCode>>,
) -> Result {
    if keys.just_pressed(KeyCode::Space) {
        let mut space = mages.single_mut()?;

        *space = match *space {
            // Camera = drawn at a fixed screen position (like UI)
            CxRenderSpace::World => CxRenderSpace::Camera,
            // World = drawn relative to world origin (like terrain)
            CxRenderSpace::Camera => CxRenderSpace::World,
        };
    }

    OK
}

#[px_layer]
struct Layer;
