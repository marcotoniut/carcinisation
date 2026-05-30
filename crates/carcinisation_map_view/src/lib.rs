//! Wolf3D-style automap / minimap rendering.
//!
//! Provides [`MapViewPlugin`] which renders a palette-indexed top-down map
//! view from a Wolf3D-style grid map. Supports toggling between FPS view and
//! map view via [`MapViewToggle`].

pub mod classification;
pub mod config;
pub mod overlay;
pub mod rendering;
pub mod wall_colors;

use bevy::prelude::*;
use carapace::prelude::*;
use carcinisation_fps::plugin::{
    CameraRes, Config as FpsConfig, FpsViewSprite, MapRes, PaletteRes, WallTextures,
};
use classification::MapGrid;
use config::MapViewConfig;
use overlay::{MapViewMarkerOverlay, MapViewPlayerMarker};
use wall_colors::wall_color_pairs_from_textures;

/// Resource storing the base map layer.
#[derive(Resource)]
pub struct MapViewLayer<L: CxLayer>(pub L);

/// Resource storing the overlay layer (rendered after the base map).
#[derive(Resource)]
pub struct MapViewOverlayLayer<L: CxLayer>(pub L);

/// Toggle resource for map-view mode.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct MapViewToggle {
    pub enabled: bool,
}

impl MapViewToggle {
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// Marker component on the map-view sprite entity.
#[derive(Component)]
pub struct MapViewSprite;

/// Plugin that adds the map view with a separate overlay layer.
///
/// Insert [`MapViewToggle`] before adding this plugin to control initial
/// state. Reads `MapRes`, `WallTextures`, and `PaletteRes` from the FPS
/// plugin at startup.
pub struct MapViewPlugin<B: CxLayer, O: CxLayer> {
    base_layer: B,
    overlay_layer: O,
}

impl<B: CxLayer, O: CxLayer> MapViewPlugin<B, O> {
    #[must_use]
    pub fn new(base_layer: B, overlay_layer: O) -> Self {
        Self {
            base_layer,
            overlay_layer,
        }
    }
}

impl<B: CxLayer, O: CxLayer> Plugin for MapViewPlugin<B, O> {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapViewLayer(self.base_layer.clone()));
        app.insert_resource(MapViewOverlayLayer(self.overlay_layer.clone()));
        app.init_resource::<MapViewToggle>()
            .init_resource::<overlay::MapViewOverlay>()
            .init_resource::<MapViewConfig>()
            .add_systems(Update, toggle_map_view)
            .add_systems(
                PostStartup,
                (
                    init_map_view::<B>,
                    overlay::init_marker_overlay::<O>.after(init_map_view::<B>),
                    overlay::init_player_marker::<O>.after(init_map_view::<B>),
                    apply_initial_visibility
                        .after(overlay::init_marker_overlay::<O>)
                        .after(overlay::init_player_marker::<O>),
                ),
            )
            .add_systems(
                Update,
                (
                    overlay::build_entity_snapshot,
                    overlay::update_marker_overlay.after(overlay::build_entity_snapshot),
                    overlay::update_player_marker,
                    update_map_position.after(overlay::update_marker_overlay),
                )
                    .run_if(|toggle: Res<MapViewToggle>| toggle.enabled),
            );
    }
}

/// One-shot startup: build the map view sprite from FPS resources.
#[allow(clippy::too_many_arguments)]
fn init_map_view<L: CxLayer>(
    mut commands: Commands,
    map_res: Res<MapRes>,
    wall_textures: Res<WallTextures>,
    palette_res: Res<PaletteRes>,
    camera_res: Res<CameraRes>,
    fps_config: Res<FpsConfig>,
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    toggle: Res<MapViewToggle>,
    mut config: ResMut<MapViewConfig>,
    layer: Res<MapViewLayer<L>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;

    let mw = map_res.0.width as u32;
    let mh = map_res.0.height as u32;
    let sw = fps_config.screen_width;
    let sh = fps_config.screen_height;
    let tile_size = (sw / mw).min(sh / mh).max(1);
    let marker_size = tile_size;
    config.tile_size = tile_size;
    config.marker_size = marker_size;

    let wall_colors = wall_color_pairs_from_textures(&wall_textures.0);
    let floor_color = palette_res.0.floor;
    let spawn = camera_res.0.position;
    let player_starts = [(spawn.x, spawn.y)];

    let grid = MapGrid::from_fps_map(&map_res.0, floor_color, &wall_colors, &player_starts);
    let image = rendering::render_map_view(&grid, tile_size);
    let asset = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    let handle = sprite_assets.add(asset);

    let vis = if toggle.enabled {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    commands.spawn((
        CxSprite(handle),
        CxPosition(IVec2::ZERO),
        CxAnchor::BottomLeft,
        layer.0.clone(),
        CxRenderSpace::Camera,
        vis,
        MapViewSprite,
    ));
}

/// Every frame: offset the map and overlay so the player is always centred.
///
/// The map sprite uses `BottomLeft` anchor in carapace's Y-up screen space,
/// so the scroll offset uses un-flipped world Y directly — the Y-up rendering
/// naturally puts grid row 0 (south) at the bottom.
#[allow(clippy::type_complexity)]
fn update_map_position(
    camera: Res<carcinisation_fps::plugin::CameraRes>,
    config: Res<MapViewConfig>,
    fps_config: Res<FpsConfig>,
    mut pos_set: ParamSet<(
        Query<&mut CxPosition, (With<MapViewSprite>, With<CxSprite>)>,
        Query<&mut CxPosition, (With<MapViewMarkerOverlay>, With<CxSprite>)>,
    )>,
) {
    let ts = config.tile_size;
    let player_px = (camera.0.position.x * ts as f32) as i32;
    let player_py = (camera.0.position.y * ts as f32) as i32;
    let cx = fps_config.screen_width as i32 / 2;
    let cy = fps_config.screen_height as i32 / 2;
    let pos = IVec2::new(cx - player_px, cy - player_py);
    for mut p in pos_set.p0().iter_mut() {
        p.0 = pos;
    }
    for mut p in pos_set.p1().iter_mut() {
        p.0 = pos;
    }
}

/// One-shot after both base map and overlay sprites exist: hide FPS view when
/// `MapViewToggle` is already enabled at startup (e.g. `--map-view` flag).
fn apply_initial_visibility(
    toggle: Res<MapViewToggle>,
    mut fps_query: Query<&mut Visibility, (With<FpsViewSprite>, With<CxSprite>)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    if !toggle.enabled {
        return;
    }
    for mut vis in &mut fps_query {
        *vis = Visibility::Hidden;
    }
}

/// Input system: Cmd+M toggles map view.
#[allow(clippy::type_complexity)]
pub fn toggle_map_view(
    keys: Res<ButtonInput<KeyCode>>,
    mut toggle: ResMut<MapViewToggle>,
    mut vis_set: ParamSet<(
        Query<&mut Visibility, (With<MapViewSprite>, With<CxSprite>)>,
        Query<&mut Visibility, (With<MapViewMarkerOverlay>, With<CxSprite>)>,
        Query<&mut Visibility, (With<MapViewPlayerMarker>, With<CxSprite>)>,
        Query<&mut Visibility, (With<FpsViewSprite>, With<CxSprite>)>,
    )>,
) {
    let modifier_held = keys.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    if !(modifier_held && keys.just_pressed(KeyCode::KeyM)) {
        return;
    }

    toggle.enabled = !toggle.enabled;
    info!(
        "Map view {}",
        if toggle.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );

    let map_vis = if toggle.enabled {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut vis in vis_set.p0().iter_mut() {
        *vis = map_vis;
    }
    for mut vis in vis_set.p1().iter_mut() {
        *vis = map_vis;
    }
    for mut vis in vis_set.p2().iter_mut() {
        *vis = map_vis;
    }

    let fps_vis = if toggle.enabled {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut vis in vis_set.p3().iter_mut() {
        *vis = fps_vis;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_defaults_to_disabled() {
        let toggle = MapViewToggle::default();
        assert!(!toggle.enabled);
    }

    #[test]
    fn toggle_new_works() {
        let toggle = MapViewToggle::new(true);
        assert!(toggle.enabled);
    }
}
