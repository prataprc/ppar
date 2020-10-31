cargo bench
cargo bench --features=ppar-rc
cargo run --bin perf --features=perf --release -- --leaf-size 1024
cargo run --bin perf --features=perf,ppar-rc --release -- --leaf-size 1024
