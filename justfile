# ─── Settings ─────────────────────────────────────────────────────────────────
set positional-arguments

# ─── Game Loop ────────────────────────────────────────────────────────────────

# List available recipes
default:
    @just --list

# Watch source & assets, rebuild & rerun via bacon
dev:
    RUST_BACKTRACE=full bacon run

# Run once without file watching
run:
    cargo run --bin carcinisation --package carcinisation --features bevy/dynamic_linking

# Run with BRP enabled for runtime inspection
run-brp:
    cargo run --bin carcinisation --package carcinisation --features bevy/dynamic_linking,brp

# Run pre-built debug binary with proper DYLD_LIBRARY_PATH (macOS)
debug-binary:
    @scripts/debug-carcinisation.sh

# Build once then run via wrapper script
build-and-run:
    @echo "Building carcinisation..."
    cargo build --bin carcinisation --package carcinisation --features bevy/dynamic_linking
    @echo "Running via debug wrapper..."
    @scripts/debug-carcinisation.sh

# Run the wasm target via 'bevy run ... web'
dev-wasm:
    bevy run --bin carcinisation --package carcinisation web

# Watch single-stage mode
dev-stage:
    @if [ ! -f apps/carcinisation/single-stage.config.ron ]; then \
        echo "single-stage.config.ron missing; copying defaults..."; \
        cp apps/carcinisation/single-stage.config.default.ron apps/carcinisation/single-stage.config.ron; \
    fi
    RUST_BACKTRACE=full bacon single-stage

# Watch lab mode
dev-lab:
    RUST_BACKTRACE=full bacon lab

# Watch gallery mode
dev-gallery:
    RUST_BACKTRACE=full bacon gallery

# ─── Multiplayer ──────────────────────────────────────────────────────────────

server-port := "7000"

# Run dedicated FPS server
dev-fps-server:
    RUST_BACKTRACE=full cargo run --bin carcinisation_server --package carcinisation_server -- --port {{ server-port }}

# Run FPS client connecting to local server
dev-fps-client:
    CARCINISATION_SKIP_CUTSCENES=1 CARCINISATION_SKIP_SPLASH=1 CARCINISATION_SKIP_MENU=1 \
    RUST_BACKTRACE=full cargo run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:{{ server-port }}

# Headless server + 1 client
dev-fps-pair:
    @echo "Starting headless server + 1 client (Ctrl+C stops both)..."
    @bash -c ' \
        SRV=; CLI=; \
        cleanup() { status=$$?; trap - INT TERM EXIT; [ -n "$$SRV" ] && kill "$$SRV" 2>/dev/null || true; [ -n "$$CLI" ] && kill "$$CLI" 2>/dev/null || true; exit "$$status"; }; \
        trap cleanup INT TERM EXIT; \
        RUST_BACKTRACE=full cargo run --bin carcinisation_server --package carcinisation_server -- --port {{ server-port }} & \
        SRV=$$!; \
        sleep 3; \
        CARCINISATION_SKIP_CUTSCENES=1 CARCINISATION_SKIP_SPLASH=1 CARCINISATION_SKIP_MENU=1 \
        RUST_BACKTRACE=full cargo run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:{{ server-port }} & \
        CLI=$$!; \
        echo "Press Ctrl+C to stop server+client"; \
        wait -n "$$SRV" "$$CLI"; \
    '

# Headless server + 2 clients
dev-fps-duo:
    @echo "Starting headless server + 2 clients (Ctrl+C stops all)..."
    @bash -c ' \
        SRV=; CLI0=; CLI1=; \
        cleanup() { status=$$?; trap - INT TERM EXIT; [ -n "$$SRV" ] && kill "$$SRV" 2>/dev/null || true; [ -n "$$CLI0" ] && kill "$$CLI0" 2>/dev/null || true; [ -n "$$CLI1" ] && kill "$$CLI1" 2>/dev/null || true; exit "$$status"; }; \
        trap cleanup INT TERM EXIT; \
        RUST_BACKTRACE=full cargo run --bin carcinisation_server --package carcinisation_server -- --port {{ server-port }} & \
        SRV=$$!; \
        sleep 3; \
        CARCINISATION_SKIP_CUTSCENES=1 CARCINISATION_SKIP_SPLASH=1 CARCINISATION_SKIP_MENU=1 \
        RUST_BACKTRACE=full cargo run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:{{ server-port }} & \
        CLI0=$$!; \
        sleep 1; \
        CARCINISATION_SKIP_CUTSCENES=1 CARCINISATION_SKIP_SPLASH=1 CARCINISATION_SKIP_MENU=1 \
        RUST_BACKTRACE=full cargo run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:{{ server-port }} & \
        CLI1=$$!; \
        echo "Press Ctrl+C to stop server+clients"; \
        wait -n "$$SRV" "$$CLI0" "$$CLI1"; \
    '

# ─── Web ──────────────────────────────────────────────────────────────────────

# Install wasm32 target, wasm-bindgen, wasm-server-runner, wasm-opt
install-web:
    @bash -c ' \
        set -euo pipefail; \
        if ! command -v wasm-opt &>/dev/null; then \
            echo "wasm-opt not found, installing binaryen..."; \
            case "$$(uname -s)" in \
                Darwin) brew install binaryen ;; \
                Linux)  sudo apt-get update -qq && sudo apt-get install -y -qq binaryen ;; \
                *)      echo "Unsupported platform"; exit 1 ;; \
            esac; \
        else \
            echo "wasm-opt already installed: $$(wasm-opt --version)"; \
        fi; \
        rustup target install wasm32-unknown-unknown; \
        cargo install wasm-server-runner; \
        cargo install -f wasm-bindgen-cli@0.2.120; \
    '

# Full web build: compile, wasm-bindgen, wasm-opt, stage assets
build-web:
    @bash -c ' \
        set -euo pipefail; \
        TARGET=wasm32-unknown-unknown; \
        OUT_DIR=web-deploy; \
        WASM=carcinisation; \
        rm -rf "$${OUT_DIR:?}/$${WASM}"*; \
        cargo build --release --target "$$TARGET" -p carcinisation --bin "$$WASM"; \
        wasm-bindgen --no-typescript --target web --out-dir "$$OUT_DIR" "./target/$${TARGET}/release/$${WASM}.wasm"; \
        wasm-opt -Oz "$${OUT_DIR}/$${WASM}_bg.wasm" --output "$${OUT_DIR}/$${WASM}_bg.wasm"; \
        rsync -a --delete ./assets/ "$${OUT_DIR}/assets/"; \
    '

# Alias for full web build
release-wasm: build-web

# ─── Tools & Assets ──────────────────────────────────────────────────────────

# Launch the in-house Bevy editor
launch-editor:
    cargo run --package editor --features full_editor

# Run scene file watcher
watch-scene-files:
    cargo run --package scene-file-watcher

# Build API docs
docs:
    @scripts/generate-docs.sh

# Build & serve API docs
docs-serve:
    @scripts/generate-docs.sh --serve

# Create/refresh Python virtualenv
py-setup:
    @test -d .venv || proto run python -- -m venv .venv
    .venv/bin/python -m pip install --upgrade pip
    .venv/bin/python -m pip install -r scripts/generate-palettes/requirements.txt

# Regenerate palette textures
generate-palettes:
    proto run python -- scripts/generate-palettes/run.py

# Rebuild bitmap fonts
generate-typeface:
    cargo run --package generate-typeface

# Process art assets for the game
process-gfx:
    cargo run --package process-gfx

# Export one Aseprite sprite entry (requires entity=<name> depth=<N>)
export-aseprite entity depth manifest="resources/sprites/data.toml" output-root="tmp/aseprite-export":
    cargo run -p process-aseprite -- --manifest {{ manifest }} --entity {{ entity }} --depth {{ depth }} --output-root {{ output-root }}

# Export attack atlases
export-attack-atlases manifest="resources/sprites/attacks/data.toml":
    cargo run -p process-aseprite -- --simple-atlases --manifest {{ manifest }}

# ─── Quality Gates ───────────────────────────────────────────────────────────

# cargo check workspace with all features
check:
    cargo check --workspace --all-features

# Build debug workspace
build:
    cargo build --workspace

# Build release workspace
build-release:
    cargo build --workspace --release

# Clean build artefacts
clean:
    cargo clean

# Format Rust sources
fmt:
    cargo fmt --all

# Format check + clippy (workspace-wide)
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Auto-fix what we can (fmt, ruff, pnpm lint, bevy lint)
lint-fix:
    cargo fmt --all
    proto run ruff -- check --fix scripts
    pnpm lint:fix
    -just bevy-lint-fix

# Bevy CLI lint
bevy-lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Bevy CLI lint with auto-fix
bevy-lint-fix:
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged

# Clippy with pedantic warnings
clippy-pedantic:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic

# Apply cargo fix suggestions (lib/tests)
fix:
    cargo fix --lib --tests --allow-dirty

# ─── Engine (carapace) ───────────────────────────────────────────────────────

# Run carapace unit tests
engine-test:
    cargo test --manifest-path crates/carapace/Cargo.toml --all-features

# Run a carapace example (usage: just engine-example <name>)
engine-example example:
    BRP_EXTRAS_PORT=15702 cargo run --example {{ example }} --manifest-path crates/carapace/Cargo.toml --features brp_extras

# Clippy the engine crate
engine-lint:
    cargo clippy --manifest-path crates/carapace/Cargo.toml --all-features -- -D warnings

# ─── Testing ─────────────────────────────────────────────────────────────────

# Run full workspace test suite
test:
    cargo test --workspace --all-features

# Re-run tests on change via bacon
test-watch:
    bacon test

# Run a single test (usage: just test-single <test_name>)
test-single test:
    cargo test --workspace --all-features {{ test }} -- --nocapture

# ─── Gallery ─────────────────────────────────────────────────────────────────

# Run gallery app
gallery:
    cargo run --bin gallery --package carcinisation --features bevy/dynamic_linking,gallery,brp

# ─── Legacy ──────────────────────────────────────────────────────────────────

# Legacy cargo-watch loop (kept for reference, prefer `just dev`)
dev-legacy:
    RUST_BACKTRACE=full cargo watch \
        --no-restart \
        -w apps/carcinisation/src \
        -w assets \
        -i target \
        -i .git \
        -s "bash -lc 'set -o pipefail; cargo run --bin carcinisation --package carcinisation --features bevy/dynamic_linking'"

# ─── Deployment ──────────────────────────────────────────────────────────────

deploy-target := "x86_64-unknown-linux-gnu"

# Cross-compile server binaries for deployment
deploy-build:
    cross build --release --target {{ deploy-target }} --bin carcinisation_server --package carcinisation_server --bin carcinisationctl --package carcinisationctl

# Build + deploy to remote server
deploy: deploy-build
    @echo "Deploying to remote server..."
    DEPLOY_SERVER_BINARY=target/{{ deploy-target }}/release/carcinisation_server \
    DEPLOY_CTL_BINARY=target/{{ deploy-target }}/release/carcinisationctl \
    bash deploy/deploy.sh
