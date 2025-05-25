PORTS := 8088 8443 6192

default: dev

dev:
	@echo "Killing ports: $(PORTS)"
	@for port in $(PORTS); do \
		kill -9 $$(lsof -t -i :$$port) 2>/dev/null || true; \
	done
	RUST_BACKTRACE=1 RUST_LOG="info,warn,debug" cargo watch -w crates -w examples -w proto -q -c -x "run -- run --config ./examples/config.yaml"

build:
	cargo build --release
