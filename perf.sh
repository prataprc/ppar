cargo bench
cargo run --release --bin perf --features=perf -- --leaf-size 1024
cargo run --release --bin perf --features=perf,ppar-rc -- --leaf-size 1024

# (cd rc; cargo bench)
