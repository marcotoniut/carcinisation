//! Shared enemy presentation state for rendering.
//!
//! SP and MP client both produce `EnemyPresentationState`, ensuring identical
//! sprite selection regardless of authority mode.
//!
//! TODO(presentation): Migrate Mosquiton to this pattern.

#[derive(Clone, Debug, PartialEq)]
pub enum EnemyPresentationState {
    Idle,
    Moving,
    /// `phase`: normalized [0..1]. `visual_height`: arc offset in map units.
    Hopping {
        phase: f32,
        visual_height: f32,
    },
    Windup {
        attack: AttackPresentationKind,
        phase: f32,
    },
    Attacking {
        attack: AttackPresentationKind,
        phase: f32,
    },
    Recover,
    Dying {
        burn: bool,
        phase: f32,
    },
    Dead {
        burn: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackPresentationKind {
    Melee,
    Ranged,
}
