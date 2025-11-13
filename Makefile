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
.PHONY: gen-types
gen-types:
	@echo "Generating TypeScript types from Rust..."
	cargo run -p carcinisation --bin gen_types --features derive-ts

.PHONY: gen-zod
gen-zod:
	@echo "⚠️  gen-zod is deprecated and removed. Zod validation was removed from the pipeline."
	@echo "   TypeScript types are generated via 'make gen-types'"

.PHONY: gen-editor-types
gen-editor-types: gen-types
	@echo "✓ All editor type generation complete"

.PHONY: watch-types
watch-types:
	@echo "🔄 Starting type watcher..."
	@cargo watch -q --ignore "target/*" --ignore "*.ts" \
		-w apps/carcinisation/src \
		-s "bash -lc 'set -o pipefail; (printf \"🌀 Type watcher triggered\n\"; QUIET=1 RUSTFLAGS=-Awarnings cargo run -p carcinisation --bin gen_types --features derive-ts) 2>&1 | python3 -u -c \"import re, sys; keep = re.compile(r'^(🌀|⚡|✅|❌|⚠️)'); err = re.compile(r'\\berror\\b', re.IGNORECASE); [sys.stdout.write(line) for line in sys.stdin if keep.match(line) or err.search(line)]\"'"

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

.PHONY: lint-fix
lint-fix:
	cargo fmt --all
	proto run ruff -- check --fix
	pnpm lint:fix

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
	@echo "  dev-stage-editor      - Start the web-based Stage Editor (auto-generates types first)"
	@echo "  build-stage-editor    - Build stage-editor for production (auto-generates types first)"
	@echo "  ci-stage-editor       - Run stage-editor CI checks (types, lint, tests)"
	@echo "  watch-scene-files  - Run the scene watcher utility"
	@echo "  gen-types          - Generate TypeScript types from Rust (run automatically by stage-editor)"
	@echo "  watch-types        - Auto-regenerate TypeScript types on Rust file changes"
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
