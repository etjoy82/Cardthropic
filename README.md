# Cardthropic

Cardthropic is a modern GNOME solitaire app built with Rust, GTK4, and Libadwaita.
It currently ships a full Klondike experience, with architecture prepared for more variants.

![Cardthropic Logo](/logo-small.png)

Current version: `0.6.0` (alpha channel)
License: `GPL-3.0-or-later`

![Cardthropic 0.5 screenshot](data/screenshots/cardthropic-0.5-screenshot.png)

## Highlights

- Native GNOME UI with keyboard, mouse, and drag-and-drop play.
- Seed-first gameplay (`🎲`, `🛟`, `W?`, `🔁`, `Go`) with persistent seed history.
- Advanced automation:
  - `🪄` Wave Magic Wand
  - `⚡` Rapid Wand
  - `🤖` Robot Mode
  - `🌀` Cyclone Shuffle
  - `🫣` Peek
- Smart Move modes: `Double Click`, `Single Click`, `Disabled`.
- Draw modes: Deal `1/2/3/4/5`.
- Session resume after restart/crash.
- Actions-per-minute telemetry + in-app APM graph.
- Full theming system:
  - curated built-in presets
  - custom CSS userstyle editor
  - clipboard-only CSS workflows (no filesystem access)

## Built-in Theme Presets

From the `🎨` menu:

- Cardthropic
- Cardthropic Night
- Cardthropic Midnight
- Arcade
- Glass
- Neon
- Noir
- Forest
- CRT
- Terminal
- Minimal Mono
- Custom (opens CSS editor)

## Shortcuts

- `F1` Help
- `F11` Toggle Fullscreen
- `Space` Draw
- `Ctrl+Z` Undo
- `Ctrl+Y` Redo
- `Ctrl+Space` Wave Magic Wand
- `Ctrl+Shift+Space` Rapid Wand
- `F3` Peek
- `F5` Cyclone Shuffle Tableau
- `F6` Robot Mode
- `Ctrl+R` Start Random Deal
- `Ctrl+Shift+R` Start Winnable Deal Search
- `Ctrl+Q` Quit

Custom CSS editor:

- `Ctrl+C` Copy CSS
- `Ctrl+V` Paste CSS
- `Ctrl+Shift+C` Copy Preset + CSS

## Install (Recommended)

Cardthropic is best installed from the official Flatpak repo so GNOME Software can show full metadata (license, releases, screenshots, updates).

Alpha testbed note: this Codeberg distribution is currently an alpha channel. The remote commands below intentionally use an unsigned remote (`--no-gpg-verify`) for this phase.

### Option A: Flatpak remote

```bash
flatpak remote-add --if-not-exists --user --no-gpg-verify cardthropic https://emviolet.codeberg.page/Cardthropic-flatpak/
flatpak update --user --appstream cardthropic
flatpak install --user cardthropic io.codeberg.emviolet.cardthropic
flatpak run io.codeberg.emviolet.cardthropic
```

### Option B: Direct bundle

If you only have `cardthropic.flatpak`:

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install ./cardthropic.flatpak
flatpak run io.codeberg.emviolet.cardthropic
```

### Ubuntu (GNOME Software + Flatpak)

1. Follow the official Flathub setup guide:
   <https://flathub.org/setup/Ubuntu>
2. Install integration packages:

```bash
sudo apt install flatpak gnome-software-plugin-flatpak
```

3. Log out/in (or reboot).
4. Ensure runtime dependency is available:

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install -y flathub org.gnome.Platform//48
```

5. Add Cardthropic remote and install:

```bash
flatpak remote-add --if-not-exists --user --no-gpg-verify cardthropic https://emviolet.codeberg.page/Cardthropic-flatpak/
flatpak update --user --appstream cardthropic
flatpak install --user cardthropic io.codeberg.emviolet.cardthropic
```

Packaging policy: Official builds are Flatpak-only. Native packages (`.deb`, `.rpm`, `snap`) are welcome as community-maintained ports.

## For Developers

Developer tooling policy:

- This Codeberg repository is an **alpha testbed**.
- Shell scripts under `scripts/` are **maintainer-only operational tooling** for this repo/workflow.
- Those scripts are not intended as a stable, third-party public interface.
- CI policy: `.woodpecker.yml` runs `scripts/release/maintainer-gate.sh --strict-tools` on push/PR.

### Contributor workflow (local coding)

```bash
cargo check
cargo run
```

Optional test run:

```bash
cargo test -q
```

### Flatpak local dev workflow (maintainer-oriented)

```bash
scripts/flatpak/bootstrap.sh
scripts/flatpak/build-install.sh
scripts/flatpak/run.sh
```

### Publish/update Flatpak repo (Codeberg Pages, maintainer-only)

```bash
scripts/flatpak-repo/master.sh
```

Preview publish actions without executing:

```bash
scripts/flatpak-repo/master.sh --dry-run
```

### Maintainer quality gate (maintainer-only)

Run this before release/hotfix flows:

```bash
scripts/release/maintainer-gate.sh
```

Shortcut:

```bash
make gate
```

Shell lint policy is repo-pinned in `.shellcheckrc`.

Fast pre-commit shell lint:

```bash
scripts/release/lint-shell.sh --strict-tools
```

Shortcut:

```bash
make shell-lint-strict
```

### Hotfix release helper (maintainer-only)

```bash
scripts/release/hotfix-flow.sh --version X.Y.Z
```

By default this runs checks, builds a bundle, verifies AppStream metadata from `build-repo`, and then prints git commands.

Skip bundle + repo verification only if needed:

```bash
scripts/release/hotfix-flow.sh --version X.Y.Z --skip-bundle
```

### Version bump helper (maintainer-only)

```bash
scripts/release/bump-version.sh --version X.Y.Z
```

### Release note finalize helper (maintainer-only)

```bash
scripts/release/finalize-release-notes.sh --version X.Y.Z \
  --note "First release note" \
  --note "Second release note"
```

## Packaging/Safety Notes

- App ID: `io.codeberg.emviolet.cardthropic`
- Flatpak runtime: `org.gnome.Platform//48`
- Flathub is required as runtime source for end-user bundle installs.
- AppStream metadata is GPLv3+ and screenshot-enabled.
- Runtime permissions avoid network access.

## Status

- Klondike is highly playable and polished.
- Engine/window modularization is in active progress for scalable multi-variant growth.
- Spider and FreeCell scaffolding already exists in the mode system.

### Variant Readiness

| Variant | Readiness | Notes |
|---|---|---|
| Klondike | Playable | Full gameplay + automation + hinting + persistence |
| Spider | Scaffolded | Runtime/data model exists; engine/rules integration in progress |
| FreeCell | Scaffolded | Mode exists as placeholder; gameplay engine not yet implemented |

## Known Constraints

- This Codeberg repository and release channel are an alpha testbed.
- Flatpak is the only official distribution format for this project at the moment.
- Remote install path currently uses `--no-gpg-verify` for alpha testing.
- Internal scripts under `scripts/` are maintainer operations, not a stable public CLI.

## Changelog

- `CHANGELOG.md`
- `RELEASE.md` (maintainer release process)
- `data/io.codeberg.emviolet.cardthropic.metainfo.xml.in` (AppStream release metadata)
