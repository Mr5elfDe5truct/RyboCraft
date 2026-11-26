# Compatibility with files from RyboCraft 1.0

RyboCraft 1.0 stored its designs, settings and exports under older internal names (`solidcraft`).
Later builds use `rybocraft` everywhere, and still read what 1.0 wrote. The older name survives
**only** in the readers below, their tests and this file. When you add or remove one of them,
update this file in the same change.

## 1. Design files

New designs are saved as `.sc`. `.solidcraft` files still open everywhere (`.rybocraft` too).
One constant names the extension new designs get, `DESIGN_EXT` (`sc`), and one list names every
extension a design may have, `DESIGN_EXTS` (`sc`, then `LEGACY_DESIGN_EXTS`); code uses those,
never its own string.

| Path in | Where |
|---|---|
| Extension → design format | `crates/io/src/lib.rs` (`DESIGN_EXTS`, `is_design_ext`, `is_design_path`, format detection) |
| Open dialog filters (desktop and browser) | `crates/io/src/formats.rs`, shown by `apps/rybocraft/src/main.rs` (`services`) |
| Drag-drop, recent files, `doc.open`, MCP `open`, CLI `eval`/`run`, Insert/Link/Relink design | the same format detection in `rybocraft_io` |
| Browser file picker | `apps/rybocraft-web/src/disk.rs` (`open_options`, `opens_design`) |
| Windows | `assets/platforms/windows/rybocraft.wxs`: `.sc` is the design ProgId's extension; the older extensions point at the same ProgId |
| macOS | `assets/platforms/macos/Info.plist.in` |
| Linux | `assets/platforms/linux/com.rgstudios.rybocraft.mime.xml` |

Opening an older file and pressing **Save** writes that same file; **Save As** offers `.sc`.
Autosaves and crash reports with the older extensions are still listed and recovered
(`rybocraft_engine::recovery`, `rybocraft_engine::crash::read`).

## 2. Format tags inside files

Files record their format as `"format": "rybocraft/N"` (designs), `rybocraft-container/N`
(`.sc` containers), `rybocraft-drawing/N`, `rybocraft-theme/1`, `rybocraft-profile/1` and
`rybocraft-shortcuts/1`. The `solidcraft…` tag of each is read too, and the file gets the new tag
when it is saved again:

| Tag | Reader |
|---|---|
| `solidcraft/N` | `crates/doc/src/format.rs` (`LEGACY_FORMAT_PREFIX`) |
| `solidcraft-drawing/N` | `crates/doc/src/format.rs` (`LEGACY_DRAWING_PREFIX`) |
| `solidcraft-container/N` | `crates/io/src/design.rs` (`LEGACY_VERSION`, `is_container_version`) |
| `solidcraft-theme/1` | `crates/ui-kit/src/theme_files.rs` (`LEGACY_FORMAT`) |
| `solidcraft-profile/1` | `crates/ui-egui/src/settings_profiles.rs` (`LEGACY_FORMAT`) |
| `solidcraft-shortcuts/1` | `crates/ui-egui/src/toolbar_custom.rs` |

Not backward-compatible: RyboCraft 1.0 refuses files saved by later builds.

## 3. Settings and environment variables

The preferences key is `rybocraft.prefs`; `solidcraft.prefs` is read when it is missing
(`rybocraft_ui_egui::prefs::stored`). Every runtime variable is read through
`rybocraft_geom::env`: `RYBOCRAFT_<NAME>`, else `SOLIDCRAFT_<NAME>`. The shell entry points do the
same (`packaging/env.sh`, `packaging/linux/AppRun`). Settings folders are named `RyboCraft` and
don't move.

## 4. Command line and MCP

New MCP setups use:

```sh
claude mcp add rybocraft -- rybocraft-cli mcp
```

Resources are `rybocraft://design` and `rybocraft://commands`; `solidcraft://` URIs still read
(`crates/mcp/src/server.rs`).
