.PHONY: run build install clean help

all: run

run:
	cargo run --release

build:
	cargo build --release

install:
	cargo install --path .

clean:
	cargo clean

help:
	@echo "tbook - Terminal E-book Reader"
	@echo ""
	@echo "Usage:"
	@echo "  make        - Build and run the app in release mode"
	@echo "  make build  - Build the release binary"
	@echo "  make install - Install the binary to your cargo bin path"
	@echo "  make clean  - Remove build artifacts"
