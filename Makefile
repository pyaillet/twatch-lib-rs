CUSTOM_RUSTC = /Users/pyaillet/Projets/esp32/rust-xtensa
RUST_BACKTRACE = 1 
XARGO_RUST_SRC = $(CUSTOM_RUSTC)/library # or /src for an older compiler
RUSTC = $(CUSTOM_RUSTC)/build/x86_64-apple-darwin/stage2/bin/rustc
RUSTDOC = $(CUSTOM_RUSTC)/build/x86_64-apple-darwin/stage2/bin/rustdoc
FEATURES = "xtensa-lx-rt/lx6,xtensa-lx/lx6,esp32,esp32-hal"

.PHONY: build
build:
	cargo xbuild --features=$(FEATURES)
