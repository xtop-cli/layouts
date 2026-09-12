# Community layouts (installable)

Layouts in this folder are **not built into the binary**: they are extra
layouts anyone can install or share. This folder is exactly what the kernel
fetches for `xtop layout install` (it sparse-clones this repo's
`layouts/custom/`), so every file here is an installable community layout.

- Drop a layout file here and open a PR to share it with everyone.
- Every file must follow the layout format (see the root `README.md` and the
  formal schema in `docs/layout-schema.md`) and use a unique `"name"` (do
  not collide with the `default/` names or with other files here).
- File name convention: `my_layout.jsonc` (name inside the file is what the
  TUI shows; `xtop layout install` accepts either as the lookup key).
- Validate before sharing: `xtop layout check <file>`.

## Using a community layout

The kernel installs community layouts into the user config dir. Either way
the file lands in the user layouts folder and is loaded at the next startup
of the TUI (and becomes part of the `l` layout palette):

```sh
# 1. Fetch and install from this repo's layouts/custom/ (needs git):
xtop layout install <name>

# 2. ...or copy the file yourself:
mkdir -p ~/.config/xtop/layouts          # Linux; see docs/authoring.md for macOS/Windows
cp my_layout.jsonc ~/.config/xtop/layouts/
```

`xtop layout install` matches a file by its file stem or by the layout
`"name"` inside it (case-insensitive), keeps the original file name, and
refuses to overwrite an existing file in the user layouts dir — edit that
file in place instead. Custom files override a built-in layout by reusing
its `"name"` (same palette position) or appear as extra layouts with a new
`"name"`; see `docs/authoring.md` for the full flow.

## Runtime widget layouts

Some layouts here target **runtime widgets** (sandboxed WASM modules or
helper processes) instead of compiled-in packs:

| File | Layout name | Widgets required |
|---|---|---|
| `wasm_demo.jsonc` | WASM Demo | `wasm-clock`, `wasm-cpu`, `wasm-procs`, `lua-clock`, `py-cpu` |
| `polyglot_dashboard.jsonc` | Polyglot Dashboard | the above plus `c-ticker`, plus the `header`/`network`/`storage` packs |
| `wasm_procs_focus.jsonc` | WASM Procs Focus | `wasm-cpu`, `py-cpu`, `wasm-procs` |
| `cpu_stats.jsonc` | CPU Stats | `py-cpu-chart`, `py-cpu`, `wasm-cpu`, `wasm-procs` |
| `math_lab.jsonc` | Math Lab | `py-mem-regression`, `lua-histogram`, `wasm-load-stats`, `c-cpu-avg` |

They need a kernel built with the opt-in `plugin-wasm` / `plugin-external`
features and the example widgets from the
[`xtop-cli/plugins`](https://github.com/xtop-cli/plugins) repo
(`examples/wasm/` and `examples/external/`), installed in the user config
dirs (`wasm/` and `external/`). Without those widgets the referenced names
render empty areas and the kernel warns once per unknown name; see the
plugins repo docs (`docs/wasm-widgets.md`, `docs/external-widgets.md`).
