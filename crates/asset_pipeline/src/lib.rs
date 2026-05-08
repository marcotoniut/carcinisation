#![allow(
    // Analysis/encoding casts (usize→f64, u64→f64) are intentional for statistics.
    clippy::cast_precision_loss,
    // Intentional truncating casts in encoding (u32→u8, u32→i32) are bounds-checked.
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    // LUT allocation is intentionally stack-based for performance.
    clippy::large_stack_arrays,
    // map().unwrap_or_else() with different error paths is intentional.
    clippy::map_unwrap_or,
)]

pub mod analysis;
pub mod aseprite;
pub mod composed_ron;
pub mod pxi;
