.PHONY: verify-comments lint coverage interop test-golden fmt clippy doc test build build-maxspeed version

# Optional compatibility mapping:
# If user passes UPSTREAM/OFFICIAL, map them to RSYNC_UPSTREAM_VER/OFFICIAL_BUILD unless already set.
RSYNC_UPSTREAM_VER ?= $(UPSTREAM)
OFFICIAL_BUILD     ?= $(OFFICIAL)
BUILD_REVISION     ?= $(shell git rev-parse HEAD)

VERIFY_COMMENT_FILES := $(shell git ls-files '*.rs')

verify-comments:
	@bash scripts/check-comments.sh $(VERIFY_COMMENT_FILES)

lint:
	cargo fmt --all --check

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

doc:
	cargo doc --no-deps --all-features

test:
	cargo test
	cargo test --all-features

coverage:
	cargo llvm-cov --workspace --doctests \
		--fail-under-lines 95 --fail-under-functions 95

interop:
	bash tests/interop/run_matrix.sh

test-golden:
	cargo build --quiet -p oc-rsync-bin --bin oc-rsync
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

# Standard release build; honors env vars if provided:
#   make build RSYNC_UPSTREAM_VER=3.4.1 OFFICIAL_BUILD=1
# or legacy:
#   make build UPSTREAM=3.4.1 OFFICIAL=1
build:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD)"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync-bin --bin oc-rsync --release
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --release

# Max performance build (uses your [profile.maxspeed])
build-maxspeed:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD) [maxspeed]"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync-bin --bin oc-rsync --profile maxspeed --release
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --profile maxspeed --release

TARGETS := aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu x86_64-pc-windows-gnu

.PHONY: $(addprefix build-,$(TARGETS)) $(addprefix build-maxspeed-,$(TARGETS))

build-%:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD) target=$*"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync-bin --bin oc-rsync --release --target $*
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --release --target $*

build-maxspeed-%:
	@echo "RSYNC_UPSTREAM_VER=$(RSYNC_UPSTREAM_VER) BUILD_REVISION=$(BUILD_REVISION) OFFICIAL_BUILD=$(OFFICIAL_BUILD) [maxspeed] target=$*"
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync-bin --bin oc-rsync --profile maxspeed --release --target $*
	@env RSYNC_UPSTREAM_VER="$(RSYNC_UPSTREAM_VER)" BUILD_REVISION="$(BUILD_REVISION)" OFFICIAL_BUILD="$(OFFICIAL_BUILD)" \
	cargo build -p oc-rsync --bin oc-rsyncd --profile maxspeed --release --target $*

# Show version from the release artifact built above
version: build
	./target/release/oc-rsync --version
	./target/release/oc-rsyncd --version

