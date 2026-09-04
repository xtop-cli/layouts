# Community layouts (installable)

Layouts in this folder are **not built into the binary**: they are extra
layouts anyone can install or share.

- Drop a layout file here and open a PR to share it with everyone.
- Every file must follow the layout format (see the root `README.md`) and
  use a unique `"name"` (do not collide with the `default/` names or with
  other files here).
- File name convention: `my_layout.jsonc` (name inside the file is what the
  TUI shows).

## Using a community layout

Either copy the file into your user layouts folder:

```sh
mkdir -p ~/.config/xtop/layouts
cp custom/<name>.jsonc ~/.config/xtop/layouts/
```

or, from a checkout of the repo, run:

```sh
cargo run --example install <name>   # future: xtop layout install <name>
```

Custom files in `~/.config/xtop/layouts/` appear as extra layouts in the
layout palette (`l` to cycle) and can override a built-in by reusing its
`"name"`.
