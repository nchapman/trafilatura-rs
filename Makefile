CARGO       ?= cargo
UNIFFI_DIR  := uniffi
TARGET_DIR  := $(UNIFFI_DIR)/target
RELEASE_DIR := $(TARGET_DIR)/release

# Detect library extension
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
  CDYLIB_EXT  := dylib
else
  CDYLIB_EXT  := so
endif

CDYLIB  := $(RELEASE_DIR)/libtrafilatura_uniffi.$(CDYLIB_EXT)

# --- Prerequisite checks ---

define require
  $(if $(shell which $(1) 2>/dev/null),,$(error "$(1)" not found — install it first))
endef

# --- Cargo build ---

.PHONY: cargo-build
cargo-build:
	$(CARGO) build --manifest-path $(UNIFFI_DIR)/Cargo.toml --release

# --- uniffi-bindgen ---

BINDGEN := $(CARGO) run --manifest-path $(UNIFFI_DIR)/Cargo.toml --features cli --bin uniffi-bindgen --

GENERATED_DIR := $(UNIFFI_DIR)/generated

$(GENERATED_DIR)/python: cargo-build
	$(BINDGEN) generate --library $(CDYLIB) --language python --out-dir $@

$(GENERATED_DIR)/swift: cargo-build
	$(BINDGEN) generate --library $(CDYLIB) --language swift --out-dir $@

$(GENERATED_DIR)/kotlin: cargo-build
	$(BINDGEN) generate --library $(CDYLIB) --language kotlin --out-dir $@

# --- Python ---

VENV := .venv
PIP := $(VENV)/bin/pip

$(VENV):
	$(call require,python3)
	python3 -m venv $(VENV)
	$(PIP) install --upgrade pip

.PHONY: setup-python
setup-python: $(VENV)
	$(PIP) install -r requirements-dev.txt

# Python uses maturin (bindgen implicit); Swift/Kotlin use explicit uniffi-bindgen generate
.PHONY: build-python
build-python: $(VENV)
	$(call require,python3)
	$(VENV)/bin/maturin develop --manifest-path $(UNIFFI_DIR)/Cargo.toml --release

.PHONY: test-python
test-python: build-python
	$(VENV)/bin/pytest tests/bindings/python/ -v

# --- Swift ---

SWIFT_TEST_DIR := tests/bindings/swift
SWIFT_SRC_DIR  := $(SWIFT_TEST_DIR)/Sources/trafilatura_uniffiFFI

.PHONY: build-swift
build-swift: $(GENERATED_DIR)/swift cargo-build
	$(call require,swift)
	cp $(GENERATED_DIR)/swift/trafilatura_uniffiFFI.h $(SWIFT_SRC_DIR)/
	cp $(GENERATED_DIR)/swift/trafilatura_uniffiFFI.modulemap $(SWIFT_SRC_DIR)/module.modulemap
	mkdir -p $(SWIFT_TEST_DIR)/Sources/Trafilatura
	cp $(GENERATED_DIR)/swift/trafilatura_uniffi.swift \
		$(SWIFT_TEST_DIR)/Sources/Trafilatura/trafilatura_uniffi.swift

.PHONY: test-swift
test-swift: build-swift
	cd $(SWIFT_TEST_DIR) && \
		swift test \
			-Xlinker -L../../../$(RELEASE_DIR) \
			-Xlinker -ltrafilatura_uniffi

# --- Kotlin ---

KOTLIN_TEST_DIR := tests/bindings/kotlin
KOTLIN_GEN_DIR  := $(KOTLIN_TEST_DIR)/src/main/kotlin

.PHONY: build-kotlin
build-kotlin: $(GENERATED_DIR)/kotlin cargo-build
	$(call require,java)
	mkdir -p $(KOTLIN_GEN_DIR)
	cp -r $(GENERATED_DIR)/kotlin/uniffi $(KOTLIN_GEN_DIR)/

.PHONY: test-kotlin
test-kotlin: build-kotlin
	cd $(KOTLIN_TEST_DIR) && ./gradlew test

# --- Aggregate ---

.PHONY: test-bindings
test-bindings: test-python test-swift test-kotlin

.PHONY: clean
clean:
	rm -rf $(GENERATED_DIR)
	rm -rf $(VENV)
	rm -rf $(SWIFT_TEST_DIR)/.build
	rm -rf $(KOTLIN_TEST_DIR)/build $(KOTLIN_TEST_DIR)/.gradle
	cd $(UNIFFI_DIR) && $(CARGO) clean
