VERSION ?= dev

.PHONY: build test lint check

build:
	FRONT_VERSION=$(VERSION) cargo build --release
	cp target/release/front ./front

test:
	cargo test

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings

check: lint test
