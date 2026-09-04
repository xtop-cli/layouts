# Design decisions

Short log of the decisions that shape this crate and the layout file
format. Grounded in this repo's code and in the kernel sources referenced
below.

## D-1 — Layouts live in a standalone, UI-free crate (DR-3)

**Decision (ROADMAP DR-3).** The layout model, loader and modes live only
in `xtop-layout`; the kernel has no internal layout model or loader. The
crate has no ratatui dependency and no dependency on any `xtop-*` contract
crate: it is pure data + serde (`src/lib.rs` module docs). The kernel
consumes it as a git dependency and translates the tree to concrete UI
constraints at render time (`xtop/src/ui/layout/engine.rs` maps
`LayoutConstraint` values to ratatui constraints).

**Why.** The kernel stays a thin host; layout authoring, defaults and the
degradation rules are testable without a terminal or the widget pack
ecosystem; the layout format is a stable interchange surface between repos
that never needs the widget registry at compile time. Consequences accepted:

- The format schema is implicit in the serde model + manual visitor of
  `src/model.rs` — documented formally in `docs/layout-schema.md` instead of
  a generated `.schema.json` (nothing consumes a JSON Schema file today).
- Widget ids are opaque strings (next entry), and nothing in this repo can
  verify them against the widgets or plugins repos.
- The palette ordering contract (defaults in slots 0–6, fixed order) is
  enforced by `DEFAULT_LAYOUT_SOURCES` order + `mode_from_layout_index` +
  `merge_layouts` semantics and pinned by unit tests (see `docs/authoring.md`
  §6).

## D-2 — Widget ids are unvalidated strings; resolution is runtime

**Decision.** Layout files name widgets as plain strings (`LayoutNode::Widget
{ name }` in `src/model.rs`). The crate does not carry an allowlist: it has
no dependency on the widgets/plugins repos, so it cannot know which ids are
valid for a given kernel build (which packs are compiled in, which plugins
are enabled).

**Consequence, as implemented in the current kernel.** Widget ids resolve at
render time against, in order: plugin widgets (e.g. `samurai`), then the
style-chosen pack, then the base pack (`xtop/src/ui/layout/engine.rs`,
`render_named`). If nothing matches, the id **renders nothing** — the layout
area is left blank. As of the kernel snapshot read for this milestone, the
kernel does **not** warn on unknown ids inside a layout: DR-3's "visible
one-time kernel warning" is tracked on the kernel side as milestone M2.7 and
is not implemented yet (only the full-screen path shows `No widget
registered for '<name>'`, `xtop/src/ui/screen.rs`). This repo's only related
reporting is the loader skipping structurally invalid *files* on stderr
(`load_layouts_from_dir`); a file with a valid shape but an unknown widget
id parses fine.

**Why it stays this way.** Compile-time validation would couple this repo to
the widget registry and to every feature flag combination of the kernel;
runtime resolution keeps layouts portable across kernel builds. The
structural safety net that *does* exist is `xtop layout check` (kernel
`commands/layout.rs`), which validates files with this crate's
`parse_layout_err`.

## D-3 — Name-based merging, slot-preserving

**Decision.** Merging is by the layout `"name"` field with exact,
case-sensitive equality (`merge_layouts` in `src/loader.rs`): a user file
replacing a default keeps the default's palette slot; new names are
appended after the seven defaults. `mode_from_layout_index` maps only slots
0–6 to modes; custom layouts beyond them are addressed by name, and the
kernel persists/restores them by name (`config.layout_name`).

**Why.** Palette order and mode ↔ layout coupling must stay stable while
users freely customize; see `docs/authoring.md` §6 for the contract this
imposes on contributors (unique names, defaults order never renumbered).

## Status

- Crate: `xtop-layout` v0.1.0, `rust-version = "1.87"`, edition 2021, MIT,
  no third-party deps beyond serde/serde_json.
- Decisions D-1..D-3 recorded 2026-09-04 as part of milestone M6
  (documentation of the implicit schema).
