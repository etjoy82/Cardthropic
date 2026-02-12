# Cardthropic Release Playbook

This is the repeatable process for publishing a new Cardthropic release (source + Flatpak bundle).

## 1) Bump Version

Update version strings:

- `Cargo.toml` (`[package].version`)
- `meson.build` (`project(... version: 'x.y.z' ...)`)
- `README.md` current version + changelog section
- `data/io.codeberg.emviolet.cardthropic.metainfo.xml.in` release entry

## 2) Validate Locally

```bash
cargo fmt
cargo check
cargo test -q
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
git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
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

## One-Command Path (Automated)

After version/changelog updates are done, run:

```bash
scripts/flatpak/release.sh
```

This performs:

- `cargo fmt`
- `cargo check`
- `cargo test -q`
- Flatpak build/install
- bundle creation (`cardthropic.flatpak`)
- checksum generation (`SHA256SUMS`)
