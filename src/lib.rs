//! xtop-layout: data-driven layout definitions for the xtop TUI system
//! monitor.
//!
//! The crate owns the layout *model* (tree of areas and widgets), the
//! *loading* logic (embedded default layouts plus user JSONC files) and the
//! *modes* (built-in layout modes and terminal-size degradation rules).
//!
//! It has no UI dependencies: the kernel translates the model to concrete
//! constraints at render time, so this crate stays pure data + serde.

mod loader;
mod mode;
mod model;

pub use loader::{
    default_layouts, load_layouts_from_dir, merge_layouts, parse_layout, parse_layout_err,
    DEFAULT_LAYOUT_SOURCES as default_layout_sources,
};
pub use mode::{
    detect_effective_layout, layout_index_from_mode, layout_mode_for_name, mode_from_layout_index,
    EffectiveLayout, LayoutMode, LAYOUT_MODE_NAMES,
};
pub use model::{Direction, LayoutArea, LayoutConstraint, LayoutDef, LayoutNode};
