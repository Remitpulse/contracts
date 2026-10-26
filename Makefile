export RUSTUP_TOOLCHAIN := $(shell sed -n 's/^channel = //p' rust-toolchain.toml | tr -d '"')
.PHONY: test build optimize deploy-testnet deploy-mainnet fmt check lint ci-clippy shellcheck audit test-wasm checksums verify ci reproducible-build
STELLAR_CLI ?= stellar
COMPONENT ?= core
WASM_DIR ?= target/wasm32v1-none/release
REPRO_DIR ?= target/repro
test:
	cargo test --workspace --locked
	python3 -m unittest discover -s tests -v
build:
	STELLAR_CLI="$(STELLAR_CLI)" ./scripts/build.sh
optimize: build
# Modern Stellar CLI optimizes as part of build. deploy.sh also supports legacy soroban.
deploy-testnet:
	STELLAR_CLI=$(STELLAR_CLI) ./deploy.sh testnet $(COMPONENT)
deploy-mainnet:
	STELLAR_CLI=$(STELLAR_CLI) ./deploy.sh mainnet $(COMPONENT)
fmt:
	cargo fmt --all
# Lint repository code only; dependency lints are excluded via --no-deps.
# No lint exceptions are currently allowed: warnings are denied for the whole
# workspace and all targets. Any future exception must be narrowly scoped
# (e.g. a single #[allow(...)] with a justifying comment) and documented here.
lint:
	cargo clippy --workspace --all-targets --locked --no-deps -- -D warnings
# CI quality gate clippy run: includes dependency lints and denies warnings.
ci-clippy:
	cargo clippy --workspace --all-targets --locked -- -D warnings
# ShellCheck deployment scripts. SC1091 is excluded because sourced files are
# resolved at runtime and not statically analyzable.
shellcheck:
	shellcheck --exclude=SC1091 deploy.sh scripts/build.sh
# Lockfile-based vulnerability scan; consumes Cargo.lock and fails on advisories.
audit:
	cargo audit --file Cargo.lock
check:
	cargo fmt --all -- --check
	cargo test --workspace --locked
	$(MAKE) lint
	$(MAKE) ci-clippy
	$(MAKE) shellcheck
	$(MAKE) audit
	bash -n deploy.sh scripts/build.sh
	python3 -m unittest discover -s tests -v

# Build first: this suite registers the release artifact inside the Soroban VM.
test-wasm:
	cargo build --workspace --locked --release --target wasm32v1-none
	cargo test -p remitpulse-account --locked --features wasm-tests

# Generate the SHA256 manifest for the release WASM binaries. This is the same
# manifest CI publishes alongside the artifacts so operators can verify them.
checksums:
	@set -e; \
	cd "$(WASM_DIR)"; \
	files=$$(ls remitpulse_*.wasm 2>/dev/null || true); \
	if [ -z "$$files" ]; then \
		echo "error: no remitpulse_*.wasm found in $(WASM_DIR); run 'make build' first" >&2; \
		exit 1; \
	fi; \
	sha256sum $$files > SHA256SUMS; \
	echo "wrote $(WASM_DIR)/SHA256SUMS:"; \
	cat SHA256SUMS

# Reproducible build check: build twice into isolated output directories and
# compare checksums of every emitted remitpulse WASM artifact.
reproducible-build:
	rm -rf "$(REPRO_DIR)"
	mkdir -p "$(REPRO_DIR)/build-a" "$(REPRO_DIR)/build-b"
	STELLAR_CLI="$(STELLAR_CLI)" OUT_DIR="$(REPRO_DIR)/build-a" ./scripts/build.sh
	STELLAR_CLI="$(STELLAR_CLI)" OUT_DIR="$(REPRO_DIR)/build-b" ./scripts/build.sh
	@set -e; \
	cd "$(REPRO_DIR)/build-a"; \
	files=$$(find . -type f -name '*remitpulse*.wasm' | sort); \
	if [ -z "$$files" ]; then \
		echo "ERROR: no remitpulse WASM artifacts found in $(REPRO_DIR)/build-a" >&2; \
		exit 1; \
	fi; \
	status=0; \
	for f in $$files; do \
		a="$(REPRO_DIR)/build-a/$$f"; \
		b="$(REPRO_DIR)/build-b/$$f"; \
		if [ ! -f "$$b" ]; then \
			echo "ERROR: missing artifact in second build: $$f" >&2; \
			status=1; \
			continue; \
		fi; \
		ha=$$(sha256sum "$$a" | cut -d' ' -f1); \
		hb=$$(sha256sum "$$b" | cut -d' ' -f1); \
		if [ "$$ha" != "$$hb" ]; then \
			echo "ERROR: checksum mismatch for $$f" >&2; \
			echo "  build-a: $$ha" >&2; \
			echo "  build-b: $$hb" >&2; \
			status=1; \
		else \
			echo "OK: $$f $$ha"; \
		fi; \
	done; \
	exit $$status

verify: check test-wasm checksums
# Full CI quality gate: formatting, tests, clippy (warnings denied), shell
# syntax, audit, WASM tests, checksums, and reproducible builds.
ci: verify reproducible-build
