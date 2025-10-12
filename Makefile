# =============================================================================
# Project configuration
# =============================================================================
RUN_BIN ?= carcinisation
RUN_PACKAGE ?= carcinisation
ARGS ?=
FEATURES ?= bevy/dynamic_linking
WASM_FEATURES ?=

# Strip helper vars to avoid accidental extra whitespace
STRIPPED_ARGS := $(strip $(ARGS))
STRIPPED_FEATURES := $(strip $(FEATURES))
STRIPPED_WASM_FEATURES := $(strip $(WASM_FEATURES))

RUN_ARG_FLAGS := $(if $(STRIPPED_ARGS),-- $(STRIPPED_ARGS),)
RUN_FEATURE_FLAGS := $(if $(STRIPPED_FEATURES),--features $(STRIPPED_FEATURES),)
RUN_WASM_FEATURE_FLAGS := $(if $(STRIPPED_WASM_FEATURES),--features $(STRIPPED_WASM_FEATURES),)

CARGO_RUN_CMD := run --bin $(RUN_BIN) --package $(RUN_PACKAGE) $(RUN_FEATURE_FLAGS) $(RUN_ARG_FLAGS)
CARGO_WASM_RUN_CMD := run --target wasm32-unknown-unknown --bin $(RUN_BIN) --package $(RUN_PACKAGE) $(RUN_WASM_FEATURE_FLAGS) $(RUN_ARG_FLAGS)

# =============================================================================
# Game launchers
# =============================================================================
.PHONY: run
run:
	RUST_BACKTRACE=full cargo $(CARGO_RUN_CMD)

.PHONY: dev
dev:
	RUST_BACKTRACE=full cargo watch -x "$(CARGO_RUN_CMD)"

.PHONY: dev-wasm
dev-wasm:
	RUST_BACKTRACE=full cargo $(CARGO_WASM_RUN_CMD)

# =============================================================================
# Tooling launchers
# =============================================================================
.PHONY: launch-editor
launch-editor:
	RUST_BACKTRACE=full cargo run -p editor

.PHONY: watch-scene-files
watch-scene-files:
	RUST_BACKTRACE=full cargo run -p scene-file-watcher

# =============================================================================
# Asset generation
# =============================================================================
.PHONY: generate-palettes
generate-palettes:
	cargo run -p generate-palettes

.PHONY: generate-typeface
generate-typeface:
	cargo run -p generate-typeface

.PHONY: process-gfx
process-gfx:
	cargo run -p process-gfx

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
	cargo build --release --target wasm32-unknown-unknown
	wasm-opt -O -ol 100 -s 100 -o target/wasm32-unknown-unknown/release/carcinisation.opt.wasm target/wasm32-unknown-unknown/release/carcinisation.wasm

# =============================================================================
# Quality gates
# =============================================================================
.PHONY: check
check:
	cargo check --workspace --all-features

.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: clean
clean:
	cargo clean

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: lint
lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

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
	cargo watch -x "test --workspace"

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
	@echo "  run                - Launch the main binary (override RUN_BIN/RUN_PACKAGE/ARGS as needed)"
	@echo "  dev                - Auto-restart the game on changes via cargo-watch"
	@echo "  dev-wasm           - Run targeting wasm32-unknown-unknown (requires wasm-runner tooling)"
	@echo ""
	@echo "🛠 Tools & Assets:"
	@echo "  launch-editor      - Open the in-house Bevy editor"
	@echo "  watch-scene-files  - Run the scene watcher utility"
	@echo "  generate-palettes  - Regenerate color palette assets"
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
	@echo "  build              - Compile debug binaries"
	@echo "  build-release      - Compile optimized binaries"
	@echo "  fmt                - Format Rust sources"
	@echo "  lint               - Run clippy with warnings as errors"
	@echo "  fix                - Apply rustfix suggestions (lib/tests only)"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  test               - Run the full workspace test suite"
	@echo "  test-watch         - Re-run tests on change via cargo-watch"
	@echo "  test-single        - Run a single test (TEST=path::to::test)"
	@echo ""
	@echo "Env vars: RUN_BIN=$(RUN_BIN) RUN_PACKAGE=$(RUN_PACKAGE) FEATURES=$(FEATURES) WASM_FEATURES=$(WASM_FEATURES) ARGS=$(ARGS)"
