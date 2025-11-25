# TODO review these scripts, move them under scripts/ and update them.

#cargo run --target wasm32-unknown-unknown
#wasm-bindgen --target web --out-dir deploy ./target/wasm32-unknown-unknown/debug/carcinisation.wasm
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --no-typescript --target web --out-dir web-deploy ./target/wasm32-unknown-unknown/release/carcinisation.wasm
wasm-opt -0z ./web-deploy/carcinisation.wasm --output ./web-deploy/carcinisation.wasm
cp -r ./assets ./web-deploy/assets
