# Authoring guide: custom layouts

Step-by-step guide for writing a custom layout for xtop, validating it, and
making it available — locally or as a community layout. The authoritative
format description is `docs/layout-schema.md`; this guide focuses on the
workflow and on the ids, modes and install/validation flow as implemented in
the kernel (`xtop/src/commands/layout.rs`, `xtop/src/ui/layout/engine.rs`,
`xtop/src/ui/screen.rs`).

## 1. Where layouts live

The crate embeds seven default layouts (`layouts/default/*.jsonc`). Custom
layouts are loaded from the **user layouts directory** at startup
(`xtop/src/commands/share/bootstrap.rs` merges defaults with everything
`load_layouts_from_dir` finds there):

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
  whose name equals a default *replaces that default in place* (same palette
  slot); every other file is appended after the seven defaults.
- Use a 3-row `header` widget on top, like every default.
- In a `"vertical"` split children stack top-to-bottom; in `"horizontal"`
  they sit side-by-side. Percentages refer to the enclosing split.
- Comment freely with `//` or `/* */`; **trailing commas are not accepted**.
- Validate the file before shipping it (next section).

## 3. Validating

The kernel exposes `xtop layout check` (`xtop/src/commands/layout.rs`), which
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
  (table above). The directory is read once at startup, so restart xtop (or
  start it) to pick changes up; cycle the palette with `l`, or select the
  layout by name from the command palette. The active layout name is
  persisted in the config and restored on the next run.
- **Seeded defaults**: on startup the kernel seeds copies of the seven
  default `.jsonc` templates into the user layouts dir (marker file
  `.xtop_initialized`, version `1`; files that already exist are never
  overwritten — `xtop/src/commands/share/assets.rs`). Editing a seeded copy
  is the supported way to tweak a built-in default; because overrides match
  by `"name"`, the edited file replaces the default at its usual palette
  slot.
- **Overrides vs. new layouts**: same name → in-place replacement (slot
  preserved, no duplicate). New name → appended as an extra layout.

## 5. Widget ids

Layout files reference widgets by plain string ids; there is no
compile-time allowlist, resolution happens at render time
(`xtop/src/ui/layout/engine.rs`). What exists today:

| Id | Where | Notes |
|---|---|---|
| `header`, `cpu`, `memory`, `storage`, `network`, `processes`, `disk_io` | Base pack (`xtop-widgets` registry) | Used by the seven defaults; always available |
| `battery`, `gpu` | Base pack registry | Registered renderers; selectable in full-screen mode, not used by any default. Their data providers are platform-dependent, so expect empty values where no source exists |
| `samurai` (plugin widgets) | Plugins with the `RenderWidgets` capability (kernel `plugins/manager.rs`) | Referencable when the kernel is built with the plugin enabled (e.g. `plugin-samurai`). Plugin renderers take precedence over pack renderers for the same name |

Reference order at render time: plugin widgets first, then the pack chosen
by the style config, falling back to the base pack. A typo'd or unavailable
id **renders nothing** in that area (the area is left blank): as of the
current kernel there is no warning for unknown ids inside a layout — the
full-screen view shows `No widget registered for '<name>'` instead. Because
of that, `xtop layout check` is structural validation only: it cannot know
whether an id will resolve.

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
before any layout logic runs — `xtop/src/ui/screen.rs`.)

Mode ↔ palette coupling (why order matters):

- The seven built-in modes map by *label* to the first seven palette slots.
  Slot order is fixed by `DEFAULT_LAYOUT_SOURCES` in `src/loader.rs`:
  `dashboard, vertical, horizontal, cpu_focus, memory_focus, network_focus,
  process_focus` → names `Dashboard, Vertical, Horizontal, CPU Focus, Memory
  Focus, Network Focus, Process Focus`. A unit test pins this order.
- `mode_from_layout_index` maps slots 0–6 back to their mode and defaults to
  `Dashboard` for anything ≥ 7, so the kernel's `l` cycling relies on the
  defaults keeping those slots: a same-name override keeps its slot, extra
  custom layouts are appended *after* them and are addressed by name only.
- The kernel restores a layout by `config.layout_name` first and falls back
  to `layout_index_from_mode(config.layout_mode, ...)` when the saved name
  vanished (e.g. an override file was removed).

## 7. Sharing a layout (community flow)

The kernel installs community layouts **from this repo's `layouts/custom/`
folder**, fetched over git:

```sh
xtop layout install <name>
```

What the kernel does (`xtop/src/commands/layout.rs`, `cmd_install`):

1. Runs `git clone --depth 1 --filter=blob:none --sparse
   https://github.com/xtop-cli/layouts` into a temp dir (git must be on
   `PATH`), then `git sparse-checkout set layouts/custom`.
2. Scans `layouts/custom/` for `*.json`/`*.jsonc` files and picks the first
   whose file stem *or* parsed `"name"` equals `<name>`
   (case-insensitive).
3. Copies that file into the user layouts dir **under its original file
   name**; it refuses to overwrite an existing target (edit in place
   instead) and errors when nothing matches or git fails.

So to share a layout: put `my_layout.jsonc` into `layouts/custom/` of this
repo (unique `"name"`, validated with `xtop layout check`), open a PR, and
after it lands anyone can `xtop layout install <name>`.

You can equally skip the repo: copy the file into the user layouts dir
yourself — the app loads it at the next startup either way. Community files
in this repo are *not* read by the kernel at runtime; they only become live
after the install (or a manual copy).
