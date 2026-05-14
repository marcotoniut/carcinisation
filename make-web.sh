#!/usr/bin/env bash
set -euo pipefail

# Build the WASM binary, bind JS glue, optimise, and stage assets.

TARGET=wasm32-unknown-unknown
OUT_DIR=web-deploy
WASM=carcinisation

# Clean stale artefacts so a failed prior build can't mask errors.
rm -rf "${OUT_DIR:?}/${WASM}.wasm" "${OUT_DIR:?}/${WASM}_bg.wasm" "${OUT_DIR:?}/${WASM}.js"

cargo build --release --target "$TARGET" -p carcinisation --bin "$WASM"

wasm-bindgen \
	--no-typescript \
	--target web \
	--out-dir "$OUT_DIR" \
	"./target/${TARGET}/release/${WASM}.wasm"

wasm-opt -Oz "${OUT_DIR}/${WASM}_bg.wasm" --output "${OUT_DIR}/${WASM}_bg.wasm"

# Sync assets (delete removed files).
rsync -a --delete ./assets/ "${OUT_DIR}/assets/"
