default: dev

dev:
	RUST_BACKTRACE=1 RUST_LOG="info,warn,debug" cargo watch -q -c -x "run -- run --config ./examples/config.yaml"

build:
	cargo build --release
