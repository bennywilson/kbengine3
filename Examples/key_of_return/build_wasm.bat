cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir target/wasm32-unknown-unknown/release target/wasm32-unknown-unknown/release/key_of_return.wasm
powershell cp index.html target/wasm32-unknown-unknown/release
python3 -m http.server -d target/wasm32-unknown-unknown/release
pause