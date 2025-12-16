//! Defines the structure of the packed binary asset file.
//! This format is used for release builds to bundle all attack configs into a single file.

use crate::config::AttackConfig;
use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

/// The magic number to identify packed attack data files.
pub const MAGIC_NUMBER: [u8; 8] = *b"ATK_PACK";
/// The version of the packed format. Increment if breaking changes are made.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct PackedAttackData {
    pub header: PackedHeader,
    pub attacks: HashMap<String, AttackConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackedHeader {
    pub magic: [u8; 8],
    pub version: u32,
}

impl PackedHeader {
    pub fn new() -> Self {
        Self {
            magic: MAGIC_NUMBER,
            version: FORMAT_VERSION,
        }
    }

    /// Checks if the header is valid.
    pub fn validate(&self) -> bool {
        self.magic == MAGIC_NUMBER && self.version == FORMAT_VERSION
    }
}

impl Default for PackedHeader {
    fn default() -> Self {
        Self::new()
    }
}
