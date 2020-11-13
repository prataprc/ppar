export RUST_BACKTRACE=full
export RUSTFLAGS=-g

cargo test -- --nocapture > test.out 2>&1
cargo run --release --bin fuzzy --features=fuzzing -- --threads 4 --load 1000000 --ops 100000 > arc.check 2>&1
cargo run --release --bin fuzzy --features=fuzzing -- --threads 1 --load 1000000 --ops 100000 > rc.check 2>&1
