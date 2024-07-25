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

pretty:
	prettier . --write --print-width 80 --prose-wrap always


.PHONY: ci build test check lint fix fmt
