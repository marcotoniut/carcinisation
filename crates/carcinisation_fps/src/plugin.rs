//! Bevy plugin that encapsulates the first-person raycaster systems.
//!
//! The plugin is generic over the game's layer type. The caller provides
//! a [`FpConfig`] that specifies which layer the FP view renders into
//! and the path to the map RON file.

use std::marker::PhantomData;

use bevy::prelude::*;
use carapace::prelude::*;

/// System set for FP plugin systems. External input systems should run
/// `.before(FpSystems)` so the FP plugin reads updated state.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FpSystems;

use crate::{
    billboard::{
        Billboard, billboards_from_enemies_indexed, billboards_from_projectiles, make_death_sprite,
        make_enemy_sprite, make_pillar_sprite, make_projectile_sprite,
    },
    camera::FpCamera,
    collision::try_move,
    data::{FpEntityKind, FpMapData},
    enemy::{FpEnemy, FpProjectile, hitscan, tick_enemies, tick_projectiles},
    map::FpMap,
    render::{FpPalette, draw_crosshair, draw_overlay_tint, render_fp_scene},
};

/// Configuration for the FP plugin.
#[derive(Resource, Clone)]
pub struct FpConfig {
    /// RON map file contents (pre-loaded string).
    pub map_ron: String,
    /// The layer value the FP sprite entity should render into.
    /// This is stored as a closure that produces the layer component.
    pub screen_width: u32,
    pub screen_height: u32,
    pub move_speed: f32,
    pub turn_speed: f32,
    pub hitscan_damage: u32,
    pub player_max_health: u32,
}

impl Default for FpConfig {
    fn default() -> Self {
        Self {
            map_ron: String::new(),
            screen_width: 160,
            screen_height: 144,
            move_speed: 2.0,
            turn_speed: 2.0,
            hitscan_damage: 15,
            player_max_health: 100,
        }
    }
}

// --- Resources ---

#[derive(Resource)]
pub struct FpSpriteHandle(pub Handle<CxSpriteAsset>);

#[derive(Resource)]
pub struct FpWallTextures(pub Vec<CxImage>);

#[derive(Resource)]
pub struct FpCameraRes(pub FpCamera);

#[derive(Resource)]
pub struct FpMapRes(pub FpMap);

#[derive(Resource)]
pub struct FpPaletteRes(pub FpPalette);

#[derive(Resource)]
pub struct FpStaticBillboards(pub Vec<Billboard>);

#[derive(Resource)]
pub struct FpEnemies(pub Vec<FpEnemy>);

#[derive(Resource)]
pub struct FpEnemySpriteIndices(pub Vec<usize>);

#[derive(Resource)]
pub struct FpSpritePairs(pub Vec<(CxImage, CxImage)>);

#[derive(Resource)]
pub struct FpProjectiles(pub Vec<FpProjectile>);

#[derive(Resource)]
pub struct FpProjectileSprite(pub CxImage);

#[derive(Resource)]
pub struct FpPlayerHealth(pub u32);

#[derive(Resource)]
pub struct FpPlayerDead(pub bool);

/// Marker resource indicating FP mode is active.
#[derive(Resource)]
pub struct FpActive;

// --- Plugin ---

/// First-person raycaster plugin.
///
/// Generic over `L: CxLayer` so it works with any game's layer enum.
/// Insert [`FpConfig`] before adding this plugin, or the setup system
/// will panic.
pub struct FpPlugin<L: CxLayer> {
    _l: PhantomData<L>,
}

impl<L: CxLayer> Default for FpPlugin<L> {
    fn default() -> Self {
        Self { _l: PhantomData }
    }
}

impl<L: CxLayer> FpPlugin<L> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L: CxLayer + Default> Plugin for FpPlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_fp::<L>);
        app.init_resource::<FpShootRequest>();
        app.add_systems(
            Update,
            (tick_enemy_ai, handle_shooting, update_fp_view)
                .chain()
                .in_set(FpSystems)
                .run_if(resource_exists::<FpActive>),
        );
    }
}

/// Setup system: parses the map from FpConfig, builds all resources.
///
/// Input handling is NOT included — the caller (binary or game plugin)
/// is responsible for reading input and updating `FpCameraRes`.
fn setup_fp<L: CxLayer + Default>(
    mut commands: Commands,
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    config: Res<FpConfig>,
) {
    let map_data = FpMapData::from_ron(&config.map_ron)
        .unwrap_or_else(|e| panic!("failed to parse FP map: {e}"));
    let map = map_data.to_map();
    let camera = map_data.to_camera();
    let palette = map_data.to_palette();
    let textures = map_data.build_wall_textures();

    let mut static_billboards = Vec::new();
    let mut enemies = Vec::new();
    let mut enemy_sprite_indices = Vec::new();

    let procedural_alive = make_enemy_sprite(24, 2);
    let procedural_death = make_death_sprite(24, 1);
    let sprite_pairs: Vec<(CxImage, CxImage)> =
        vec![(procedural_alive.clone(), procedural_death.clone())];

    for spawn in &map_data.entities {
        let pos = Vec2::new(spawn.x, spawn.y);
        match &spawn.kind {
            FpEntityKind::Pillar {
                color,
                width,
                height,
            } => {
                static_billboards.push(Billboard {
                    position: pos,
                    sprite: make_pillar_sprite(*width, *height, *color),
                });
            }
            FpEntityKind::Enemy { health, speed, .. } => {
                enemies.push(FpEnemy::new(pos, *health, *speed));
                enemy_sprite_indices.push(0);
            }
            FpEntityKind::SpriteEnemy { health, speed, .. } => {
                enemies.push(FpEnemy::new(pos, *health, *speed));
                // TODO: asset-loaded sprites (requires async resolution).
                // For now, use procedural fallback.
                enemy_sprite_indices.push(0);
            }
        }
    }

    // Render first frame.
    let enemy_bbs = billboards_from_enemies_indexed(&enemies, &enemy_sprite_indices, &sprite_pairs);
    let all_bbs: Vec<Billboard> = static_billboards.iter().cloned().chain(enemy_bbs).collect();
    let mut image = CxImage::empty(UVec2::new(config.screen_width, config.screen_height));
    render_fp_scene(&mut image, &map, &camera, &textures, &palette, &all_bbs);
    draw_crosshair(&mut image, 4);
    let initial = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    let handle = sprite_assets.add(initial);

    // Spawn the FP view sprite entity.
    commands.spawn((
        CxSprite(handle.clone()),
        CxPosition(IVec2::ZERO),
        CxAnchor::BottomLeft,
        L::default(),
        CxRenderSpace::Camera,
        Visibility::Visible,
    ));

    commands.insert_resource(FpSpriteHandle(handle));
    commands.insert_resource(FpWallTextures(textures));
    commands.insert_resource(FpCameraRes(camera));
    commands.insert_resource(FpMapRes(map));
    commands.insert_resource(FpPaletteRes(palette));
    commands.insert_resource(FpStaticBillboards(static_billboards));
    commands.insert_resource(FpEnemies(enemies));
    commands.insert_resource(FpEnemySpriteIndices(enemy_sprite_indices));
    commands.insert_resource(FpSpritePairs(sprite_pairs));
    commands.insert_resource(FpProjectiles(Vec::new()));
    commands.insert_resource(FpProjectileSprite(make_projectile_sprite(3)));
    commands.insert_resource(FpPlayerHealth(config.player_max_health));
    commands.insert_resource(FpPlayerDead(false));
    commands.insert_resource(FpActive);

    info!("FP mode initialized");
}

/// Movement helper for external input systems. Call this with the computed
/// move delta to update the camera position with wall collision.
pub fn move_camera(camera: &mut FpCamera, delta: Vec2, map: &FpMap) {
    try_move(&mut camera.position, delta, 0.2, map);
}

fn tick_enemy_ai(
    time: Res<Time>,
    camera: Res<FpCameraRes>,
    map: Res<FpMapRes>,
    mut enemies: ResMut<FpEnemies>,
    mut projectiles: ResMut<FpProjectiles>,
    mut health: ResMut<FpPlayerHealth>,
    mut dead: ResMut<FpPlayerDead>,
) {
    if dead.0 {
        return;
    }
    let dt = time.delta_secs();

    let new_projs = tick_enemies(&mut enemies.0, camera.0.position, &map.0, dt);
    projectiles.0.extend(new_projs);

    let damage = tick_projectiles(&mut projectiles.0, camera.0.position, &map.0, dt);
    if damage > 0 {
        health.0 = health.0.saturating_sub(damage);
        if health.0 == 0 {
            dead.0 = true;
            info!("Player died!");
        }
    }
}

fn handle_shooting(
    camera: Res<FpCameraRes>,
    map: Res<FpMapRes>,
    config: Res<FpConfig>,
    mut enemies: ResMut<FpEnemies>,
    dead: Res<FpPlayerDead>,
    mut shoot: ResMut<FpShootRequest>,
) {
    if dead.0 || !shoot.0 {
        return;
    }
    shoot.0 = false;

    let result = hitscan(&camera.0, &enemies.0, &map.0);
    if let Some(idx) = result.enemy_idx {
        enemies.0[idx].take_damage(config.hitscan_damage);
        info!(
            "Hit enemy {idx} at distance {:.1} — health: {}",
            result.distance, enemies.0[idx].health
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn update_fp_view(
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    handle: Res<FpSpriteHandle>,
    textures: Res<FpWallTextures>,
    camera: Res<FpCameraRes>,
    map: Res<FpMapRes>,
    palette: Res<FpPaletteRes>,
    config: Res<FpConfig>,
    static_bbs: Res<FpStaticBillboards>,
    enemies: Res<FpEnemies>,
    indices: Res<FpEnemySpriteIndices>,
    pairs: Res<FpSpritePairs>,
    projectiles: Res<FpProjectiles>,
    proj_sprite: Res<FpProjectileSprite>,
    health: Res<FpPlayerHealth>,
    dead: Res<FpPlayerDead>,
) {
    let mut image = CxImage::empty(UVec2::new(config.screen_width, config.screen_height));

    let enemy_bbs = billboards_from_enemies_indexed(&enemies.0, &indices.0, &pairs.0);
    let proj_bbs = billboards_from_projectiles(&projectiles.0, &proj_sprite.0);
    let all_bbs: Vec<Billboard> = static_bbs
        .0
        .iter()
        .cloned()
        .chain(enemy_bbs)
        .chain(proj_bbs)
        .collect();

    render_fp_scene(
        &mut image,
        &map.0,
        &camera.0,
        &textures.0,
        &palette.0,
        &all_bbs,
    );

    if dead.0 {
        draw_overlay_tint(&mut image, 2, 0.5);
    } else {
        draw_crosshair(&mut image, 4);
    }

    // Health bar at top-left.
    let bar_w = 20;
    let filled = (health.0 as i32 * bar_w / config.player_max_health as i32).max(0);
    {
        let data = image.data_mut();
        let w = config.screen_width as i32;
        for x in 1..=bar_w {
            let color = if x <= filled { 2 } else { 1 };
            data[(w + x) as usize] = color;
            data[(2 * w + x) as usize] = color;
        }
    }

    if let Some(asset) = sprite_assets.get_mut(&handle.0) {
        *asset = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    }
}

/// Set to `true` from your input system to trigger a hitscan shot.
/// The plugin resets it to `false` after processing.
#[derive(Resource, Default)]
pub struct FpShootRequest(pub bool);
