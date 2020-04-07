.DEFAULT_GOAL := docker

BUILD_TARGET?=x86_64-unknown-linux-gnu
OUT_DIR?=$(shell pwd)

check: docker-exists
docker-exists: ; @which docker > /dev/null

# Builds the project in a Docker container
docker: check
	docker run --rm \
	-v $(shell pwd):/opt \
	rustlang/rust:nightly \
	/bin/bash -c 'cd /opt && make compile OUT_DIR=/opt BUILD_TARGET=$(BUILD_TARGET)'

# Compiles the project via `cargo build`
# Enable unstable-options to use `--out-dir`
compile:
	cargo build --release --bins \
	-Z unstable-options \
	--out-dir $(OUT_DIR) \
	--target $(BUILD_TARGET)

# Enable static OpenSSL linking on Windows
windows: export OPENSSL_STATIC = 1
windows: export RUSTFLAGS = -Ctarget-feature=+crt-static

windows:
	cargo build \
	-Z features=itarget

