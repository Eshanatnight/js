.PHONY: all build check clippy fmt fmt-check clean test

all: fmt clippy build rel

build:
	cargo build

check:
	cargo check

clippy:
	cargo clippy -- -W clippy::all -W clippy::correctness -W clippy::complexity -W clippy::perf -W clippy::style -D warnings

clippy-pedantic:
	cargo clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::restriction -W clippy::correctness -W clippy::complexity -W clippy::perf -W clippy::style -W clippy::all  -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

clean:
	cargo clean

test:
	cargo test

release:
	cargo build --release

rel:
	cargo build --release
