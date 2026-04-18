#![allow(
    // Analysis/encoding casts (usize→f64, u64→f64) are intentional for statistics.
    clippy::cast_precision_loss,
    // Intentional truncating casts in encoding (u32→u8, u32→i32) are bounds-checked.
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
)]

pub mod analysis;
pub mod aseprite;
pub mod composed_ron;
pub mod pxi;
