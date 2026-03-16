.PHONY: build test lint fmt clean check

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check:
	cargo fmt -- --check
	cargo clippy -- -D warnings
	cargo test

clean:
	cargo clean
