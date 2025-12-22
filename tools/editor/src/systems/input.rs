use crate::components::{
    Draggable, EditorCamera, SceneData, SelectedItem, SelectionOutline, StageSpawnRef,
};
use crate::constants::{
    CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_SENSITIVITY, CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN,
};
use bevy::ecs::system::SystemParam;
use bevy::image::TextureAtlasLayout;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use bevy_prototype_lyon::prelude::*;
use std::time::{Duration, Instant};

const ZOOM_SENSITIVITY: f32 = 0.003;
const WHEEL_ZOOM_SENSITIVITY: f32 = 0.0015;
const WHEEL_ANCHOR_TIMEOUT: Duration = Duration::from_millis(200);

type CameraTransformQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Camera, &'static mut Transform),
    (With<EditorCamera>, Without<SelectedItem>),
>;
type SelectedTransformQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (With<SelectedItem>, With<Draggable>, Without<EditorCamera>),
>;
type SelectedDragQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, Option<&'static StageSpawnRef>),
    (With<Draggable>, Without<EditorCamera>),
>;

#[derive(Resource, Default)]
pub struct DragState {
    pub active: Option<DragInfo>,
}

#[derive(Copy, Clone, Debug)]
pub struct DragInfo {
    pub entity: Entity,
    pub offset: Vec2,
}

#[derive(SystemParam)]
pub struct MousePressParams<'w, 's> {
    pub window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub images: Res<'w, Assets<Image>>,
    pub texture_atlas_layouts: Res<'w, Assets<TextureAtlasLayout>>,
    pub drag_state: ResMut<'w, DragState>,
    pub selected_query: Query<'w, 's, Entity, With<SelectedItem>>,
    pub outline_query: Query<'w, 's, Entity, With<SelectionOutline>>,
    pub draggable_query: Query<
        'w,
        's,
        (
            Entity,
            &'static Sprite,
            &'static GlobalTransform,
            Option<&'static Anchor>,
        ),
        With<Draggable>,
    >,
    pub camera_query: Query<'w, 's, (&'static Camera, &'static Transform), With<EditorCamera>>,
}

/// @system Alt + mouse motion zooms the camera around the cursor.
pub fn on_alt_mouse_motion(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<EditorCamera>>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let alt_pressed =
        keyboard_buttons.pressed(KeyCode::AltLeft) || keyboard_buttons.pressed(KeyCode::AltRight);

    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    for event in cursor_moved_events.read() {
        let delta = event
            .delta
            .or_else(|| last_cursor.map(|last| event.position - last));
        *last_cursor = Some(event.position);

        if !alt_pressed {
            continue;
        }

        if let Some(delta) = delta {
            let zoom_delta = -delta.y * ZOOM_SENSITIVITY;
            apply_zoom(&mut transform, zoom_delta);
        }
    }
}

/// @system Ctrl-drag pans the camera when nothing is selected; right-click drags selected entities.
pub fn on_ctrl_mouse_motion(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut selected_query: SelectedTransformQuery,
    mut camera_query: CameraTransformQuery,
    window_query: Query<Entity, With<PrimaryWindow>>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let ctrl_pressed = keyboard_buttons.pressed(KeyCode::ControlLeft)
        || keyboard_buttons.pressed(KeyCode::ControlRight);

    let no_selection = selected_query.is_empty();
    let Ok((camera, mut camera_transform)) = camera_query.single_mut() else {
        return;
    };
    let Ok(window_entity) = window_query.single() else {
        return;
    };

    for event in cursor_moved_events.read() {
        if event.window != window_entity {
            continue;
        }

        let delta = event
            .delta
            .or_else(|| last_cursor.map(|last| event.position - last));
        *last_cursor = Some(event.position);

        if ctrl_pressed && no_selection {
            if let Some(delta) = delta {
                camera_transform.translation.x -= delta.x * CAMERA_MOVE_SENSITIVITY;
                camera_transform.translation.y += delta.y * CAMERA_MOVE_SENSITIVITY;

                camera_transform.translation.x = camera_transform
                    .translation
                    .x
                    .clamp(-CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_BOUNDARY);
                camera_transform.translation.y = camera_transform
                    .translation
                    .y
                    .clamp(-CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_BOUNDARY);
            }
        }

        if mouse_buttons.pressed(MouseButton::Right) {
            let cursor_position = event.position;

            if let Some(world_position) =
                screen_to_world(camera, &camera_transform, cursor_position)
            {
                let world_position = world_position.extend(0.0);
                for mut transform in selected_query.iter_mut() {
                    transform.translation = world_position;
                }
            }
        }
    }
}

/// @system Left click selects the top-most draggable entity under the cursor.
pub fn on_mouse_press(
    buttons: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut params: MousePressParams,
) {
    if buttons.just_pressed(MouseButton::Left) {
        params.drag_state.active = None;

        let Ok(window) = params.window_query.single() else {
            return;
        };
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = params.camera_query.single() {
                if let Some(world_position) =
                    screen_to_world(camera, camera_transform, cursor_position)
                {
                    let mut candidates: Vec<_> = params.draggable_query.iter().collect();
                    candidates.sort_by(|a, b| {
                        b.2.translation()
                            .z
                            .partial_cmp(&a.2.translation().z)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                    let mut picked = None;
                    for (entity, sprite, global_transform, anchor) in candidates {
                        if sprite_hit_test(
                            sprite,
                            anchor.copied().unwrap_or_default(),
                            global_transform,
                            world_position,
                            &params.images,
                            &params.texture_atlas_layouts,
                        ) {
                            let position = global_transform.translation().truncate();
                            picked = Some((entity, sprite, anchor, position));
                            break;
                        }
                    }

                    if let Some((entity, sprite, anchor, position)) = picked {
                        for entity in params.selected_query.iter() {
                            commands.entity(entity).remove::<SelectedItem>();
                        }
                        for entity in params.outline_query.iter() {
                            commands.entity(entity).despawn();
                        }
                        commands.entity(entity).insert(SelectedItem);
                        spawn_selection_outline(
                            &mut commands,
                            entity,
                            sprite,
                            anchor.copied().unwrap_or_default(),
                            &params.images,
                            &params.texture_atlas_layouts,
                        );
                        params.drag_state.active = Some(DragInfo {
                            entity,
                            offset: world_position - position,
                        });
                    } else {
                        for entity in params.selected_query.iter() {
                            commands.entity(entity).remove::<SelectedItem>();
                        }
                        for entity in params.outline_query.iter() {
                            commands.entity(entity).despawn();
                        }
                        params.drag_state.active = None;
                    }
                }
            }
        }
    }
}

/// @system Clears drag state when the left mouse button is released.
pub fn on_mouse_release(buttons: Res<ButtonInput<MouseButton>>, mut drag_state: ResMut<DragState>) {
    if buttons.just_released(MouseButton::Left) {
        drag_state.active = None;
    }
}

/// @system Drag selected entities with the left mouse button.
pub fn on_mouse_drag(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut selected_query: SelectedDragQuery,
    mut scene_data: Option<ResMut<SceneData>>,
    mut drag_state: ResMut<DragState>,
    camera_query: Query<(&Camera, &Transform), With<EditorCamera>>,
    window_query: Query<Entity, With<PrimaryWindow>>,
) {
    let Some(drag_info) = drag_state.active else {
        return;
    };
    if !mouse_buttons.pressed(MouseButton::Left) {
        drag_state.active = None;
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(window_entity) = window_query.single() else {
        return;
    };

    for event in cursor_moved_events.read() {
        if event.window != window_entity {
            continue;
        }

        if let Some(world_position) = screen_to_world(camera, camera_transform, event.position) {
            let target_position = world_position - drag_info.offset;
            if let Ok((mut transform, spawn_ref)) = selected_query.get_mut(drag_info.entity) {
                transform.translation = target_position.extend(transform.translation.z);
                let Some(spawn_ref) = spawn_ref else {
                    continue;
                };
                if let Some(scene_data) = scene_data.as_mut() {
                    if let SceneData::Stage(stage_data) = scene_data.bypass_change_detection() {
                        update_spawn_from_drag(spawn_ref, target_position, stage_data);
                    }
                }
            } else {
                drag_state.active = None;
            }
        }
    }
}

/// @system Mouse wheel zooms around a stable cursor anchor for short bursts.
pub fn on_mouse_wheel(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&Camera, &mut Transform), With<EditorCamera>>,
    mut wheel_anchor: Local<Option<Vec2>>,
    mut wheel_anchor_timestamp: Local<Option<Instant>>,
) {
    let Ok((camera, mut camera_transform)) = query.single_mut() else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let mut wheel_events = mouse_wheel_events.read();
    let Some(first_event) = wheel_events.next() else {
        return;
    };

    let now = Instant::now();
    let reset_anchor = wheel_anchor_timestamp
        .map(|ts| now.duration_since(ts) > WHEEL_ANCHOR_TIMEOUT)
        .unwrap_or(true);
    if reset_anchor {
        *wheel_anchor = Some(cursor_position);
    }
    let anchor = *wheel_anchor.get_or_insert(cursor_position);
    wheel_anchor_timestamp.replace(now);

    let zoom_delta = first_event.y * WHEEL_ZOOM_SENSITIVITY;
    apply_zoom_at_cursor(camera, &mut camera_transform, anchor, zoom_delta);
    for event in wheel_events {
        let zoom_delta = event.y * WHEEL_ZOOM_SENSITIVITY;
        apply_zoom_at_cursor(camera, &mut camera_transform, anchor, zoom_delta);
    }
}

/// Converts a screen position to world coordinates using the editor camera.
fn screen_to_world(camera: &Camera, transform: &Transform, cursor_position: Vec2) -> Option<Vec2> {
    camera
        .viewport_to_world_2d(&GlobalTransform::from(*transform), cursor_position)
        .ok()
}

/// Applies a uniform zoom delta with clamp and returns whether it changed.
fn apply_zoom(transform: &mut Transform, delta: f32) -> bool {
    if delta.abs() < f32::EPSILON {
        return false;
    }
    let current = transform.scale.x;
    let target = (current + delta).clamp(CAMERA_ZOOM_MIN, CAMERA_ZOOM_MAX);
    if (target - current).abs() < f32::EPSILON {
        return false;
    }
    transform.scale = Vec3::new(target, target, transform.scale.z);
    true
}

/// Zooms toward a cursor position by offsetting camera translation after scaling.
fn apply_zoom_at_cursor(
    camera: &Camera,
    transform: &mut Transform,
    cursor_position: Vec2,
    delta: f32,
) {
    let Some(before) = screen_to_world(camera, transform, cursor_position) else {
        return;
    };
    if !apply_zoom(transform, delta) {
        return;
    }
    let Some(after) = screen_to_world(camera, transform, cursor_position) else {
        return;
    };
    let offset = before - after;
    transform.translation.x += offset.x;
    transform.translation.y += offset.y;
}

fn sprite_hit_test(
    sprite: &Sprite,
    anchor: Anchor,
    global_transform: &GlobalTransform,
    world_position: Vec2,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> bool {
    let Some(image) = images.get(&sprite.image) else {
        return false;
    };
    let local_position = global_transform
        .to_matrix()
        .inverse()
        .transform_point3(world_position.extend(0.0))
        .truncate();

    let Ok(texture_point) =
        sprite.compute_pixel_space_point(local_position, anchor, images, texture_atlas_layouts)
    else {
        return false;
    };

    let size = image.size();
    if size.x == 0 || size.y == 0 {
        return false;
    }
    let x = texture_point.x.floor().clamp(0.0, size.x as f32 - 1.0) as usize;
    let y = texture_point.y.floor().clamp(0.0, size.y as f32 - 1.0) as usize;
    let idx = (y * size.x as usize + x) * 4;
    let Some(data) = image.data.as_deref() else {
        return false;
    };
    if idx + 3 >= data.len() {
        return false;
    }
    data[idx + 3] > 0
}

fn spawn_selection_outline(
    commands: &mut Commands,
    entity: Entity,
    sprite: &Sprite,
    anchor: Anchor,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) {
    let size = sprite_size(sprite, images, texture_atlas_layouts).unwrap_or(Vec2::ZERO);
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }

    let sprite_center = -anchor.as_vec() * size;
    let shape = shapes::Rectangle {
        extents: size,
        origin: RectangleOrigin::Center,
        radii: None,
    };
    commands.entity(entity).with_children(|builder| {
        builder.spawn((
            SelectionOutline,
            Name::new("Selection Outline"),
            ShapeBuilder::with(&shape)
                .stroke((Color::WHITE, 1.0))
                .build(),
            Transform::from_translation(sprite_center.extend(0.1)),
            GlobalTransform::default(),
        ));
    });
}

fn sprite_size(
    sprite: &Sprite,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<Vec2> {
    sprite
        .custom_size
        .or_else(|| sprite_texture_rect(sprite, images, texture_atlas_layouts).map(|r| r.size()))
}

fn sprite_texture_rect(
    sprite: &Sprite,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<Rect> {
    let image_size = images.get(&sprite.image).map(Image::size)?;
    let atlas_rect = sprite
        .texture_atlas
        .as_ref()
        .and_then(|s| s.texture_rect(texture_atlas_layouts))
        .map(|r| r.as_rect());
    let base_rect = Rect::new(0.0, 0.0, image_size.x as f32, image_size.y as f32);

    let rect = match (atlas_rect, sprite.rect) {
        (None, None) => base_rect,
        (None, Some(sprite_rect)) => sprite_rect,
        (Some(atlas_rect), None) => atlas_rect,
        (Some(atlas_rect), Some(mut sprite_rect)) => {
            sprite_rect.min += atlas_rect.min;
            sprite_rect.max += atlas_rect.min;
            sprite_rect
        }
    };
    Some(rect)
}

fn update_spawn_from_drag(
    spawn_ref: &StageSpawnRef,
    world_position: Vec2,
    stage_data: &mut carcinisation::stage::data::StageData,
) {
    match *spawn_ref {
        StageSpawnRef::Static { index } => {
            if let Some(spawn) = stage_data.spawns.get_mut(index) {
                set_spawn_coordinates(spawn, world_position);
            }
        }
        StageSpawnRef::Step {
            step_index,
            spawn_index,
            step_origin,
        } => {
            if let Some(step) = stage_data.steps.get_mut(step_index) {
                let local_position = world_position - step_origin;
                match step {
                    carcinisation::stage::data::StageStep::Tween(step) => {
                        if let Some(spawn) = step.spawns.get_mut(spawn_index) {
                            set_spawn_coordinates(spawn, local_position);
                        }
                    }
                    carcinisation::stage::data::StageStep::Stop(step) => {
                        if let Some(spawn) = step.spawns.get_mut(spawn_index) {
                            set_spawn_coordinates(spawn, local_position);
                        }
                    }
                    carcinisation::stage::data::StageStep::Cinematic(_) => {}
                }
            }
        }
    }
}

fn set_spawn_coordinates(spawn: &mut carcinisation::stage::data::StageSpawn, position: Vec2) {
    use carcinisation::stage::data::StageSpawn;

    match spawn {
        StageSpawn::Destructible(spawn) => spawn.coordinates = position,
        StageSpawn::Enemy(spawn) => spawn.coordinates = position,
        StageSpawn::Object(spawn) => spawn.coordinates = position,
        StageSpawn::Pickup(spawn) => spawn.coordinates = position,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::camera::RenderTargetInfo;
    use bevy::ecs::message::Messages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy::window::WindowResolution;
    use carcinisation::stage::components::placement::Depth;
    use carcinisation::stage::data::{ObjectSpawn, ObjectType, SkyboxData, StageData, StageSpawn};

    #[derive(Bundle)]
    struct TestCameraBundle {
        camera: Camera,
        camera_2d: Camera2d,
        transform: Transform,
        global_transform: GlobalTransform,
    }

    #[test]
    fn drag_updates_stage_spawn_coordinates() {
        let mut app = App::new();
        app.add_message::<CursorMoved>();
        app.init_resource::<DragState>();
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<TextureAtlasLayout>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let stage_data = StageData {
            name: "Test".to_string(),
            background_path: "".to_string(),
            music_path: "".to_string(),
            skybox: SkyboxData {
                path: "".to_string(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: vec![StageSpawn::Object(ObjectSpawn {
                object_type: ObjectType::BenchBig,
                coordinates: Vec2::ZERO,
                depth: Depth::Zero,
            })],
            steps: Vec::new(),
            on_start_transition_o: None,
            on_end_transition_o: None,
        };
        app.insert_resource(SceneData::Stage(stage_data));

        app.add_systems(Update, (on_mouse_press, on_mouse_drag, on_mouse_release));

        let window_entity = app
            .world_mut()
            .spawn((
                Window {
                    resolution: WindowResolution::new(200, 200),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();

        {
            let mut window_entity_mut = app.world_mut().entity_mut(window_entity);
            let mut window = window_entity_mut.get_mut::<Window>().unwrap();
            window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
        }

        let mut camera = Camera::default();
        let half = Vec2::new(100.0, 100.0);
        camera.computed.clip_from_view =
            Mat4::from_scale(Vec3::new(1.0 / half.x, 1.0 / half.y, 1.0));
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(200, 200),
            scale_factor: 1.0,
        });

        app.world_mut().spawn((
            TestCameraBundle {
                camera,
                camera_2d: Camera2d,
                transform: Transform::default(),
                global_transform: GlobalTransform::default(),
            },
            EditorCamera,
        ));

        let image = Image::new_fill(
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        let image_handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);

        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let entity = app
            .world_mut()
            .spawn((
                Sprite {
                    image: image_handle,
                    ..default()
                },
                Anchor::CENTER,
                Draggable,
                StageSpawnRef::Static { index: 0 },
                transform,
                GlobalTransform::from(transform),
            ))
            .id();

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        app.world_mut()
            .resource_mut::<Messages<CursorMoved>>()
            .write(CursorMoved {
                window: window_entity,
                position: Vec2::new(120.0, 100.0),
                delta: Some(Vec2::new(20.0, 0.0)),
            });
        app.update();

        let transform = app.world().entity(entity).get::<Transform>().unwrap();
        assert!((transform.translation.x - 20.0).abs() < 0.01);
        assert!((transform.translation.y - 0.0).abs() < 0.01);

        let scene_data = app.world().resource::<SceneData>();
        let SceneData::Stage(stage_data) = scene_data else {
            panic!("Expected stage data");
        };
        let StageSpawn::Object(spawn) = &stage_data.spawns[0] else {
            panic!("Expected object spawn");
        };
        assert!((spawn.coordinates.x - 20.0).abs() < 0.01);
        assert!((spawn.coordinates.y - 0.0).abs() < 0.01);
    }
}
