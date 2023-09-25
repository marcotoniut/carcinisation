run:
	RUST_BACKTRACE=full; cargo run --features bevy/dynamic_linking

dev:
	cargo watch -x run --features bevy/dynamic_linking;

wasm:
	cargo run --target wasm32-unknown-unknown 

wasm-release:
	cargo run --release --target wasm32-unknown-unknown 

generate-palettes:
	cd scripts/generate-palettes && cargo run;

generate-typeface:
	cd scripts/generate-typeface && cargo run;

process-gfx:
	cd scripts/process-gfx && cargo run;
