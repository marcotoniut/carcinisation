//! The main Bevy plugin for the attack data pipeline.
//! This wires up all the asset loaders, resources, and systems.

use crate::{
    asset::{AttackPackedAsset, AttackPackedAssetLoader, AttackRonAsset, AttackRonAssetLoader},
    compiler,
    runtime::{AttackRuntimeConfigs, AttackTuning},
};
use bevy::prelude::*;

pub struct AttackDataPlugin;

impl Plugin for AttackDataPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AttackRuntimeConfigs>();

        if cfg!(feature = "attack_hot_reload") {
            info!("Attack hot-reloading is enabled.");
            app.init_asset::<AttackRonAsset>()
                .init_asset_loader::<AttackRonAssetLoader>()
                .add_systems(Startup, setup_hot_reload)
                .add_systems(Update, process_ron_assets);
        } else {
            info!("Using packed attack assets.");
            app.init_asset::<AttackPackedAsset>()
                .init_asset_loader::<AttackPackedAssetLoader>()
                .add_systems(Startup, setup_packed_assets)
                .add_systems(Update, process_packed_assets);
        }
    }
}

// --- Hot-Reloading Path (dev) ---

#[derive(Resource)]
struct AttackHandleMap(bevy::utils::HashMap<String, Handle<AttackRonAsset>>);

fn setup_hot_reload(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Note: This relies on assets being discoverable.
    // For this to work well, ensure your assets are in the expected path.
    let handles: Vec<Handle<AttackRonAsset>> = asset_server.load_folder("attacks").unwrap();
    let mut handle_map = AttackHandleMap(bevy::utils::HashMap::new());
    for handle in handles {
        // The asset path gives us a unique key.
        let key = handle.path().unwrap().path().to_str().unwrap().to_string();
        handle_map.0.insert(key, handle);
    }
    commands.insert_resource(handle_map);
}

fn process_ron_assets(
    mut events: EventReader<AssetEvent<AttackRonAsset>>,
    assets: Res<Assets<AttackRonAsset>>,
    mut runtime_configs: ResMut<AttackRuntimeConfigs>,
) {
    for event in events.read() {
        match event {
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } => {
                if let Some(asset) = assets.get(*id) {
                    info!("Loading/reloading attack: {}", asset.config.attack_id);
                    let tuning = compiler::compile(&asset.config);
                    runtime_configs
                        .configs
                        .insert(asset.config.attack_id.clone(), tuning);
                }
            }
            AssetEvent::Removed { id } => {
                // To properly handle removal, we'd need to map the AssetId back to our attack_id.
                // For now, we'll log a warning that removal isn't fully handled.
                warn!("Attack asset removed (ID: {:?}), but not unloaded from runtime configs. Restart to clear.", id);
            }
            _ => {}
        }
    }
}

// --- Packed Asset Path (release) ---

#[derive(Resource)]
struct PackedAttacksHandle(Handle<AttackPackedAsset>);

fn setup_packed_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<AttackPackedAsset> = asset_server.load("attacks/attacks.bin");
    commands.insert_resource(PackedAttacksHandle(handle));
}

fn process_packed_assets(
    mut events: EventReader<AssetEvent<AttackPackedAsset>>,
    assets: Res<Assets<AttackPackedAsset>>,
    mut runtime_configs: ResMut<AttackRuntimeConfigs>,
    mut processed: Local<bool>,
) {
    // This system will trigger every time the asset event fires,
    // but we only want to process the packed data once.
    if *processed {
        return;
    }

    for event in events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            if let Some(asset) = assets.get(*id) {
                info!("Processing packed attack data...");
                let mut loaded_count = 0;
                for (id, config) in &asset.data.attacks {
                    let tuning: AttackTuning = compiler::compile(config);
                    runtime_configs.configs.insert(id.clone(), tuning);
                    loaded_count += 1;
                }
                info!("Loaded {} attacks from packed asset.", loaded_count);
                *processed = true;
                break;
            }
        }
    }
}
