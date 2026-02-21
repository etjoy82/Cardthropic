# Cardthropic Release Playbook

This is the repeatable process for publishing a new Cardthropic release (source + Flatpak bundle).
Current channel: beta.

Policy:

- This Codeberg repository is a **beta testbed**.
- Scripts under `scripts/` are **maintainer-only operational tooling** for this project and workflow.
- CI must pass `scripts/release/maintainer-gate.sh --strict-tools` on push/PR before merge.
- Release tags must be signed.

## 1) Bump Version

Update version strings:

- `Cargo.toml` (`[package].version`)
- `meson.build` (`project(... version: 'x.y.z[-prerelease]' ...)`)
- `README.md` current version
- `src/config.rs` (`VERSION` constant for app About/version display)
- `CHANGELOG.md` latest release entry
- `data/io.codeberg.emviolet.cardthropic.metainfo.xml.in` release entry

Then verify consistency:

```bash
scripts/release/check-release-consistency.sh
```

Or perform the version bump skeleton automatically:

```bash
scripts/release/bump-version.sh --version X.Y.Z[-PRERELEASE]
```

After writing final release notes, replace placeholders in changelog + AppStream:

```bash
scripts/release/finalize-release-notes.sh --version X.Y.Z[-PRERELEASE] \
  --note "First release note" \
  --note "Second release note"
```

## 2) Validate Locally

```bash
scripts/release/maintainer-gate.sh
```

Shortcut:

```bash
make gate
```

Expanded local quality gate (Rust + security/policy checks via `just`):

```bash
just ci-local
```

This gate enforces:

- shell script policy checks
- release-version consistency checks
- `shellcheck` lint when available (using repo policy in `.shellcheckrc`)
- `cargo fmt --check`, `cargo check`, `cargo test -q`

Fast shell-only preflight:

```bash
scripts/release/lint-shell.sh --strict-tools
```

Shortcut:

```bash
make shell-lint-strict
```

## 3) Ensure Flatpak Tooling Is Ready

```bash
scripts/flatpak/bootstrap.sh
```

## 4) Build and Install Flatpak (Local Test)

```bash
scripts/flatpak/build-install.sh
scripts/flatpak/run.sh
```

Sanity check:

- app launches
- app icon resolves correctly in dock/app grid
- seed tools/menus/robot/wand behavior still works

## 5) Build Distributable Bundle

```bash
scripts/flatpak/bundle.sh
```

Expected output:

- `cardthropic.flatpak`

## 6) Test Bundle Install Path

```bash
scripts/flatpak/install-bundle.sh
flatpak run io.codeberg.emviolet.cardthropic
```

## 7) Commit and Tag

```bash
git add -A
git commit -S -m "release: vX.Y.Z[-PRERELEASE]"
git tag -s vX.Y.Z[-PRERELEASE] -m "Cardthropic vX.Y.Z[-PRERELEASE]"
git push origin main --tags
```

## 8) Publish on Codeberg

Create a new release and upload:

- `cardthropic.flatpak`
- optional checksums file (`SHA256SUMS`)

For full GNOME Software metadata (license/screenshots), publish as a Flatpak
repository too:

```bash
scripts/flatpak-repo/init-codeberg-pages.sh --repo-url "https://codeberg.org/<user>/<repo>.git"
scripts/flatpak-repo/publish-codeberg-pages.sh
scripts/flatpak-repo/make-flatpakrepo.sh --base-url "https://<user>.codeberg.page/<repo>/"
```

Toolkit docs:

- `scripts/flatpak-repo/README.md`

Suggested install instructions for release notes:

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install ./cardthropic.flatpak
flatpak run io.codeberg.emviolet.cardthropic
```

## 9) Optional: Generate SHA256

```bash
sha256sum cardthropic.flatpak > SHA256SUMS
```

## 10) Post-Release Verification (One Command)

Run this after pushing release commits/tags to verify both source remotes and
the optional Flatpak Pages checkout are in sync:

```bash
scripts/release/post-release-check.sh --version X.Y.Z[-PRERELEASE]
```

Useful flags:

```bash
# Use cached remote refs only (no network calls)
scripts/release/post-release-check.sh --version X.Y.Z[-PRERELEASE] --offline

# Skip Cardthropic-flatpak checkout verification
scripts/release/post-release-check.sh --version X.Y.Z[-PRERELEASE] --skip-flatpak-checkout
```

## One-Command Path (Automated)

After version/changelog updates are done, run:

```bash
scripts/flatpak/release.sh
```

This performs:

- `scripts/release/maintainer-gate.sh`
- Flatpak build/install
- bundle creation (`cardthropic.flatpak`)
- checksum generation (`SHA256SUMS`)

## Hotfix Helper Flow

For hotfix releases, use:

```bash
scripts/release/hotfix-flow.sh --version X.Y.Z[-PRERELEASE]
```

Behavior:

- Runs local checks
- Builds Flatpak bundle (`scripts/flatpak/bundle.sh`)
- Verifies built repo AppStream metadata (`scripts/flatpak-repo/verify-appstream.sh`)
- Prints the exact git commands to run next

If you need to skip bundle + repo verification:

```bash
scripts/release/hotfix-flow.sh --version X.Y.Z[-PRERELEASE] --skip-bundle
```
