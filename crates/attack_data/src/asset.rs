//! Defines the custom Bevy asset types and their corresponding loaders.
//! This is the bridge between asset files on disk and the Bevy asset system.

use crate::{
    config::AttackConfig,
    packed::{PackedAttackData, FORMAT_VERSION, MAGIC_NUMBER},
};
use anyhow::anyhow;
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};

// --- RON Asset (for dev/hot-reloading) ---

#[derive(Asset, TypePath, Debug, Clone)]
pub struct AttackRonAsset {
    pub config: AttackConfig,
}

#[derive(Default)]
pub struct AttackRonAssetLoader;

impl AssetLoader for AttackRonAssetLoader {
    type Asset = AttackRonAsset;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let config: AttackConfig = ron::de::from_bytes(&bytes)?;
            Ok(AttackRonAsset { config })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

// --- Packed Binary Asset (for release) ---

#[derive(Asset, TypePath, Debug)]
pub struct AttackPackedAsset {
    pub data: PackedAttackData,
}

#[derive(Default)]
pub struct AttackPackedAssetLoader;

impl AssetLoader for AttackPackedAssetLoader {
    type Asset = AttackPackedAsset;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let data: PackedAttackData = bincode::deserialize(&bytes)?;

            if !data.header.validate() {
                return Err(anyhow!(
                    "Invalid packed attack data file. Magic: {:?}, Version: {}. Expected Magic: {:?}, Version: {}",
                    data.header.magic,
                    data.header.version,
                    MAGIC_NUMBER,
                    FORMAT_VERSION
                ));
            }

            Ok(AttackPackedAsset { data })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bin"]
    }
}
