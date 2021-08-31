#! /usr/bin/env bash

export RUST_BACKTRACE=full
export RUSTFLAGS=-g

exec > $1
exec 2>&1

set -o xtrace

exec_prg() {
    for i in {0..5};
    do
        date; time cargo +nightly test -- --nocapture || exit $?
        date; time cargo +nightly test --release -- --nocapture || exit $?
        date; time cargo +stable test -- --nocapture || exit $?
        date; time cargo +stable test --release -- --nocapture || exit $?
    done
}

exec_prg
