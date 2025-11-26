# =============================================================================
# Project configuration
# =============================================================================
RUN_BIN ?= carcinisation
RUN_PACKAGE ?= carcinisation
ARGS ?=
FEATURES ?= bevy/dynamic_linking
WASM_FEATURES ?=
BEVY ?= bevy

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
ifeq ($(OS),Windows_NT)
	PYTHON_BIN := $(PYTHON_VENV)/Scripts/python.exe
endif

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

.PHONY: dev-stage
dev-stage:
	@if [ ! -f apps/carcinisation/single-stage.config.ron ]; then \
		echo "single-stage.config.ron missing; copying defaults..."; \
		cp apps/carcinisation/single-stage.config.default.ron apps/carcinisation/single-stage.config.ron; \
	fi
	RUST_BACKTRACE=full bacon single-stage

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

# =============================================================================
# Tooling launchers
# =============================================================================
.PHONY: launch-editor
launch-editor:
	RUST_BACKTRACE=full $(BEVY) run --package editor

.PHONY: watch-scene-files
watch-scene-files:
	RUST_BACKTRACE=full $(BEVY) run --package scene-file-watcher

.PHONY: docs
docs:
	@scripts/generate-docs.sh

.PHONY: docs-serve
docs-serve:
	@scripts/generate-docs.sh --serve

.PHONY: dev-stage-editor
dev-stage-editor:
	pnpm --filter stage-editor dev

.PHONY: build-stage-editor
build-stage-editor:
	pnpm --filter stage-editor build

.PHONY: ci-stage-editor
ci-stage-editor:
	pnpm --filter stage-editor lint && pnpm --filter stage-editor test

# =============================================================================
# Type generation for stage-editor
# =============================================================================
TS_OUT := tools/stage-editor/src/types/generated
TS_RS_EXPORT_DIR := $(TS_OUT)
.PHONY: gen-types
gen-types:
	@echo "Generating TypeScript types from Rust..."
	TS_RS_EXPORT_DIR=$(TS_RS_EXPORT_DIR) TS_OUT=$(TS_OUT) \
	$(BEVY) run --package generate-editor-bindings --features derive-ts

.PHONY: gen-zod
gen-zod:
	@echo "⚠️  gen-zod is deprecated and removed. Zod validation was removed from the pipeline."
	@echo "   TypeScript types are generated via 'make gen-types'"

.PHONY: gen-editor-types
gen-editor-types: gen-types
	@echo "✓ All editor type generation complete"

.PHONY: watch-types
watch-types:
	@echo "🔄 Starting type watcher via bacon..."
	# Use bacon's gen-types job for cleaner, faster watching
	bacon gen-types

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

# =============================================================================
# Web targets
# =============================================================================
.PHONY: install-web-deps
install-web-deps:
	bash install-web.sh

.PHONY: build-web
build-web:
	bash make-web.sh

.PHONY: release-wasm
release-wasm:
	$(BEVY) build $(RUN_TARGET_FLAGS) --release --target wasm32-unknown-unknown
	wasm-opt -O -ol 100 -s 100 -o target/wasm32-unknown-unknown/release/carcinisation.opt.wasm target/wasm32-unknown-unknown/release/carcinisation.wasm

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
	$(BEVY) lint --workspace --all-targets --all-features

.PHONY: lint-fix
lint-fix:
	cargo fmt --all
	proto run ruff -- check --fix
	pnpm lint:fix
	$(BEVY) lint --workspace --all-targets --all-features --fix

.PHONY: clippy
clippy:
	@echo "⚠️  Use 'make lint' instead (runs workspace-wide clippy)"
	@$(MAKE) lint

.PHONY: fix
fix:
	cargo fix --lib --tests --allow-dirty

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
	@echo "  dev                - Watch and rebuild via bacon (replaces cargo-watch, faster and more reliable)"
	@echo "  debug-binary       - Run pre-built target/debug/carcinisation with proper DYLD_LIBRARY_PATH (for IDEs/debuggers)"
	@echo "  build-and-run      - Build once, then run via wrapper script (faster for repeated manual runs)"
	@echo "  dev-wasm           - Run the wasm target via 'bevy run ... web'"
	@echo ""
	@echo "🛠 Tools & Assets:"
	@echo "  launch-editor      - Open the in-house Bevy editor binary"
	@echo "  dev-stage-editor      - Start the web-based Stage Editor (auto-generates types first)"
	@echo "  build-stage-editor    - Build stage-editor for production (auto-generates types first)"
	@echo "  ci-stage-editor       - Run stage-editor CI checks (types, lint, tests)"
	@echo "  watch-scene-files  - Run the scene watcher utility"
	@echo "  gen-types          - Generate TypeScript types from Rust (run automatically by stage-editor)"
	@echo "  watch-types        - Auto-regenerate TypeScript types on Rust file changes"
	@echo "  docs               - Build local API docs (scripts/generate-docs.sh)"
	@echo "  palettes           - Regenerate color palette assets"
	@echo "  generate-typeface  - Rebuild bitmap fonts"
	@echo "  process-gfx        - Process art assets for the game"
	@echo ""
	@echo "🌐 Web Targets:"
	@echo "  install-web-deps   - Install wasm toolchain dependencies"
	@echo "  build-web          - Produce web build artifacts via make-web.sh"
	@echo "  release-wasm       - Build optimized wasm binary (output in target/wasm32-unknown-unknown/release)"
	@echo ""
	@echo "✅ Quality Gates:"
	@echo "  check              - cargo check across the workspace with all features"
	@echo "  build              - Compile debug binaries via 'bevy build --workspace'"
	@echo "  build-release      - Compile optimized binaries via 'bevy build --workspace --release'"
	@echo "  fmt                - Format Rust sources"
	@echo "  lint               - Run 'bevy lint' workspace-wide"
	@echo "  fix                - Apply rustfix suggestions (lib/tests only)"
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
