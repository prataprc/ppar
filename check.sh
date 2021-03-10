#! /usr/bin/env bash

export RUST_BACKTRACE=full
export RUSTFLAGS=-g
exec > check.out
exec 2>&1

set -o xtrace

exec_prg() {
    for i in {0..10};
    do
        cargo +nightly test -- --nocapture || exit $?
        cargo +nightly test --release -- --nocapture || exit $?
        cargo +stable test -- --nocapture || exit $?
        cargo +stable test --release -- --nocapture || exit $?
        cargo +nightly bench -- --nocapture || exit $?
        cargo +stable bench -- --nocapture || exit $?
    done
}

exec_prg
