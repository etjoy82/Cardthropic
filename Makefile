.PHONY: gate gate-strict shell-lint shell-lint-strict doctor ci-local ci-nightly

gate:
	@scripts/release/maintainer-gate.sh

gate-strict:
	@scripts/release/maintainer-gate.sh --strict-tools

shell-lint:
	@scripts/release/lint-shell.sh

shell-lint-strict:
	@scripts/release/lint-shell.sh --strict-tools

doctor:
	@just doctor

ci-local:
	@just ci-local

ci-nightly:
	@just ci-nightly
