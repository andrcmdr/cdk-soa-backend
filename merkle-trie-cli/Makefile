.PHONY: build test test-viem test-exit-codes clean help

help:
	@echo "Available targets:"
	@echo "  build           - Build all binaries"
	@echo "  test            - Run all tests"
	@echo "  test-viem       - Run viem compatibility tests"
	@echo "  test-exit-codes - Test CLI exit codes"
	@echo "  clean           - Clean build artifacts"

build:
	cargo build --release --bin merkle-cli-viem-compat

test:
	cargo test --all

test-viem:
	cargo test viem_compatibility_tests -- --nocapture

test-exit-codes: build
	@chmod +x test_exit_codes.sh
	./test_exit_codes.sh

clean:
	cargo clean
	rm -f test_data.csv reference.json invalid_*.json output*.json
