# Cardthropic Flatpak Repo Toolkit

These scripts publish Cardthropic as a Flatpak repository (OSTree) on Codeberg
Pages, which is the reliable way to surface AppStream metadata (license,
screenshots, releases) in GNOME Software.

## What You Get

- `master.sh`: one-command publish pipeline with Cardthropic defaults.
- `init-codeberg-pages.sh`: clone/create a `pages` branch checkout for repo hosting.
- `publish-codeberg-pages.sh`: sync local `build-repo/` into Pages checkout and push.
- `make-flatpakrepo.sh`: generate a `.flatpakrepo` file users can open directly.
- `add-test-remote.sh`: add/update a remote locally and install from it.
- `verify-appstream.sh`: inspect repo AppStream branch for license/screenshot entries.

## One Command (Recommended)

```bash
scripts/flatpak-repo/master.sh
```

Defaults baked in:

- Codeberg repo: `https://codeberg.org/emviolet/Cardthropic-flatpak.git`
- Pages URL: `https://emviolet.codeberg.page/Cardthropic-flatpak/`
- Local checkout: `$HOME/Projects/Cardthropic-flatpak`
- Test remote name: `cardthropic`
- Output descriptor: `./cardthropic.flatpakrepo`

Useful variants:

```bash
# Reuse existing build-repo (skip rebuilding)
scripts/flatpak-repo/master.sh --skip-bundle

# Publish only (skip local test install)
scripts/flatpak-repo/master.sh --skip-test-remote

# Preview publish actions without executing them
scripts/flatpak-repo/master.sh --dry-run
```

## Typical Release Flow

1. Build Flatpak repo payload:

```bash
scripts/flatpak/bundle.sh
```

2. Initialize (one-time) Codeberg Pages checkout:

```bash
scripts/flatpak-repo/init-codeberg-pages.sh \
  --repo-url "https://codeberg.org/emviolet/Cardthropic-flatpak.git" \
  --checkout-dir "$HOME/Projects/Cardthropic-flatpak"
```

3. Publish current `build-repo/`:

```bash
scripts/flatpak-repo/publish-codeberg-pages.sh \
  --checkout-dir "$HOME/Projects/Cardthropic-flatpak"
```

4. Generate `.flatpakrepo` descriptor:

```bash
scripts/flatpak-repo/make-flatpakrepo.sh \
  --base-url "https://emviolet.codeberg.page/Cardthropic-flatpak/" \
  --out "$HOME/Projects/Cardthropic/cardthropic.flatpakrepo"
```

5. Add remote + test install on host:

```bash
scripts/flatpak-repo/add-test-remote.sh \
  --remote cardthropic \
  --url "https://emviolet.codeberg.page/Cardthropic-flatpak/"
```

6. Verify repo AppStream metadata includes license/screenshot:

```bash
scripts/flatpak-repo/verify-appstream.sh --repo "$HOME/Projects/Cardthropic/build-repo"
```

If needed, pass architecture explicitly:

```bash
scripts/flatpak-repo/verify-appstream.sh --repo "$HOME/Projects/Cardthropic/build-repo" --arch x86_64
```

## Notes

- These scripts assume your Flatpak payload exists at `build-repo/`.
- No network permissions are added to the app itself.
- Screenshot visibility in GNOME Software is best when installed from a remote
  repo (not just sideloading a single `.flatpak` file).
