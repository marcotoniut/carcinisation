dev:
	cargo watch -x run --features bevy/dynamic_linking;

quantize-sprites:
	cd scripts/quantize-sprites;
	cargo run;
