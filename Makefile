PORTS := 8088 8443 6192

default: dev

dev:
	@echo "Killing ports: $(PORTS)"
	@for port in $(PORTS); do \
		kill -9 $$(lsof -t -i :$$port) 2>/dev/null || true; \
	done
	RUST_BACKTRACE=1 cargo watch -w crates -w examples -w proto -w sdk -q -c -s "make build-examples && cargo run -- run --config ./examples/config.yaml"

dev-debug:
	RUST_LOG="info,warn,debug" make dev

generate:
	flatc --rust -o sdk/rust/src/fbs proto/dispatcher.fbs
	flatc --rust -o sdk/rust/src/fbs proto/http_context.fbs
	flatc --go -o sdk/go/fbs proto/dispatcher.fbs
	flatc --go -o sdk/go/fbs proto/http_context.fbs

build-examples:
	cd examples/go && go build -buildmode=c-shared -o ./../../target/examples/go/plugin_sdk.so

build:
	cargo build --release
