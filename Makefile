.PHONY: gate gate-strict shell-lint shell-lint-strict doctor ci-local ci-nightly post-release post-release-offline

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

post-release:
	@if [ -n "$(VERSION)" ]; then \
		scripts/release/post-release-check.sh --version "$(VERSION)"; \
	else \
		scripts/release/post-release-check.sh; \
	fi

post-release-offline:
	@if [ -n "$(VERSION)" ]; then \
		scripts/release/post-release-check.sh --version "$(VERSION)" --offline --skip-flatpak-checkout; \
	else \
		scripts/release/post-release-check.sh --offline --skip-flatpak-checkout; \
	fi
