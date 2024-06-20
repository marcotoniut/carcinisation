.PHONY: run dev dev-wasm editor release-wasm generate-palettes generate-typeface process-gfx

run:
	RUST_BACKTRACE=full; cargo run --features bevy/dynamic_linking

dev:
	cargo watch -x run --features bevy/dynamic_linking;

dev-wasm:
	cargo run --target wasm32-unknown-unknown

editor:
	cd editor && cargo run;

release-wasm:
	cargo build --release --target wasm32-unknown-unknown
	wasm-opt -O -ol 100 -s 100 -o target/wasm32-unknown-unknown/release/carcinisation.opt.wasm target/wasm32-unknown-unknown/release/carcinisation.wasm

generate-palettes:
	cd scripts/generate-palettes && cargo run;

generate-typeface:
	cd scripts/generate-typeface && cargo run;

process-gfx:
	cd scripts/process-gfx && cargo run;
