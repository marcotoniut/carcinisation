use bevy::{prelude::*, window::PrimaryWindow};
use leafwing_input_manager::{
    prelude::{ActionState, InputMap},
    InputManagerBundle,
};

use crate::{events::*, AppState, GBInput, globals::CROSSHAIR_SPEED};

use self::crosshair::{CrosshairBundle, Crosshair, CrosshairSettings};

pub mod crosshair;

fn spawn_default(
    mut commands: Commands,
    window_ref: &Window,
    asset_server: Res<AssetServer>
){

    //let position = Vec3::new(window_ref.width() / 2.0, window_ref.height() / 2.0, 0.0);
    let position = Vec3::new(0.0, 0.0, 10.0);

    let crosshair = commands.spawn((
            SpriteBundle {
                transform: Transform::from_xyz(position.x, position.y, position.z),
                texture: asset_server.load("sprites/crosshairs/default.png"),
                ..default()
            },
            CrosshairBundle {
                crosshair: Crosshair {
                    name: "Default".to_string(),
                    pos: position
                }
            }
        )
    ).id();

    info!("spawned default crosshair: {}", crosshair.index() )
}

fn spawn_tshape(
    mut commands: Commands,
    window_ref: &Window,
    asset_server: Res<AssetServer>
){

    let position = Vec3::new(window_ref.width() / 2.0, window_ref.height() / 2.0, 0.0);

    let crosshair = commands.spawn((
            SpriteBundle {
                transform: Transform::from_xyz(position.x, position.y, position.z),
                texture: asset_server.load("sprites/crosshairs/t.png"),
                ..default()
            },
            CrosshairBundle {
                crosshair: Crosshair {
                    name: "T-shape".to_string(),
                    pos: position
                }
            }
        )
    ).id();

    info!("spawned default crosshair: {}", crosshair.index() )
}

pub fn render_crosshair(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    crosshair_settings: Res<CrosshairSettings>,
){
    let window = window_query.get_single().unwrap();

    match crosshair_settings.0 {
        0=> spawn_default(commands, window, asset_server),
        1=> spawn_tshape(commands, window, asset_server),
        _=> spawn_default(commands, window, asset_server)
    }
}

pub fn move_crosshair(
    gb_input_query: Query<&ActionState<GBInput>>,
    mut crosshair_transform: Query<&mut Transform, With<Crosshair>>,
    mut crosshair_obj: Query<&mut Crosshair>,
    time: Res<Time>
) {
    if let Ok(mut crosshair) = crosshair_obj.get_single_mut() {
        if let Ok(mut transform) = crosshair_transform.get_single_mut() {
            let mut dir = Vec3::ZERO;
            let gb_input = gb_input_query.single();

            if gb_input.pressed(GBInput::Up) {
                dir += Vec3::new(0.0, 1.0, 0.0);
            }
            if gb_input.pressed(GBInput::Down) {
                dir += Vec3::new(0.0, -1.0, 0.0);
            }
            if gb_input.pressed(GBInput::Left) {
                dir += Vec3::new(-1.0, 0.0, 0.0);
            }
            if gb_input.pressed(GBInput::Right) {
                dir += Vec3::new(1.0, 0.0, 0.0);
            }

            
            // if gb_input.pressed(GBInput::A) {
            //     dir += Vec3::new(0.0, 0.0, -1.0);
            // }
            // if gb_input.pressed(GBInput::B) {
            //     dir += Vec3::new(0.0, 0.0, 1.0);
            // }

            crosshair.pos += dir * CROSSHAIR_SPEED * time.delta_seconds();
            //info!("crosshair pos {},{},{}", crosshair.pos.x, crosshair.pos.y, crosshair.pos.z);

            transform.translation = crosshair.pos;
        }
    }
}