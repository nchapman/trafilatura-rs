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
DART_BINDGEN := $(CARGO) run --manifest-path $(UNIFFI_DIR)/Cargo.toml --features dart-cli --bin uniffi-bindgen-dart --
JS_BINDGEN := $(CARGO) run --manifest-path $(UNIFFI_DIR)/Cargo.toml --features js-cli --bin uniffi-bindgen-js --
CS_BINDGEN := uniffi-bindgen-cs

GENERATED_DIR := $(UNIFFI_DIR)/generated

$(GENERATED_DIR)/ruby: cargo-build
	$(BINDGEN) generate --library $(CDYLIB) --language ruby --out-dir $@

$(GENERATED_DIR)/swift: cargo-build
	$(BINDGEN) generate --library $(CDYLIB) --language swift --out-dir $@

$(GENERATED_DIR)/kotlin: cargo-build
	$(BINDGEN) generate --library $(CDYLIB) --language kotlin --out-dir $@

$(GENERATED_DIR)/dart: cargo-build
	$(DART_BINDGEN) generate --library $(CDYLIB) --out-dir $@ --crate trafilatura_uniffi

# --- WASM build ---

WASM_DIR     := $(UNIFFI_DIR)/wasm
WASM_TARGET  := $(WASM_DIR)/target/wasm32-unknown-unknown/release/trafilatura_uniffi.wasm

.PHONY: cargo-build-wasm
cargo-build-wasm:
	$(CARGO) build --manifest-path $(WASM_DIR)/Cargo.toml \
		--target wasm32-unknown-unknown --release

$(GENERATED_DIR)/js: cargo-build-wasm
	$(JS_BINDGEN) generate --out-dir $@ $(WASM_TARGET)
	@# Patch loadWasm to auto-stub any WASM imports (e.g. wasm-bindgen glue
	@# compiled into chrono that is never called at runtime).
	perl -i -pe 's/return WebAssembly\.instantiate\(bytes\)/return WebAssembly.instantiate(bytes, await _stubImports(bytes))/' $@/uniffi_runtime.ts
	perl -i -pe 's/return WebAssembly\.instantiate\(await resp\.arrayBuffer\(\)\)/return WebAssembly.instantiate(await resp.arrayBuffer(), await _stubImports(await resp.clone().arrayBuffer()))/' $@/uniffi_runtime.ts
	@# Insert the _stubImports helper before the loadWasm function
	perl -i -e '$$inserted=0; while(<>){if(!$$inserted && /^async function loadWasm/){print "async function _stubImports(buf: ArrayBuffer | Uint8Array): Promise<WebAssembly.Imports> {\n  const mod = await WebAssembly.compile(buf instanceof ArrayBuffer ? buf : buf.buffer);\n  const imports: WebAssembly.Imports = {};\n  for (const { module, name, kind } of WebAssembly.Module.imports(mod)) {\n    imports[module] ??= {};\n    if (kind === '\''function'\'') (imports[module] as Record<string,unknown>)[name] = () => {};\n    else if (kind === '\''global'\'') (imports[module] as Record<string,unknown>)[name] = new WebAssembly.Global({ value: '\''i32'\'', mutable: true }, 0);\n    else if (kind === '\''table'\'') (imports[module] as Record<string,unknown>)[name] = new WebAssembly.Table({ initial: 0, element: '\''anyfunc'\'' });\n  }\n  return imports;\n}\n\n"; $$inserted=1;} print;}' $@/uniffi_runtime.ts
	@grep -q '_stubImports' $@/uniffi_runtime.ts || \
		(echo "ERROR: _stubImports patch failed — uniffi_runtime.ts format may have changed" && exit 1)

$(GENERATED_DIR)/cs: cargo-build
	$(call require,$(CS_BINDGEN))
	$(CS_BINDGEN) --library $(CDYLIB) --out-dir $@
	@# Patch contract version: uniffi-bindgen-cs v0.10 emits contract 29 (uniffi 0.29)
	@# but this project uses uniffi 0.31 (contract 30). The ABI is compatible
	@# because we only use free functions and value types (no callback interfaces).
	perl -i -pe 's/\bif \(29 != /if (30 != /; s/expected version `29`/expected version `30`/' $@/trafilatura_uniffi.cs
	@grep -q 'if (30 != ' $@/trafilatura_uniffi.cs || \
		(echo "ERROR: contract version patch failed — check uniffi-bindgen-cs version" && exit 1)

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

# --- Ruby ---

RUBY_TEST_DIR := tests/bindings/ruby

.PHONY: build-ruby
build-ruby: $(GENERATED_DIR)/ruby cargo-build
	$(call require,ruby)
	$(call require,bundle)
	mkdir -p $(RUBY_TEST_DIR)/lib
	cp $(GENERATED_DIR)/ruby/trafilatura_uniffi.rb $(RUBY_TEST_DIR)/lib/
	cd $(RUBY_TEST_DIR) && bundle install

.PHONY: test-ruby
test-ruby: build-ruby
	cd $(RUBY_TEST_DIR) && bundle exec rake test

# --- Dart ---

DART_TEST_DIR := tests/bindings/dart

.PHONY: build-dart
build-dart: $(GENERATED_DIR)/dart cargo-build
	$(call require,dart)
	mkdir -p $(DART_TEST_DIR)/lib
	cp $(GENERATED_DIR)/dart/trafilatura_uniffi.dart $(DART_TEST_DIR)/lib/
	cd $(DART_TEST_DIR) && dart pub get

.PHONY: test-dart
test-dart: build-dart
	cd $(DART_TEST_DIR) && dart test -r expanded

# --- C# ---

CS_TEST_DIR := tests/bindings/cs

.PHONY: build-cs
build-cs: $(GENERATED_DIR)/cs cargo-build
	$(call require,dotnet)
	mkdir -p $(CS_TEST_DIR)/lib
	cp $(GENERATED_DIR)/cs/trafilatura_uniffi.cs $(CS_TEST_DIR)/lib/
	cd $(CS_TEST_DIR) && dotnet build
	@# Copy native library next to test binary so DllImport can find it
	cp $(CDYLIB) $(CS_TEST_DIR)/bin/Debug/net8.0/

.PHONY: test-cs
test-cs: build-cs
	cd $(CS_TEST_DIR) && dotnet test --no-build

# --- JavaScript/TypeScript ---

JS_TEST_DIR := tests/bindings/js

.PHONY: build-js
build-js: $(GENERATED_DIR)/js
	$(call require,node)
	$(call require,pnpm)
	mkdir -p $(JS_TEST_DIR)/lib
	cp $(GENERATED_DIR)/js/trafilatura_uniffi.ts $(JS_TEST_DIR)/lib/
	cp $(GENERATED_DIR)/js/trafilatura_uniffi.wasm $(JS_TEST_DIR)/lib/
	cp $(GENERATED_DIR)/js/uniffi_runtime.ts $(JS_TEST_DIR)/lib/
	cd $(JS_TEST_DIR) && pnpm install --frozen-lockfile

.PHONY: test-js
test-js: build-js
	cd $(JS_TEST_DIR) && pnpm test

# --- Aggregate ---

.PHONY: test-bindings
test-bindings: test-swift test-kotlin test-ruby test-dart test-cs test-js

.PHONY: clean
clean:
	rm -rf $(GENERATED_DIR)
	rm -rf $(SWIFT_TEST_DIR)/.build
	rm -rf $(KOTLIN_TEST_DIR)/build $(KOTLIN_TEST_DIR)/.gradle
	rm -rf $(RUBY_TEST_DIR)/vendor $(RUBY_TEST_DIR)/.bundle $(RUBY_TEST_DIR)/Gemfile.lock
	rm -rf $(DART_TEST_DIR)/.dart_tool $(DART_TEST_DIR)/pubspec.lock
	rm -rf $(CS_TEST_DIR)/bin $(CS_TEST_DIR)/obj $(CS_TEST_DIR)/lib
	rm -rf $(JS_TEST_DIR)/node_modules $(JS_TEST_DIR)/lib
	cd $(UNIFFI_DIR) && $(CARGO) clean
