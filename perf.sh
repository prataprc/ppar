exec > perf.out
exec 2>&1

set -o xtrace

PERF=$HOME/.cargo/target/release/perf

date; cargo run --release --bin perf --features=perf -- --loads 100000 --ops 10000 || exit $?
date; valgrind --leak-check=full --show-leak-kinds=all --track-origins=yes $PERF --loads 10000 --ops 10000 || exit $?
