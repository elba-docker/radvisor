.DEFAULT_GOAL := docker

BUILD_TARGET?=x86_64-unknown-linux-gnu
FEATURES?=docker kubernetes
OUT_DIR?=$(shell pwd)

check: docker-exists
docker-exists: ; @which docker > /dev/null

# Builds the project in a Docker container
docker: check
	docker run --rm \
	-v $(shell pwd):/opt \
	rust:latest \
	/bin/bash -c 'cd /opt && make compile OUT_DIR=/opt BUILD_TARGET=$(BUILD_TARGET) "FEATURES=$(FEATURES)"'

# Compiles the project via `cargo build`
compile:
	cargo build --release --bins \
	--package radvisor \
	--target $(BUILD_TARGET) \
	--no-default-features \
	--features "$(FEATURES)" \
	&& cp ./target/$(BUILD_TARGET)/release/radvisor $(OUT_DIR)/radvisor

# Compiles the toolbox via `cargo build`
compile-toolbox:
	cargo build --release --bins \
	--package radvisor-toolbox \
	--target $(BUILD_TARGET) \
	&& cp ./target/$(BUILD_TARGET)/release/radvisor-toolbox $(OUT_DIR)/radvisor-toolbox

# Compiles the main binary and the toolbox
all: compile compile-toolbox

# Remove compiled files
clean:
	cargo clean
