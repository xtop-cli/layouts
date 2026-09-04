# Layout file schema (JSONC)

Formal, code-grounded description of the layout format consumed by
`xtop-layout` (crate `xtop-layout` v0.1.0). The schema is *not* enforced by a
separate `.schema.json`: it is implemented by the serde model and the manual
visitor in `src/model.rs`, plus the JSONC comment stripper in `src/loader.rs`.
This document mirrors that code exactly, so it is authoritative for
contributors and for the kernel's `xtop layout check` command (which runs the
same `parse_layout_err` function from `src/loader.rs`).

## File conventions

- One layout per file. The layout's display name is the `"name"` field inside
  the file, not the file name.
- Files may use `.jsonc` or `.json`; JSONC (comments allowed) is the
  convention everywhere in this repo.
- Built-in defaults live in `layouts/default/` and are embedded into the
  binary through `DEFAULT_LAYOUT_SOURCES` in `src/loader.rs` (7 files).
- User layouts live in the platform config dir under `layouts/` — see
  `docs/authoring.md` for the concrete paths per platform.
- A directory is loaded by `load_layouts_from_dir`: every `*.json`/`*.jsonc`
  entry is parsed; files that fail to parse are skipped and reported on
  stderr (see `docs/authoring.md`).
- Layout names must be unique among defaults and user files: `merge_layouts`
  matches names with exact, case-sensitive string equality.

## Document structure

A layout document is a single JSON object with exactly two keys:

```jsonc
{
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

| Key | Type | Required | Meaning |
|---|---|---|---|
| `name` | string | yes | Display name shown in the TUI palette; used for mode and override matching. Missing → error `missing field \`name\``. |
| `root` | area object | yes | The layout tree. Missing → error `missing field \`root\``. |

Any other top-level key is rejected (the custom deserializer only accepts
`name`/`root`; error `unknown field ...`). Inside `root` and inside every
area object the parser is permissive: extra keys are silently ignored, only
the keys in the table below are read.

The root uses the same object grammar as every area (below). Its `size` key
is parsed and validated like any other area's, **but the value is
discarded**: a `LayoutDef` keeps only the root node, so the root always
spans the whole terminal render area the kernel gives it.

## Area objects

Every area in a split (and the root) is one JSON object:

```jsonc
{
  "size": <size>,          // optional, see "size" below
  "widget": "name",        // leaf: a widget instance
  // -- or --
  "direction": "vertical", // container: nested split
  "areas": [ <area>, ... ] // children of the split (only with "direction")
}
```

| Key | Type | Meaning |
|---|---|---|
| `size` | number, string, or omitted | Constraint this area gets from its parent split (see below). Omitted ≡ `"*"` (fill). |
| `widget` | string | Widget id; see `docs/authoring.md` for the ids that exist. |
| `direction` | string | Split direction: `"vertical"` or `"horizontal"`, case-insensitive (`direction.to_lowercase()` is compared, so `"Vertical"`, `"VERTICAL"` etc. are accepted; anything else → error `invalid direction: <value>`). |
| `areas` | array of area objects | Children of a split. Optional; missing/empty → an empty split that parses but renders nothing. |

Exactly one of `widget` and `direction` must be present, otherwise the file is
rejected with `layout area must have 'widget' or 'direction'`. If *both* are
present the area parses, `widget` wins and the split keys (`direction`,
`areas`) are ignored. Extra keys on an area object are ignored (the struct
deserializer does not enable `deny_unknown_fields`).

### `size` — constraint syntax

The raw value is deserialized as either a JSON number or a JSON string
(`SizeRaw` in `src/model.rs`), then converted to a constraint:

| JSONC spelling | Constraint | Effect in the kernel |
|---|---|---|
| *(omitted)* | Fill | Take the remaining space |
| bare number `3` | `Length(3)` | Fixed `3` rows (vertical split) or columns (horizontal split) |
| `"45%"` | `Percentage(45)` | `45%` of the parent container |
| `"*"` | Fill | Take the remaining space |

Parsing details, exactly as implemented in `TryFrom<LayoutAreaRaw>`:

- **Bare number**: deserialized as `u16` — must be an integer between `0` and
  `65535`. Fractions (`3.5`), negatives and out-of-range values fail JSON
  deserialization of the untagged enum (`data did not match any variant of
  untagged enum SizeRaw`).
- **`"*"`**: the exact string, checked *before* the percentage branch. Any
  other string ending in `%` goes to the percentage branch.
- **`"NN%"`**: all trailing `%` characters are trimmed
  (`trim_end_matches('%')`, so `"45%%"` parses as `45`), then the remainder is
  parsed as `u16`; failure produces `invalid percentage: <value>`. The parsed
  number is **not** range-checked against `100` — anything `0..=65535` is
  accepted syntactically; the kernel hands the value to the TUI constraint
  engine, so percentages above `100` are possible but meaningless there.
  `"%"` alone fails (`invalid percentage: %`).
- **Any other string** (not `"*"`, not ending in `%`, or empty) → error
  `invalid size constraint: <value>`.

Fill areas are resolved by the kernel's renderer with equal weight
(`Constraint::Fill(1)` in `xtop/src/ui/layout/engine.rs`): when several
siblings use `"*"` (or omit `size`), the leftover space is shared equally.

## Direction semantics

The format is UI-framework agnostic; the kernel's render engine maps each
split to a TUI layout of the same direction:

- `"direction": "vertical"` — children are stacked **top to bottom**; each
  area spans the full width and gets a height according to its `size`.
- `"direction": "horizontal"` — children are placed **left to right**; each
  area spans the full height and gets a width according to its `size`.

Constraint translation (kernel `ui/layout/engine.rs`):
`Length(n)` → `Constraint::Length(n)`, `Percentage(p)` →
`Constraint::Percentage(p)`, Fill → `Constraint::Fill(1)`. Percentages are
relative to the enclosing split after fixed lengths are reserved; how the
leftover space is distributed exactly (rounding, partial percentage sums) is
the constraint engine's behavior, not part of this schema.

## JSONC dialect

Comments are removed before JSON parsing by `strip_jsonc_comments` in
`src/loader.rs` (the file is then parsed with `serde_json`, which accepts
strict JSON only):

- `//` line comments — up to the next newline.
- `/* ... */` block comments — not nestable; an unterminated block comment
  silently consumes the rest of the file.
- The stripper is string-aware: comment markers inside a string literal do
  not start a comment. Both `"` and `'` are tracked as possible string
  delimiters (escapes are honoured), but only `"..."` is valid JSON, so use
  double quotes for every value.
- **Trailing commas are not supported** (the stripper does not remove them;
  `serde_json` rejects them). Keep JSON syntax otherwise strict.

## Canonical (re-)serialized shape

`LayoutDef` implements a mirror serializer, so re-serialized layouts come out
as the same grammar with these exact spellings: `size` numbers for fixed
lengths, `"NN%"` strings for percentages, `"*"` for fill (fill is always
written explicitly), then either `widget` or `direction` + `areas`.

## Embedded defaults

`DEFAULT_LAYOUT_SOURCES` in `src/loader.rs` embeds seven files in palette
order (index 0–6). Palette order and names are covered by a unit test
(`test_default_layouts_count_and_order`):

| Slot | File | `"name"` | Root structure | Widget ids used |
|---|---|---|---|---|
| 0 | `dashboard.jsonc` | `Dashboard` | vertical: `header` (3) → `horizontal` split (45%, cpu 50% + vertical split of memory/storage/network) → `processes` (52%) | header, cpu, memory, storage, network, processes |
| 1 | `vertical.jsonc` | `Vertical` | vertical: `header` (3), `cpu` (8), `memory` (8), `storage` (6), `network` (5), `processes` (*) | header, cpu, memory, storage, network, processes |
| 2 | `horizontal.jsonc` | `Horizontal` | vertical: `header` (3) → `horizontal` split (*) of cpu/memory/storage/network (25% each) | header, cpu, memory, storage, network |
| 3 | `cpu_focus.jsonc` | `CPU Focus` | vertical: `header` (3), `cpu` (60%), `processes` (*) | header, cpu, processes |
| 4 | `memory_focus.jsonc` | `Memory Focus` | vertical: `header` (3), `memory` (60%), `processes` (*) | header, memory, processes |
| 5 | `network_focus.jsonc` | `Network Focus` | vertical: `header` (3) → `horizontal` split (50%) of `network` + `disk_io` (50% each) → `processes` (*) | header, network, disk_io, processes |
| 6 | `process_focus.jsonc` | `Process Focus` | vertical: `header` (3) → `horizontal` split (8) of cpu/memory/storage/network (25% each) → `processes` (*) | header, cpu, memory, storage, network, processes |

Seven distinct widget ids are referenced across the defaults: `header`,
`cpu`, `memory`, `storage`, `network`, `processes`, `disk_io`. None of the
defaults puts a `size` on the root.

The focus layouts share the same shape: a 3-row `header`, the focused widget
(plus a companion column for `Network Focus`) taking most of the middle, and
the `processes` list filling the remainder.
