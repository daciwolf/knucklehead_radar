.PHONY: all build check test fmt fmt-check clippy run clean

all: check test build

build:
	cargo build

check:
	cargo check

test:
	cargo test

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

run:
	cargo run

clean:
	cargo clean
