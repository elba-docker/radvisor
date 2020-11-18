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
	rustlang/rust:latest \
	/bin/bash -c 'cd /opt && make compile OUT_DIR=/opt BUILD_TARGET=$(BUILD_TARGET) "FEATURES=$(FEATURES)"'

# Compiles the project via `cargo build`
compile:
	cargo build --release --bins \
	--target $(BUILD_TARGET) \
	--no-default-features \
	--features "$(FEATURES)" \
	&& cp ./target/$(BUILD_TARGET)/release/radvisor $(OUT_DIR)/radvisor
