build:
	# ... build ...
	cargo +nightly build
	cargo +stable build
	# ... test ...
	cargo +nightly test --no-run
	cargo +stable test --no-run
	# ... bench ...
	cargo +nightly bench --no-run
	# cargo +stable bench --no-run
	# ... bins ...
	cargo +nightly build --release --bin perf --features=perf
	cargo +stable build --release --bin perf --features=perf
	# ... doc ...
	cargo +nightly doc
	cargo +stable doc
	# ... meta commands ...
	cargo +nightly clippy --all-targets --all-features
flamegraph:
	cargo flamegraph --features=perf --bin=perf -- --load 100000 --ops 10000
prepare:
	check.sh
	perf.sh
clean:
	rm -f check.out perf.out flamegraph.svg perf.data perf.data.old

