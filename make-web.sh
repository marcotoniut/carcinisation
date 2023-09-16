#cargo run --target wasm32-unknown-unknown
#wasm-bindgen --target web --out-dir deploy ./target/wasm32-unknown-unknown/debug/punished-gb.wasm
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --no-typescript --target web --out-dir web-deploy ./target/wasm32-unknown-unknown/release/punished-gb.wasm
wasm-opt -0z ./web-deploy/punished-gb.wasm --output ./web-deploy/punished-gb.wasm
cp -r ./assets ./web-deploy/assets