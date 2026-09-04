# xtop-layout

Data-driven **layout** definitions for the [xtop](https://github.com/xtop-cli/xtop)
TUI system monitor.

This crate owns everything about layouts so the kernel and future consumers
stay thin:

- `model` — layout tree (`LayoutDef` → splits of areas → widget leaves),
  serializable from JSON/JSONC, no UI dependencies.
- `loader` — 7 embedded **default layouts** (`Dashboard`, `Vertical`,
  `Horizontal`, `CPU Focus`, `Memory Focus`, `Network Focus`,
  `Process Focus`) plus loading of user files from a directory.
- `merge` — defaults + user layouts with user files overriding defaults of
  the same name (editable and personalizable out of the box).
- `mode` — built-in layout modes and terminal-size degradation rules.

## Use

```toml
[dependencies]
xtop-layout = { git = "https://github.com/xtop-cli/layout" }
```

```rust
use xtop_layout::{default_layouts, merge_layouts, load_layouts_from_dir, LayoutDef};

let defaults = default_layouts();
let custom = load_layouts_from_dir(&layouts_dir); // user ~/.config/xtop/layouts
let all: Vec<LayoutDef> = merge_layouts(defaults, custom); // user wins by name
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
