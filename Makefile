.PHONY: verify-comments lint coverage interop test-golden

verify-comments:
	bash scripts/check-comments.sh

lint:
	cargo fmt --all --check

coverage:
	cargo tarpaulin --all --features blake3

interop:
	bash tests/interop/run_matrix.sh

test-golden:
	cargo build --quiet -p oc-rsync-bin --bin oc-rsync --features blake3
	@set -e; \
	for script in tests/golden/cli_parity/*.sh; do \
		echo "Running $$script"; \
		bash $$script; \
	done; \
	echo "Running tests/filter_rule_precedence.sh"; \
	bash tests/filter_rule_precedence.sh; \
	echo "Running tests/partial_transfer_resume.sh"; \
	bash tests/partial_transfer_resume.sh

