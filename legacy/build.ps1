rustup target add aarch64-linux-android wasm32-unknown-unknown x86_64-pc-windows-msvc
# cargo install cargo-cross
cargo install cargo-ndk
cargo build --target x86_64-pc-windows-msvc --release
cargo build --target wasm32-unknown-unknown --release
cargo ndk --target arm64-v8a build --release