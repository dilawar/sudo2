ci: build test

build:
	cargo build --all-targets

test: build
	cargo test --all-targets

check lint:
	cargo clippy 

fix:
	cargo clippy --fix --allow-dirty

fmt:
	cargo +nightly fmt


.PHONY: ci build test check lint fix fmt
