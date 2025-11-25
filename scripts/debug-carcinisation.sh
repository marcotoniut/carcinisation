#!/usr/bin/env bash
# =============================================================================
# debug-carcinisation.sh
# =============================================================================
# Wrapper script to run the debug binary with proper DYLD_LIBRARY_PATH on macOS.
#
# Why this is needed:
#   When using Bevy's dynamic_linking feature (bevy/dynamic_linking), the game
#   dynamically links to libstd-*.dylib and Bevy's shared libraries. On macOS,
#   the dynamic linker (dyld) needs DYLD_LIBRARY_PATH set to find:
#     1. Rust's standard library dylibs (from rustc's target-libdir)
#     2. Bevy's dylibs (from target/debug/deps)
#
# This script is the canonical way to run target/debug/carcinisation outside
# of `cargo run` or `bevy run` - especially useful for:
#   - IDE debuggers (Zed, VSCode with CodeLLDB)
#   - bacon watch workflows
#   - Manual testing of pre-built binaries
#
# Usage:
#   ./scripts/debug-carcinisation.sh [args...]
#
# Any arguments passed to this script are forwarded to the game binary.
#
# Note: This script changes to the repo root before execution, so it can be
# called from any working directory.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

RUST_LIB_DIR="$(rustc --print target-libdir)"
export DYLD_LIBRARY_PATH="$RUST_LIB_DIR:$REPO_ROOT/target/debug/deps${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"

BINARY="$REPO_ROOT/target/debug/carcinisation"
if [[ ! -f "$BINARY" ]]; then
    echo "Error: Binary not found at $BINARY" >&2
    echo "Run 'make build' or 'cargo build --bin carcinisation' first." >&2
    exit 1
fi

exec "$BINARY" "$@"
