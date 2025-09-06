.PHONY: verify-comments lint coverage interop test-golden fmt clippy doc test build build-maxspeed version clean build-all package release

# Optional compatibility mapping:
# If user passes UPSTREAM/OFFICIAL, map them to RSYNC_UPSTREAM_VER/OFFICIAL_BUILD unless already set.
RSYNC_UPSTREAM_VER ?= $(UPSTREAM)
OFFICIAL_BUILD     ?= $(OFFICIAL)
BUILD_REVISION     ?= $(shell git rev-parse --short=12 HEAD)

VERIFY_COMMENT_FILES := $(shell git ls-files '*.rs')

verify-comments:
	@bash scripts/check-comments.sh $(VERIFY_COMMENT_FILES)

lint:
	cargo fmt --all --check
	if rustc --version | grep -q nightly; then \
	cargo clippy --all-targets --all-features -- -D warnings; \
	else \
	cargo clippy --all-targets -- -D warnings; \
	echo "note: AVX-512 linting requires a nightly toolchain"; \
	fi

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

doc:
	cargo doc --no-deps --all-features

test:
	env LC_ALL=C LANG=C COLUMNS=80 TZ=UTC cargo nextest run --workspace --no-fail-fast
	env LC_ALL=C LANG=C COLUMNS=80 TZ=UTC cargo nextest run --workspace --no-fail-fast --features "cli nightly"

# Run the test suite without ACL support for environments lacking libacl.
test-noacl:
	env LC_ALL=C LANG=C COLUMNS=80 TZ=UTC cargo nextest run --workspace --no-fail-fast --no-default-features --features "no-acl"
	env LC_ALL=C LANG=C COLUMNS=80 TZ=UTC cargo nextest run --workspace --no-fail-fast --no-default-features --features "cli nightly no-acl"

coverage:
	cargo llvm-cov nextest --workspace --features "cli nightly" --doctests \
	--fail-under-lines 95 --fail-under-functions 95 -- --no-fail-fast

interop:
	@bash tests/interop/run_matrix.sh

test-golden:
	env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" cargo build --quiet -p oc-rsync --bin oc-rsync
	@set -euo pipefail; \
	for script in tests/golden/cli_parity/*.sh; do \
		echo "Running $$script"; \
		bash "$$script"; \
	done; \
	echo "Running tests/filter_rule_precedence.sh"; \
	bash tests/filter_rule_precedence.sh; \
	echo "Running tests/partial_transfer_resume.sh"; \
	bash tests/partial_transfer_resume.sh; \
	echo "Running tests/partial_dir_transfer_resume.sh"; \
        bash tests/partial_dir_transfer_resume.sh

refresh-upstream-goldens:
	@bash scripts/refresh_upstream_goldens.sh

# Standard release build; honors env vars if provided:
#   make build RSYNC_UPSTREAM_VER=3.4.1 OFFICIAL_BUILD=1
# or legacy:
#   make build UPSTREAM=3.4.1 OFFICIAL=1
build:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD)"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsync --release
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --release

# Max performance build (uses your [profile.maxspeed])
build-maxspeed:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD) [maxspeed]"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsync --profile maxspeed --release
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --profile maxspeed --release

TARGETS := aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu x86_64-pc-windows-gnu

.PHONY: $(addprefix build-,$(TARGETS)) $(addprefix build-maxspeed-,$(TARGETS))

build-%:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD) target=$*"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsync --release --target $*
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --release --target $*

build-maxspeed-%:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD) [maxspeed] target=$*"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsync --profile maxspeed --release --target $*
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --profile maxspeed --release --target $*

# Show version from the release artifact built above
version: build
	./target/release/oc-rsync --version
	./target/release/oc-rsyncd --version

clean: ; @env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" cargo clean; rm -rf dist; rm -f oc-rsync-* oc-rsyncd-*

build-all: ; set -e; for target in $(TARGETS); do echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD) target=$$target"; env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" cargo build -p oc-rsync --bin oc-rsync --bin oc-rsyncd --release --target $$target; done

package: ; env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" bash -c 'set -e; mkdir -p dist; for target in $(TARGETS); do for bin in oc-rsync oc-rsyncd; do ext=tar.gz; exe=$$bin; case $$target in *windows*) ext=zip; exe=$$bin.exe ;; esac; archive="dist/$$bin-$$target-$(RSYNC_UPSTREAM_VER)-$(BUILD_REVISION).$$ext"; if [ $$ext = zip ]; then (cd target/$$target/release && zip -j "../../../$$archive" $$exe); else tar -C target/$$target/release -czf $$archive $$exe; fi; sha256sum $$archive > $$archive.sha256; cargo sbom --output $$archive.sbom 2>/dev/null || true; done; done'

release: ; env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" $(MAKE) lint; env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" $(MAKE) test; env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" $(MAKE) coverage; env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" $(MAKE) build-all; env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" $(MAKE) package

