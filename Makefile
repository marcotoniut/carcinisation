run:
	cargo run --features bevy/dynamic_linking

dev:
	cargo watch -x run --features bevy/dynamic_linking;

quantize-images:
	cd scripts/quantize-images && cargo run;

generate-palettes:
	cd scripts/generate-palettes && cargo run;
