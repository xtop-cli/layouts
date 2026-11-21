# Authoring guide: custom layouts

Step-by-step guide for writing a custom layout for xtop, validating it, and
making it available — locally or as a community layout. The authoritative
format description is `docs/layout-schema.md`; this guide focuses on the
workflow and on the ids, modes and install/validation flow as implemented in
the kernel (`xtop/src/cli/commands/layout.rs`, `xtop/src/tui/layout/engine.rs`,
`xtop/src/tui/screen.rs`).

## 1. Where layouts live

The crate embeds only the primary `Dashboard` layout. The other nine default
files (six mode-bound layouts and three preset extras:
`detail_dashboard`, `detail_network`, `detail_processes`, DR-UX6) are
external catalog assets installed with `xtop layout download <name>` or
`xtop config download --all`. Since UX8.5 the extras are dense, full-coverage pages: `Detail
Dashboard` is the full-monitor page (tall `cpu` with `summary`/`sensors`
side column + full-width `processes`), `Detail Network` focuses the
per-interface `network` box with a `summary`/`disk_io`/`memory` side column
and a bottom `processes` strip, and `Detail Processes` is a compact stat
strip (`summary` + `cpu`/`memory`/`storage`/`network`) over a full-height
`processes` table. Custom and downloaded layouts are loaded from the **user layouts
directory** at startup. The kernel's `core::bootstrap::load_layout_defs`
merges the embedded Dashboard with everything `load_layouts_from_dir` finds
there:

| Platform | User config dir | User layouts dir |
|---|---|---|
| Linux | `$XDG_CONFIG_HOME/xtop`, else `~/.config/xtop` | `<config>/layouts` |
| macOS | `~/Library/Application Support/xtop` | `<config>/layouts` |
| Windows | `%APPDATA%\xtop` | `<config>\layouts` |

The `xtop-layout` crate itself never touches the filesystem layout dirs of
users; the kernel owns the paths above (`xtop/src/config/platform/`).

## 2. Writing a layout

Create a `.jsonc` file containing exactly one layout:

```jsonc
{
  // My Layout: header, CPU chart and process list stacked
  "name": "My Layout",
  "root": {
    "direction": "vertical",
    "areas": [
      { "widget": "header", "size": 3 },
      { "widget": "cpu", "size": "60%" },
      { "widget": "processes", "size": "*" }
    ]
  }
}
```

Rules of thumb (full grammar: `docs/layout-schema.md`):

- `"name"` must be unique. Merging is by exact, case-sensitive name: a file
  whose name equals the embedded Dashboard *replaces it in place*; every
  other file is appended after the base. Downloaded and user layouts share
  the same directory and the same name-based merge rule. See
  `docs/layout-schema.md`, "Base and catalog defaults".
- Use a 3-row `header` widget on top, like every default.
- In a `"vertical"` split children stack top-to-bottom; in `"horizontal"`
  they sit side-by-side. Percentages refer to the enclosing split.
- Comment freely with `//` or `/* */`; **trailing commas are not accepted**.
- Each widget instance can take an optional `options` object that tunes how
  *that instance* is drawn — e.g.
  `{ "widget": "processes", "options": { "cpu": "total" } }`. Options are
  passthrough: the layout crate only stores and forwards them, renderers
  interpret them (grammar and examples: `docs/layout-schema.md`, section
  "Widget `options`"). No `options` means the widget's default behavior.
- Validate the file before shipping it (next section).

### Density guidance: no orphan bands, no starved boxes

UX8.5 presets are dense: every layout row is covered by a box and every box
is tall enough for its content to grow (charts/lists fill their box — the
UX8.4 widgets wave). The render engine (`xtop/src/tui/layout/engine.rs`)
feeds the constraints to ratatui, where percentages measure the **whole**
split and Fill takes the leftover, so write splits that tile by construction:

- **Pure-percentage stacks**: children sized `"p1%"`, `"p2%"`, ... with
  `p1 + p2 + ... = 100` tile the split exactly at every terminal size (no
  Fill needed).
- **Fixed header + Fill**: put a fixed `size: 3` header (or `size: 8`
  strip) next to a `"*"`/Fill sibling, never next to a percentage —
  percentages would measure the whole split (including the header rows) and
  over-claim. This is why every default root is `header (3)` + body `"*"`,
  with the tiling percentages living *inside* the body.
- **Fill alongside percentages**: fine when the percentages sum below 100 —
  the Fill absorbs the remainder (e.g. a `"*"` cpu column next to a `"36%"`
  side column). Keep the percentage demand below 100 or the Fill starves.
- A split with neither a Fill sibling nor percentages summing to 100 leaves
  an orphan band; the loader tests
  (`test_detail_presets_split_coverage_full_tiling`) enforce the rule on
  every split of every detail preset.

Row budget at the common sizes (body = terminal rows − 3 header rows):

| Widget | Content it absorbs rows with | Comfortable box (rows) | Keep under |
|---|---|---|---|
| `cpu` | core grid + per-core freq/temp + multi-row history | 12–22 | — |
| `processes` | process list | 9–30 | — |
| `summary` | load/uptime/proc-count lines + sparklines | 6–9 | ~10 |
| `sensors` | per-core temperature heat view (one-line fallback without temps) | 7–12 | ~14 |
| `memory` | RAM/SWP rows + sections + history chart | 6–10 | ~12 |
| `network` | per-iface rows + aggregate dual chart | 6–26 | — |
| `disk_io` | per-disk rows + r/w chart | 6–12 | — |
| `storage` | per-disk bars | 5–9 | ~10 |

Rough numbers at 100x34 (31 body rows): `Detail Dashboard` = 18-row monitor
band (cpu 64 cols × 18 rows, summary 8 rows, sensors 10 rows) over 13 rows
of processes; `Detail Network` = 22-row network box + 7/8/7
summary/disk_io/memory side column over 9 rows of processes; `Detail
Processes` = 8-row strip over a 23-row process table. The same trees give
80x24 (21 body rows) 12/9, 15/6 and 8/13 splits and 120x40 (37) 21/16,
26/11 and 8/29 — always full coverage, because every split is
percentage-exact or Fill-absorbed.

## 3. Validating

The kernel exposes `xtop layout check` (`xtop/src/cli/commands/layout.rs`), which
runs the exact parser of this crate (`parse_layout_err`):

```sh
xtop layout check my_layout.jsonc
# OK  my_layout.jsonc -> layout "My Layout" is valid
# (invalid files fail with an "Error: INVALID <path> -> <reason>" line
#  and a non-zero exit code)
```

The underlying crate functions are also public: `parse_layout(source) ->
Option<LayoutDef>` and `parse_layout_err(source) -> Result<LayoutDef, String>`
(the latter is what `check` evaluates).

While developing, prefer a directory the kernel already reads, or call the
crate from a scratch test — but remember how *invalid files* are handled:
`load_layouts_from_dir` skips any file that fails to parse and prints
`[xtop-layout] skipping invalid layout file: <path>` to stderr, so a broken
file never kills the app — it simply does not appear in the palette. It is
easy to miss, which is why `xtop layout check` exists.

## 4. Making it active

- **Personal layout**: put the file directly into the user layouts dir
  (table above), then run `xtop layout set <name>` or restart xtop. The TUI
  polls the persisted layout name and reloads the layouts directory when the
  name is not loaded yet, so a running instance can pick up a new file after
  `set`. The active layout name is persisted in the config and restored on
  the next run.
- **Base layout**: only `dashboard.jsonc` is embedded. The kernel writes it
  as an editable template when initializing assets or when
  `xtop --gen-config` runs (`xtop/src/core/assets`). Existing files are
  never overwritten. Editing it customizes the base Dashboard; overrides
  match by `"name"`.
- **Catalog layouts**: every other layout is downloaded from the official
  catalog into the user layouts dir with version, author, license and
  checksum metadata. The downloaded file is a normal editable JSONC file.
- **Overrides vs. new layouts**: same name → in-place replacement. New name
  → appended as an extra layout.

## 5. Widget ids

Layout files reference widgets by plain string ids; there is no
compile-time allowlist, resolution happens at render time
(`xtop/src/tui/layout/engine.rs`). What the presets reference today:

| Id | Where | Notes |
|---|---|---|
| `header`, `cpu`, `memory`, `storage`, `network`, `processes`, `disk_io` | Base pack (`xtop-widgets` registry) | Used by every default (mode layouts and detail presets); always available |
| `summary`, `sensors` | Base pack registry (UX8.4 additions) | Referenced by the detail presets since UX8.5: `summary` is the load/uptime/process-count panel with sparklines, `sensors` the per-core temperature heat view (honest one-line fallback when no temp data). They resolve at render time like every other name — a layout file is valid before the pack lands, it just renders nothing for a name no renderer provides |
| `battery`, `gpu` | Base pack registry | Registered renderers; selectable in full-screen mode, not used by any default. Their data providers are platform-dependent, so expect empty values where no source exists |
| `samurai` (plugin widgets) | Plugins with the `RenderWidgets` capability (kernel `plugins/manager.rs`) | Referencable when the kernel is built with the plugin enabled (e.g. `plugin-samurai`). Plugin renderers take precedence over pack renderers for the same name |

Reference order at render time: plugin widgets first, then the pack chosen
by the style config, falling back to the base pack. A typo'd or unavailable
id **renders nothing** in that area (the area is left blank); the kernel
prints a one-time stderr warning per unknown name (`xtop: layout '<name>'
references unknown widget '<name>'` — `xtop/src/tui/layout/engine.rs`), and
the full-screen view shows `No widget registered for '<name>'` instead.
Because of that, `xtop layout check` is structural validation only: it
cannot know whether an id will resolve.

## 6. Modes and thresholds

The crate computes how the requested mode degrades with terminal size
(`src/mode.rs`, `detect_effective_layout`). Exact thresholds:

| Requested mode | Terminal size | Effective layout |
|---|---|---|
| any | width < 60 **or** height < 14 | `Minimal` (minimal renderer, no layout file) |
| `Dashboard` | width < 80 | `Vertical` |
| `Dashboard` | 80 ≤ width < 100 **or** height < 28 | `Compact` (Dashboard visuals, less padding) |
| `Dashboard` | width ≥ 100 and height ≥ 28 | `Dashboard` |
| `Vertical` / `Horizontal` / focus modes | ≥ 60×14 | the requested mode itself |

(Kernel UI floor: below 40×8 the screen shows a "Terminal too small" notice
before any layout logic runs — `xtop/src/tui/screen.rs`.)

Mode and layout resolution:

- `DEFAULT_LAYOUT_SOURCES` in `src/loader.rs` embeds only `dashboard`
  (`Dashboard`). The other six mode layouts are catalog assets with the
  names `Vertical`, `Horizontal`, `CPU Focus`, `Memory Focus`,
  `Network Focus` and `Process Focus`.
- The kernel resolves layouts by display name. `mode_from_layout_index` is a
  legacy helper for mode-ordered lists; it is not used to infer the mode of
  downloaded layouts. A layout whose name matches a built-in mode selects
  that mode; anything else keeps the previous mode.
- The kernel cycles through the effective list in file order and wraps
  around. The absolute order of downloaded layouts is not a compatibility
  contract; users select by name or cycle through what is installed.
- The kernel restores a layout by `config.layout_name` first and falls back
  to `layout_index_from_mode(config.layout_mode, ...)` when the saved name
  vanished (e.g. an override file was removed).

## 7. Sharing a layout (community flow)

The kernel installs default and community layouts **from the official
catalog**:

```sh
xtop layout download <name>     # or: xtop layout install <name>
xtop layout download --all
```

What the kernel does (`xtop/src/cli/commands/layout.rs`):

1. Fetches `catalog.json` from the kernel repository (or
   `XTOP_CATALOG_URL`).
2. Finds the layout by catalog id or display name (case-insensitive).
3. Downloads the raw JSONC file, verifies the catalog SHA-256, validates it
   with `parse_layout_err`, and writes it atomically into the user layouts
   dir under its catalog file name.
4. Records version, author and license in `assets.json`.

Existing files are skipped unless `--force` is used.

So to share a layout: put `my_layout.jsonc` into `layouts/custom/` of this
repo (unique `"name"`, validated with `xtop layout check`), add it to
`catalog.json` in the kernel repository with its version, author, license,
URL and SHA-256, and open the PRs. After they land, anyone can
`xtop layout download <name>`.

You can equally skip the catalog: copy the file into the user layouts dir
yourself — the app loads it after `xtop layout set` or the next startup.
Community files in this repo are *not* read by the kernel at runtime; they
only become live after the download (or a manual copy).
