<h1 align="center">
<img src="https://raw.githubusercontent.com/xtop-cli/web/main/public/img/logo.png" width="100px" alt="Xtop logo" />Layouts</h1>

Data-driven **layout** definitions for the [xtop](https://github.com/xtop-cli/xtop)
TUI system monitor.

This crate (`xtop-layout`) owns everything about layouts so the kernel stays
thin:

- `model` — layout tree (`LayoutDef` → splits of areas → widget leaves),
  serializable from JSON/JSONC, no UI dependencies.
- `default/` — the shipped **default layout files**: the primary `Dashboard`
  layout (the only one embedded in the crate) plus the other six mode-bound
  layouts (`Vertical`, `Horizontal`, `CPU Focus`, `Memory Focus`, `Network
  Focus`, `Process Focus`) and three preset extras — `Detail Dashboard`,
  `Detail Network`, `Detail Processes` (`detail_*.jsonc`) — that showcase
  per-widget `options` (DR-UX6). The non-base files are installed on demand
  from the official catalog into the user config directory.

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
  On startup the kernel writes the base `Dashboard` as an editable template
  into the user config layouts dir. The other files are downloaded with
  `xtop theme`/`xtop layout download` from the official catalog and are
  never overwritten without `--force`.
- `custom/` — **community layouts**: installable extras, shared via PRs.
  They are listed in the official catalog and installed with
  `xtop layout download <name>` / `xtop layout install <name>` (or copied
  manually into `~/.config/xtop/layouts/` on Linux).
- `loader` — loading/parsing of JSONC files and merging with user overrides.
- `mode` — built-in layout modes and terminal-size degradation rules.

## Layout structure

```
xtop-cli/layouts/
  src/                  xtop-layout crate (model, loader, mode)
  layouts/
    default/            default layout files (Dashboard is embedded; the rest are catalog assets)
    custom/             community layouts (catalog assets, via PR)
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
  area grammar, constraint syntax, comments dialect, base and catalog assets).
- `docs/authoring.md` — step-by-step guide: writing, validating and
  installing layouts, widget ids, modes and terminal-size thresholds.
- `docs/decisions.md` — design decision log (DR-3, unvalidated widget ids,
  name-based merging).

## Personalization model

1. **Base**: `Dashboard` is embedded and copied as a template to
   `~/.config/xtop/layouts/` on first run.
2. **Catalog assets**: every other shipped layout is external. Install it
   with `xtop layout download <name>` (or `xtop config download --all`); the
   file lands in the user layouts directory with version, author and license
   metadata recorded in `assets.json`.
3. **Overrides**: edit a file whose `"name"` matches an installed layout and
   it replaces it in place; `--force` is required to replace a downloaded
   file through the CLI.
4. **New layouts**: any extra JSONC file in the directory shows up as an
   additional layout.
5. **Community**: share layouts via PR to `layouts/custom/`; users install
   them by catalog name.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). User-contributed layouts remain under the license chosen by their authors; see the license note.
