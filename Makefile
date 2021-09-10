build:
	# ... build ...
	cargo +nightly build
	cargo +stable build
	# ... test ...
	cargo +nightly test --no-run
	cargo +stable test --no-run
	# ... bench ...
	cargo +nightly bench --no-run
	# ... bins ...
	cargo +nightly build --release --bin perf --features=perf
	cargo +stable build --release --bin perf --features=perf
	# ... doc ...
	cargo +nightly doc
	cargo +stable doc
	# ... meta commands ...
	cargo +nightly clippy --all-targets --all-features

test:
	# ... test ...
	cargo +nightly test
	# TODO: cargo +stable test --no-run

bench:
	# ... test ...
	cargo +nightly bench
	# TODO: cargo +stable test --no-run

flamegraph:
	cargo flamegraph --features=perf --bin=perf -- --load 100000 --ops 10000

prepare:
	check.sh check.out
	perf.sh perf.out

clean:
	cargo clean
	rm -f check.out perf.out flamegraph.svg perf.data perf.data.old

