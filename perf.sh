#! /usr/bin/env bash

exec > $1
exec 2>&1

set -o xtrace

PERF=$HOME/.cargo/target/release/perf

# regular benchmark
date; time cargo +nightly bench -- --nocapture || exit $?
date; time cargo +stable bench -- --nocapture || exit $?

LOADS=100000
OPS=10000
# invoke perf binary
date; time cargo +nightly run --release --bin perf --features=perf -- --loads $LOADS --ops $OPS || exit $?
# invoke perf binary, with valgrid
date; valgrind --leak-check=full --show-leak-kinds=all --track-origins=yes $PERF --loads 10000 --ops $OPS || exit $?
