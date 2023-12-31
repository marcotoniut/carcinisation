use crate::{
    globals::{SCREEN_RESOLUTION, VIEWPORT_MULTIPLIER, VIEWPORT_RESOLUTION_OFFSET},
    stage::components::{
        interactive::{ColliderData, ColliderShape},
        placement::*,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub const LINE_EXTENSION: f32 = 1000.;

const SCREEN_X: f32 = SCREEN_RESOLUTION.x as f32;
const SCREEN_Y: f32 = SCREEN_RESOLUTION.y as f32;

pub fn to_viewport_ratio_x(x: f32) -> f32 {
    VIEWPORT_MULTIPLIER * x
}

pub fn to_viewport_ratio_y(y: f32) -> f32 {
    VIEWPORT_MULTIPLIER * y
}

pub fn to_viewport_ratio(v: Vec2) -> Vec2 {
    Vec2::new(to_viewport_ratio_x(v.x), to_viewport_ratio_y(v.y))
}

pub fn to_viewport_coordinate_x(x: f32) -> f32 {
    VIEWPORT_RESOLUTION_OFFSET.x + VIEWPORT_MULTIPLIER * (x - SCREEN_X * 0.5)
}

pub fn to_viewport_coordinate_y(y: f32) -> f32 {
    VIEWPORT_RESOLUTION_OFFSET.y + VIEWPORT_MULTIPLIER * (y - SCREEN_Y * 0.5)
}

pub fn to_viewport_coordinates(position: Vec2) -> Vec2 {
    Vec2::new(
        to_viewport_coordinate_x(position.x),
        to_viewport_coordinate_y(position.y),
    )
}

pub fn draw_floor_lines(mut gizmos: Gizmos, query: Query<(&Depth, &Floor)>) {
    for (_, floor) in query.iter() {
        let floor_y = to_viewport_coordinate_y(floor.0);
        // TODO calculate position in the real camera SCREEN_RES vs the virtual one
        gizmos.line(
            Vec3::new(-LINE_EXTENSION, floor_y, 0.),
            Vec3::new(LINE_EXTENSION, floor_y, 0.),
            Color::YELLOW_GREEN,
        );
    }
}

pub fn draw_colliders(
    mut gizmos: Gizmos,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    query: Query<(&ColliderData, &PxSubPosition)>,
) {
    let camera_pos = camera_query.get_single().unwrap();

    for (data, position) in query.iter() {
        let absolute_position = position.0 - camera_pos.0;
        for data in data.0.iter() {
            match data.shape {
                ColliderShape::Circle(radius) => {
                    gizmos.circle_2d(
                        to_viewport_coordinates(absolute_position + data.offset),
                        to_viewport_ratio_x(radius),
                        Color::ALICE_BLUE,
                    );
                }
                ColliderShape::Box(size) => {
                    gizmos.rect_2d(
                        // to_viewport_coordinates(absolute_position - half_rect),
                        to_viewport_coordinates(absolute_position + data.offset),
                        0.,
                        to_viewport_ratio(size),
                        Color::FUCHSIA,
                    );
                }
            }
        }
    }
}
