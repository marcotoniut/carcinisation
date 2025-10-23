//! Convenience helpers for consistent debug logging.

use colored::*;

/// Logs a startup message for the given module (debug builds only).
pub fn debug_print_startup(module: &str) {
    println!("{} {}", "STARTUP".cyan(), module);
}

/// Logs a shutdown message for the given module (debug builds only).
pub fn debug_print_shutdown(module: &str) {
    println!("{} {}", "SHUTDOWN".magenta(), module);
}
