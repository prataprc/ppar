export RUST_BACKTRACE=full
export RUSTFLAGS=-g

cargo test -- --nocapture
cargo run --release --bin fuzzy --features=fuzzing # thread-safe
cargo run --release --bin fuzzy --features=fuzzing -- --rc

# (cd rc; cargo test -- --nocapture)
