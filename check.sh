cargo test -- --nocapture
cargo run --release --bin fuzzy --features=fuzzy # thread-safe
cargo run --release --bin fuzzy --features=fuzzy -- --rc

# (cd rc; cargo test -- --nocapture)
