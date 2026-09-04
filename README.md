# xtop-layouts

Data-driven **layout** definitions for the [xtop](https://github.com/xtop-cli/xtop)
TUI system monitor.

This crate (`xtop-layout`) owns everything about layouts so the kernel stays
thin:

- `model` — layout tree (`LayoutDef` → splits of areas → widget leaves),
  serializable from JSON/JSONC, no UI dependencies.
- `default/` — 7 embedded **default layouts** shipped with the crate
  (`Dashboard`, `Vertical`, `Horizontal`, `CPU Focus`, `Memory Focus`,
  `Network Focus`, `Process Focus`). On startup the kernel writes these as
  editable templates into the user config layouts dir when it initializes
  its assets (never overwriting files the user already edited).
- `custom/` — **community layouts**: installable extras, shared via PRs.
  They never ship in the binary; the kernel's `xtop layout install <name>`
  fetches this folder and copies the layout into the user config layouts
  dir (or copy the file there yourself, e.g. `~/.config/xtop/layouts/` on
  Linux).
- `loader` — loading/parsing of JSONC files and merging with user overrides.
- `mode` — built-in layout modes and terminal-size degradation rules.

## Layout structure

```
xtop-cli/layouts/
  src/                  xtop-layout crate (model, loader, mode)
  layouts/
    default/            built-in defaults (embedded in the binary)
    custom/             community layouts (installable, via PR)
```

## Use

```toml
[dependencies]
xtop-layout = { git = "https://github.com/xtop-cli/layouts" }
```

```rust
use xtop_layout::{default_layouts, merge_layouts, load_layouts_from_dir};

let defaults = default_layouts();
let custom = load_layouts_from_dir(&user_layouts_dir); // ~/.config/xtop/layouts
let all = merge_layouts(defaults, custom); // user wins by name
```

Layout files are JSON/JSONC:

```jsonc
{
  "name": "My Layout",
  "root": {
    "direction": "vertical",
    "areas": [
      { "widget": "header", "size": 3 },
      { "widget": "cpu", "size": "45%" },
      { "widget": "processes", "size": "*" }
    ]
  }
}
```

`size` accepts a fixed number of rows/columns, a percentage (`"45%"`) or
`"*"`/omitted for the remaining space. Widgets are referenced by name; the
kernel decides which renderer each name maps to.

## Documentation

- `docs/layout-schema.md` — formal JSONC layout schema (document structure,
  area grammar, constraint syntax, comments dialect, embedded defaults).
- `docs/authoring.md` — step-by-step guide: writing, validating and
  installing layouts, widget ids, modes and terminal-size thresholds.
- `docs/decisions.md` — design decision log (DR-3, unvalidated widget ids,
  name-based merging).

## Personalization model

1. **Defaults** are compiled into the binary and copied as templates to
   `~/.config/xtop/layouts/` on first run.
2. **Overrides**: edit a file whose `"name"` matches a default → it replaces
   that default in place (same palette position, no duplicates).
3. **New layouts**: any extra file shows up as an additional layout.
4. **Community**: share layouts via PR to `layouts/custom/`; users copy or
   install them into their config dir.
