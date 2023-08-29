run:
	cargo run --features bevy/dynamic_linking

dev:
	cargo watch -x run --features bevy/dynamic_linking;

generate-palettes:
	cd scripts/generate-palettes && cargo run;

generate-typeface:
	cd scripts/generate-typeface && cargo run;

process-gfx:
	cd scripts/process-gfx && cargo run;
