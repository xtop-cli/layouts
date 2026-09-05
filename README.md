# xtop-layouts

Data-driven **layout** definitions for the [xtop](https://github.com/xtop-cli/xtop)
TUI system monitor.

This crate (`xtop-layout`) owns everything about layouts so the kernel stays
thin:

- `model` — layout tree (`LayoutDef` → splits of areas → widget leaves),
  serializable from JSON/JSONC, no UI dependencies.
- `default/` — 10 embedded **default layouts** shipped with the crate: the
  seven mode-bound layouts (`Dashboard`, `Vertical`, `Horizontal`, `CPU
  Focus`, `Memory Focus`, `Network Focus`, `Process Focus`) plus three
  preset extras — `Detail Dashboard`, `Detail Network`, `Detail
  Processes` (`detail_*.jsonc`) — that sit after the modes in file order
  and showcase per-widget `options` (DR-UX6).

  Preset table (UX8.5: dense full-monitor pages with zero orphan rows at
  common sizes — every split is percentage-exact or Fill-absorbed, see
  `docs/authoring.md` "Density guidance"):

  | File | `"name"` | Composition at 100x34 (31 body rows) | Widgets |
  |---|---|---|---|
  | `detail_dashboard.jsonc` | `Detail Dashboard` | header → 18-row monitor band (cpu left, 36% side column of summary/sensors) → 13-row full-width processes | header, cpu, summary, sensors, processes |
  | `detail_network.jsonc` | `Detail Network` | header → 22-row band (network per-iface box + summary/disk_io/memory column) → 9-row processes | header, network, disk_io, summary, memory, processes |
  | `detail_processes.jsonc` | `Detail Processes` | header → 8-row stat strip (summary/cpu/memory/storage/network) → 23-row full-height processes | header, summary, cpu, memory, storage, network, processes |

  `summary` (load/uptime/process-count panel) and `sensors` (per-core
  temperature heat view) are the UX8.4 widget-wave arrivals placed by the
  UX8.5 presets; names resolve at render time like every other widget id.
  On startup the kernel writes
  these as editable templates into the user config layouts dir when it
  initializes its assets (never overwriting files the user already
  edited).
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
   `~/.config/xtop/layouts/` on first run (seven mode-bound layouts first,
   then the `detail_*` preset extras — order is part of the mode-slot
   contract, see `docs/authoring.md` §6).
2. **Overrides**: edit a file whose `"name"` matches a default → it replaces
   that default in place (same palette position, no duplicates).
3. **New layouts**: any extra file shows up as an additional layout.
4. **Community**: share layouts via PR to `layouts/custom/`; users copy or
   install them into their config dir.
