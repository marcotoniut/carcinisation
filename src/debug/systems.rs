use bevy::prelude::*;

use crate::{
    globals::{SCREEN_RESOLUTION, VIEWPORT_RESOLUTION},
    stage::components::placement::*,
};

pub const LINE_EXTENSION: f32 = 1000.;

pub fn draw_floor_lines(mut gizmos: Gizmos, query: Query<(&Depth, &Floor)>) {
    for (_, floor) in query.iter() {
        let screen_y = SCREEN_RESOLUTION.y as f32;
        let floor_y = VIEWPORT_RESOLUTION.y * (floor.0 / screen_y - 0.5);
        // TODO calculate position in the real camera SCREEN_RES vs the virtual one
        gizmos.line(
            Vec3::new(-LINE_EXTENSION, floor_y, 0.),
            Vec3::new(LINE_EXTENSION, floor_y, 0.),
            Color::YELLOW_GREEN,
        );
    }
}
