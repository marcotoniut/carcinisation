//! Prediction data structures for client-side movement prediction.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_math::Vec2;
use carcinisation_fps_core::map::Map;
use carcinisation_fps_core::movement::SnapTurnKind;

use crate::tick::InputSequence;

/// Client-side collision map for prediction.
///
/// Populated from the rendering map when the multiplayer client connects.
/// Prediction systems use this for `apply_movement` collision detection,
/// matching the server's `ServerMap` resource.
#[derive(Resource)]
pub struct ClientMap(pub Map);

/// Snapshot of predicted state at a given input sequence.
#[derive(Clone, Debug)]
pub struct PredictionSnapshot {
    pub position: Vec2,
    pub angle: f32,
}

/// Input state stored for replay during reconciliation.
#[derive(Clone, Debug)]
pub struct PredictedInput {
    pub movement: Vec2,
    pub turn: f32,
    pub snap_turn: Option<SnapTurnKind>,
    /// `AimMode` active (`AimCommitment` only). Translation suppressed; turn still applies.
    pub aim_held: bool,
}

/// A single prediction entry: the input applied and the resulting state.
#[derive(Clone, Debug)]
pub struct PredictionEntry {
    pub sequence: InputSequence,
    pub input: PredictedInput,
    pub result: PredictionSnapshot,
    /// Fixed delta time used when applying this input (typically 1/30).
    pub dt: f32,
}

/// Ring buffer of recent prediction entries for reconciliation.
///
/// Entries are stored in chronological (insertion) order. The buffer
/// automatically drops the oldest entry when `MAX_ENTRIES` is exceeded.
#[derive(Resource, Default, Debug)]
pub struct PredictionHistory {
    entries: VecDeque<PredictionEntry>,
}

impl PredictionHistory {
    /// Maximum stored entries. At 30 Hz input, 60 entries = 2 seconds.
    pub const MAX_ENTRIES: usize = 60;

    /// Append a prediction entry. Drops the oldest if the buffer is full.
    pub fn push(&mut self, entry: PredictionEntry) {
        if self.entries.len() >= Self::MAX_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Remove all entries with sequence at or before `acked_seq` (wrapping-aware).
    /// After this call, only entries strictly after `acked_seq` remain.
    pub fn prune_through(&mut self, acked_seq: InputSequence) {
        self.entries.retain(|e| e.sequence.is_after(acked_seq));
    }

    /// Find the entry with an exact sequence match.
    #[must_use]
    pub fn get(&self, seq: InputSequence) -> Option<&PredictionEntry> {
        self.entries.iter().find(|e| e.sequence == seq)
    }

    /// Iterate all entries in chronological order.
    pub fn iter_all(&self) -> impl Iterator<Item = &PredictionEntry> {
        self.entries.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The most recent entry, if any.
    #[must_use]
    pub fn latest(&self) -> Option<&PredictionEntry> {
        self.entries.back()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::cast_precision_loss)]
    use super::*;

    fn entry(seq: u32, x: f32, y: f32) -> PredictionEntry {
        PredictionEntry {
            sequence: InputSequence(seq),
            input: PredictedInput {
                movement: Vec2::new(0.0, 1.0),
                turn: 0.0,
                snap_turn: None,
                aim_held: false,
            },
            result: PredictionSnapshot {
                position: Vec2::new(x, y),
                angle: 0.0,
            },
            dt: 1.0 / 30.0,
        }
    }

    #[test]
    fn push_and_get() {
        let mut history = PredictionHistory::default();
        history.push(entry(1, 1.0, 0.0));
        history.push(entry(2, 2.0, 0.0));

        assert_eq!(history.len(), 2);
        let e = history.get(InputSequence(1)).unwrap();
        assert!((e.result.position.x - 1.0).abs() < 1e-6);

        let e2 = history.get(InputSequence(2)).unwrap();
        assert!((e2.result.position.x - 2.0).abs() < 1e-6);

        assert!(history.get(InputSequence(99)).is_none());
    }

    #[test]
    fn prune_removes_old_entries() {
        let mut history = PredictionHistory::default();
        for i in 1..=5 {
            history.push(entry(i, i as f32, 0.0));
        }
        assert_eq!(history.len(), 5);

        history.prune_through(InputSequence(3));

        // Entries 1,2,3 removed. 4,5 remain.
        assert_eq!(history.len(), 2);
        assert!(history.get(InputSequence(3)).is_none());
        assert!(history.get(InputSequence(4)).is_some());
        assert!(history.get(InputSequence(5)).is_some());
    }

    #[test]
    fn prune_all() {
        let mut history = PredictionHistory::default();
        for i in 1..=3 {
            history.push(entry(i, 0.0, 0.0));
        }
        history.prune_through(InputSequence(3));
        assert!(history.is_empty());
    }

    #[test]
    fn prune_beyond_max() {
        let mut history = PredictionHistory::default();
        for i in 1..=3 {
            history.push(entry(i, 0.0, 0.0));
        }
        // Prune through seq=100 — all entries are "at or before" 100.
        history.prune_through(InputSequence(100));
        assert!(history.is_empty());
    }

    #[test]
    fn overflow_drops_oldest() {
        let mut history = PredictionHistory::default();
        for i in
            1..=u32::try_from(PredictionHistory::MAX_ENTRIES).expect("MAX_ENTRIES fits in u32") + 10
        {
            history.push(entry(i, i as f32, 0.0));
        }
        assert_eq!(history.len(), PredictionHistory::MAX_ENTRIES);

        // Oldest should be entry 11 (entries 1-10 were dropped).
        assert!(history.get(InputSequence(10)).is_none());
        assert!(history.get(InputSequence(11)).is_some());
    }

    #[test]
    fn clear_empties_history() {
        let mut history = PredictionHistory::default();
        history.push(entry(1, 0.0, 0.0));
        history.push(entry(2, 0.0, 0.0));
        assert_eq!(history.len(), 2);

        history.clear();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn latest_returns_most_recent() {
        let mut history = PredictionHistory::default();
        assert!(history.latest().is_none());

        history.push(entry(1, 1.0, 0.0));
        history.push(entry(2, 2.0, 0.0));

        let latest = history.latest().unwrap();
        assert_eq!(latest.sequence.0, 2);
    }

    #[test]
    fn prune_with_wrapping_sequences() {
        let mut history = PredictionHistory::default();
        // Sequences near u32::MAX wrapping to 0.
        history.push(entry(u32::MAX - 1, 0.0, 0.0));
        history.push(entry(u32::MAX, 0.0, 0.0));
        history.push(entry(0, 0.0, 0.0)); // wrapped
        history.push(entry(1, 0.0, 0.0)); // wrapped

        assert_eq!(history.len(), 4);

        // Prune through u32::MAX — should remove MAX-1 and MAX.
        history.prune_through(InputSequence(u32::MAX));

        // 0 and 1 are after u32::MAX in wrapping order.
        assert_eq!(history.len(), 2);
        assert!(history.get(InputSequence(0)).is_some());
        assert!(history.get(InputSequence(1)).is_some());
    }

    #[test]
    fn snap_turn_stored_in_entry() {
        let e = PredictionEntry {
            sequence: InputSequence(1),
            input: PredictedInput {
                movement: Vec2::ZERO,
                turn: 0.0,
                snap_turn: Some(SnapTurnKind::QuickTurn),
                aim_held: false,
            },
            result: PredictionSnapshot {
                position: Vec2::ZERO,
                angle: std::f32::consts::PI,
            },
            dt: 1.0 / 30.0,
        };
        assert!(matches!(e.input.snap_turn, Some(SnapTurnKind::QuickTurn)));
    }
}
