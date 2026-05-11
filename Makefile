# =============================================================================
# Project configuration
# =============================================================================
RUN_BIN ?= carcinisation
RUN_PACKAGE ?= carcinisation
ARGS ?=
FEATURES ?= bevy/dynamic_linking
WASM_FEATURES ?=
BEVY ?= bevy

# Multiplayer convenience
SERVER_PORT ?= 7000
CLIENT_FLAGS ?= CARCINISATION_SKIP_CUTSCENES=1 CARCINISATION_SKIP_SPLASH=1 CARCINISATION_SKIP_MENU=1

# Signal trap: Ctrl+C kills all child processes cleanly.
define SETUP_TRAP
trap "kill 0 2>/dev/null" INT TERM EXIT
endef

# Strip helper vars to avoid accidental extra whitespace
STRIPPED_ARGS := $(strip $(ARGS))
STRIPPED_FEATURES := $(strip $(FEATURES))
STRIPPED_WASM_FEATURES := $(strip $(WASM_FEATURES))

RUN_TARGET_FLAGS := --bin $(RUN_BIN) --package $(RUN_PACKAGE)
RUN_ARG_FLAGS := $(if $(STRIPPED_ARGS),-- $(STRIPPED_ARGS),)
RUN_FEATURE_FLAGS := $(if $(STRIPPED_FEATURES),--features $(STRIPPED_FEATURES),)
RUN_WASM_FEATURE_FLAGS := $(if $(STRIPPED_WASM_FEATURES),--features $(STRIPPED_WASM_FEATURES),)

BEVY_RUN_CMD := $(BEVY) run $(RUN_TARGET_FLAGS) $(RUN_FEATURE_FLAGS) $(RUN_ARG_FLAGS)
BEVY_WEB_RUN_CMD := $(BEVY) run $(RUN_TARGET_FLAGS) $(RUN_WASM_FEATURE_FLAGS) web $(RUN_ARG_FLAGS)

PYTHON_VENV := .venv
PYTHON_BIN := $(PYTHON_VENV)/bin/python
ASEPRITE_MANIFEST ?= resources/sprites/data.toml
ENTITY ?=
DEPTH ?=
OUTPUT_ROOT ?= tmp/aseprite-export
ifeq ($(OS),Windows_NT)
	PYTHON_BIN := $(PYTHON_VENV)/Scripts/python.exe
endif

# =============================================================================
# Multiplayer convenience targets
# =============================================================================

.PHONY: dev-fps-server
dev-fps-server:
	RUST_BACKTRACE=full $(BEVY) run --bin carcinisation_server --package carcinisation_server -- --port $(SERVER_PORT)

.PHONY: dev-fps-client
dev-fps-client:
	RUST_BACKTRACE=full $(CLIENT_FLAGS) $(BEVY) run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:$(SERVER_PORT)

.PHONY: dev-fps-pair
dev-fps-pair:
	@echo "Starting headless server + 1 client (Ctrl+C stops both)…"
	@bash -c '\
		set -euo pipefail; \
		$(SETUP_TRAP); \
		RUST_BACKTRACE=full $(BEVY) run --bin carcinisation_server --package carcinisation_server -- --port $(SERVER_PORT) & \
		SRV=$$!; \
		sleep 3; \
		$(CLIENT_FLAGS) RUST_BACKTRACE=full $(BEVY) run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:$(SERVER_PORT) & \
		CLI=$$!; \
		echo "Press Ctrl+C to stop server+client"; \
		wait $$SRV $$CLI \
	'

.PHONY: dev-fps-duo
dev-fps-duo:
	@echo "Starting headless server + 2 clients (Ctrl+C stops all)…"
	@bash -c '\
		set -euo pipefail; \
		$(SETUP_TRAP); \
		RUST_BACKTRACE=full $(BEVY) run --bin carcinisation_server --package carcinisation_server -- --port $(SERVER_PORT) & \
		SRV=$$!; \
		sleep 3; \
		$(CLIENT_FLAGS) RUST_BACKTRACE=full $(BEVY) run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:$(SERVER_PORT) & \
		CLI0=$$!; \
		sleep 1; \
		$(CLIENT_FLAGS) RUST_BACKTRACE=full $(BEVY) run --bin multiplayer_client --package carcinisation -- --connect 127.0.0.1:$(SERVER_PORT) & \
		CLI1=$$!; \
		echo "Press Ctrl+C to stop server+clients"; \
		wait $$SRV $$CLI0 $$CLI1 \
	'

# =============================================================================
# Game launchers
# =============================================================================
.PHONY: run
run:
	RUST_BACKTRACE=full $(BEVY_RUN_CMD)

.PHONY: debug-binary
debug-binary:
	# Run the pre-built debug binary with proper DYLD_LIBRARY_PATH (macOS dynamic linking)
	# This script is the canonical way to run target/debug/carcinisation outside cargo/bevy.
	# Useful for IDE debuggers, manual testing, or when you want to avoid cargo's overhead.
	@scripts/debug-carcinisation.sh

.PHONY: build-and-run
build-and-run:
	# Build once, then run via the wrapper script (faster than cargo run for repeated runs)
	@echo "Building carcinisation..."
	@cargo build --bin carcinisation --package carcinisation --features bevy/dynamic_linking
	@echo "Running via debug wrapper..."
	@scripts/debug-carcinisation.sh

.PHONY: dev
dev:
	# Watch game source and assets, rebuild and rerun automatically via bacon.
	# Uses bacon's 'run' job which matches the behavior of 'make run'.
	# On macOS with bevy/dynamic_linking, DYLD_LIBRARY_PATH is handled by cargo run.
	RUST_BACKTRACE=full bacon run

.PHONY: run-brp
run-brp:
	RUST_BACKTRACE=full $(BEVY) run $(RUN_TARGET_FLAGS) --features bevy/dynamic_linking,brp $(RUN_ARG_FLAGS)

.PHONY: dev-stage
dev-stage:
	@if [ ! -f apps/carcinisation/single-stage.config.ron ]; then \
		echo "single-stage.config.ron missing; copying defaults..."; \
		cp apps/carcinisation/single-stage.config.default.ron apps/carcinisation/single-stage.config.ron; \
	fi
	RUST_BACKTRACE=full bacon single-stage

.PHONY: dev-lab
dev-lab:
	RUST_BACKTRACE=full bacon lab

.PHONY: dev-gallery
dev-gallery:
	RUST_BACKTRACE=full bacon gallery

.PHONY: dev-legacy
dev-legacy:
	# Legacy cargo-watch based dev loop (kept for reference, prefer 'make dev')
	RUST_BACKTRACE=full cargo watch \
		--no-restart \
		-w apps/carcinisation/src \
		-w assets \
		-i target \
		-i .git \
		-s "bash -lc 'set -o pipefail; $(BEVY_RUN_CMD)'"

.PHONY: dev-wasm
dev-wasm:
	RUST_BACKTRACE=full $(BEVY_WEB_RUN_CMD)

.PHONY: gallery
gallery:
	RUST_BACKTRACE=full $(BEVY) run --bin gallery --package carcinisation --features bevy/dynamic_linking,gallery,brp

# =============================================================================
# Tooling launchers
# =============================================================================
.PHONY: launch-editor
launch-editor:
	RUST_BACKTRACE=full $(BEVY) run --package editor --features full_editor

.PHONY: watch-scene-files
watch-scene-files:
	RUST_BACKTRACE=full $(BEVY) run --package scene-file-watcher

.PHONY: docs
docs:
	@scripts/generate-docs.sh

.PHONY: docs-serve
docs-serve:
	@scripts/generate-docs.sh --serve

# =============================================================================
# Asset generation
# =============================================================================
.PHONY: py-setup
py-setup:
	@test -d $(PYTHON_VENV) || proto run python -- -m venv $(PYTHON_VENV)
	$(PYTHON_BIN) -m pip install --upgrade pip
	$(PYTHON_BIN) -m pip install -r scripts/generate-palettes/requirements.txt

PALETTE_DEPS := scripts/generate-palettes/.deps_installed
PALETTE_STAMP := scripts/generate-palettes/.palettes_stamp
PALETTE_SOURCES := scripts/generate-palettes/run.py scripts/generate-palettes/palettes.json

$(PALETTE_DEPS): scripts/generate-palettes/requirements.txt
	@echo "📦 Installing palette generator dependencies via proto..."
	proto run python -- -m pip install --upgrade pip >/dev/null
	proto run python -- -m pip install -r scripts/generate-palettes/requirements.txt
	@touch $(PALETTE_DEPS)

$(PALETTE_STAMP): $(PALETTE_DEPS) $(PALETTE_SOURCES)
	@echo "🎨 Building palette textures..."
	proto run python -- scripts/generate-palettes/run.py
	@touch $(PALETTE_STAMP)

generate-palettes: $(PALETTE_STAMP)

.PHONY: generate-typeface
generate-typeface:
	$(BEVY) run --package generate-typeface

.PHONY: process-gfx
process-gfx:
	$(BEVY) run --package process-gfx

.PHONY: export-aseprite
export-aseprite:
	@if [ -z "$(ENTITY)" ] || [ -z "$(DEPTH)" ]; then \
		echo "Usage: make export-aseprite ENTITY=mosquiton DEPTH=3 [ASEPRITE_MANIFEST=resources/sprites/data.toml] [OUTPUT_ROOT=tmp/aseprite-export]"; \
		exit 1; \
	fi
	cargo run -p process-aseprite -- --manifest $(ASEPRITE_MANIFEST) --entity $(ENTITY) --depth $(DEPTH) --output-root $(OUTPUT_ROOT)

ATTACK_MANIFEST ?= resources/sprites/attacks/data.toml

.PHONY: export-attack-atlases
export-attack-atlases:
	cargo run -p process-aseprite -- --simple-atlases --manifest $(ATTACK_MANIFEST)

# =============================================================================
# Web targets
# =============================================================================
.PHONY: install-web-deps
install-web-deps:
	bash install-web.sh

.PHONY: build-web
build-web:
	bash make-web.sh

# Alias: full web build + optimise + stage assets (same as build-web).
.PHONY: release-wasm
release-wasm: build-web

# =============================================================================
# Deployment
# =============================================================================
DEPLOY_TARGET ?= x86_64-unknown-linux-gnu
DEPLOY_SERVER_BINARY := target/$(DEPLOY_TARGET)/release/carcinisation_server
DEPLOY_CTL_BINARY := target/$(DEPLOY_TARGET)/release/carcinisationctl

.PHONY: deploy-build
deploy-build:
	cross build --release --target $(DEPLOY_TARGET) --bin carcinisation_server --package carcinisation_server --bin carcinisationctl --package carcinisationctl

.PHONY: deploy
deploy: deploy-build
	@echo "Deploying to remote server…"
	DEPLOY_SERVER_BINARY=$(DEPLOY_SERVER_BINARY) DEPLOY_CTL_BINARY=$(DEPLOY_CTL_BINARY) bash deploy/deploy.sh

# =============================================================================
# Quality gates
# =============================================================================
.PHONY: check
check:
	cargo check --workspace --all-features

.PHONY: build
build:
	$(BEVY) build --workspace

.PHONY: build-release
build-release:
	$(BEVY) build --workspace --release

.PHONY: clean
clean:
	cargo clean

.PHONY: fmt
fmt:
	cargo fmt --all
.PHONY: lint
lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: lint-fix
lint-fix:
	cargo fmt --all
	proto run ruff -- check --fix scripts
	pnpm lint:fix
	# rust lints can't "auto-fix" like ruff/eslint; clippy has limited suggestions but no universal --fix
	# keep bevy lint fix as optional
	@$(MAKE) bevy-lint-fix || true

.PHONY: bevy-lint
bevy-lint:
	$(BEVY) lint --workspace --all-targets --all-features

.PHONY: bevy-lint-fix
bevy-lint-fix:
	$(BEVY) lint --workspace --all-targets --all-features --fix

.PHONY: clippy
clippy:
	@echo "⚠️  Use 'make lint' instead (runs workspace-wide fmt+clippy)"
	@$(MAKE) lint

.PHONY: clippy-pedantic
clippy-pedantic:
	cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic

.PHONY: fix
fix:
	cargo fix --lib --tests --allow-dirty

# =============================================================================
# Engine (carapace)
# =============================================================================
CARAPACE_MANIFEST := crates/carapace/Cargo.toml

.PHONY: engine-test
engine-test:
	cargo test --manifest-path $(CARAPACE_MANIFEST) --all-features

.PHONY: engine-example
engine-example:
	@if [ -z "$(EXAMPLE)" ]; then \
		echo "Usage: make engine-example EXAMPLE=composite_sprite"; \
		echo "Available:"; \
		ls crates/carapace/examples/*.rs | xargs -n1 basename | sed 's/\.rs//' | sed 's/^/  /'; \
		exit 1; \
	fi
	BRP_EXTRAS_PORT=15702 cargo run --example $(EXAMPLE) --manifest-path $(CARAPACE_MANIFEST) --features brp_extras

.PHONY: engine-lint
engine-lint:
	cargo clippy --manifest-path $(CARAPACE_MANIFEST) --all-features -- -D warnings

# =============================================================================
# Testing
# =============================================================================
.PHONY: test
test:
	cargo test --workspace --all-features

.PHONY: test-watch
test-watch:
	# Continuously run tests via bacon (replaces cargo-watch)
	bacon test

.PHONY: test-single
test-single:
	@echo "Usage: make test-single TEST=my_test_name"
	@echo "Example: make test-single TEST=systems::my_system::tests::my_case"
	cargo test --workspace --all-features $(TEST) -- --nocapture

# =============================================================================
# Documentation
# =============================================================================
.PHONY: help
help:
	@echo "Carcinisation Makefile Commands"
	@echo "================================"
	@echo ""
	@echo "🎮 Game Loop:"
	@echo "  run                - Launch the main binary via 'bevy run' (override RUN_BIN/RUN_PACKAGE/ARGS as needed)"
	@echo "  run-brp            - Launch with BRP (Bevy Remote Protocol) enabled for runtime inspection"
	@echo "  dev                - Watch and rebuild via bacon (replaces cargo-watch, faster and more reliable)"
	@echo "  debug-binary       - Run pre-built target/debug/carcinisation with proper DYLD_LIBRARY_PATH (for IDEs/debuggers)"
	@echo "  build-and-run      - Build once, then run via wrapper script (faster for repeated manual runs)"
	@echo "  dev-wasm           - Run the wasm target via 'bevy run ... web'"
	@echo ""
	@echo "🛠 Tools & Assets:"
	@echo "  launch-editor      - Open the in-house Bevy editor binary"
	@echo "  watch-scene-files  - Run the scene watcher utility"
	@echo "  docs               - Build local API docs (scripts/generate-docs.sh)"
	@echo "  palettes           - Regenerate color palette assets"
	@echo "  generate-typeface  - Rebuild bitmap fonts"
	@echo "  process-gfx        - Process art assets for the game"
	@echo "  export-aseprite    - Export one Aseprite sprite entry from resources/sprites/data.toml"
	@echo ""
	@echo "🌐 Web Targets:"
	@echo "  install-web-deps   - Install wasm toolchain dependencies"
	@echo "  build-web          - Full web build: compile, wasm-bindgen, wasm-opt, stage assets"
	@echo "  release-wasm       - Alias for build-web"
	@echo ""
	@echo "✅ Quality Gates:"
	@echo "  check              - cargo check across the workspace with all features"
	@echo "  build              - Compile debug binaries via 'bevy build --workspace'"
	@echo "  build-release      - Compile optimized binaries via 'bevy build --workspace --release'"
	@echo "  fmt                - Format Rust sources"
	@echo "  lint               - Run 'bevy lint' workspace-wide"
	@echo "  fix                - Apply rustfix suggestions (lib/tests only)"
	@echo ""
	@echo "🔧 Engine (carapace):"
	@echo "  engine-test        - Run carapace unit tests (all features)"
	@echo "  engine-example     - Run a carapace example with BRP (EXAMPLE=composite_sprite)"
	@echo "  engine-lint        - Clippy the engine crate (all features)"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  test               - Run the full workspace test suite"
	@echo "  test-watch         - Re-run tests on change via bacon (replaces cargo-watch)"
	@echo "  test-single        - Run a single test (TEST=path::to::test)"
	@echo ""
	@echo "Note: This project uses bacon instead of cargo-watch for all watch workflows."
	@echo "      Install with: cargo install bacon --locked"
	@echo ""
	@echo "Env vars: RUN_BIN=$(RUN_BIN) RUN_PACKAGE=$(RUN_PACKAGE) FEATURES=$(FEATURES) WASM_FEATURES=$(WASM_FEATURES) ARGS=$(ARGS)"
