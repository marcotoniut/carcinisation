.PHONY: run
run:
	RUST_BACKTRACE=full; cargo run --features bevy/dynamic_linking

.PHONY: dev
dev:
	cargo watch -x run --features bevy/dynamic_linking

.PHONY: dev-wasm
dev-wasm:
	cargo run --target wasm32-unknown-unknown

.PHONY: launch-editor
launch-editor:
	cd tools/editor && cargo run

.PHONY: release-wasm
release-wasm:
	cargo build --release --target wasm32-unknown-unknown
	wasm-opt -O -ol 100 -s 100 -o target/wasm32-unknown-unknown/release/carcinisation.opt.wasm target/wasm32-unknown-unknown/release/carcinisation.wasm

.PHONY: generate-palettes
generate-palettes:
	cd scripts/generate-palettes && cargo run

.PHONY: generate-typeface
generate-typeface:
	cd scripts/generate-typeface && cargo run

.PHONY: process-gfx
process-gfx:
	cd scripts/process-gfx && cargo run

.PHONY: watch-scene-files
watch-scene-files:
	cd tools/scene-file-watcher && cargo run
