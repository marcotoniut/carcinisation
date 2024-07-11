use bevy::prelude::*;
use colored::*;

pub fn debug_print_startup(module: &str) {
    println!("{} {}", "STARTUP".cyan(), module);
}

pub fn debug_print_shutdown(module: &str) {
    println!("{} {}", "SHUTDOWN".magenta(), module);
}
