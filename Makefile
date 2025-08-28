.PHONY: test-golden

test-golden:
	cargo build --quiet --bin rsync-rs
	@set -e; \
	for script in tests/golden/cli_parity/*.sh; do \
		echo "Running $$script"; \
		bash $$script; \
	done
