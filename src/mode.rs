//! Layout modes and effective-layout detection.
//!
//! How the requested layout mode degrades depending on terminal size.

use serde::{Deserialize, Serialize};

use crate::model::LayoutDef;

/// The built-in layout modes. Extra user layouts are not represented here:
/// they are addressed by name through [`LayoutDef::name`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayoutMode {
    Dashboard,
    Vertical,
    Horizontal,
    CpuFocus,
    MemoryFocus,
    NetworkFocus,
    ProcessFocus,
}

/// Names of the built-in layout modes, in default palette order.
pub const LAYOUT_MODE_NAMES: &[&str] = &[
    "Dashboard",
    "Vertical",
    "Horizontal",
    "CPU Focus",
    "Memory Focus",
    "Network Focus",
    "Process Focus",
];

impl LayoutMode {
    #[cfg(test)]
    pub fn next(self) -> Self {
        match self {
            Self::Dashboard => Self::Vertical,
            Self::Vertical => Self::Horizontal,
            Self::Horizontal => Self::CpuFocus,
            Self::CpuFocus => Self::MemoryFocus,
            Self::MemoryFocus => Self::NetworkFocus,
            Self::NetworkFocus => Self::ProcessFocus,
            Self::ProcessFocus => Self::Dashboard,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Vertical => "Vertical",
            Self::Horizontal => "Horizontal",
            Self::CpuFocus => "CPU Focus",
            Self::MemoryFocus => "Memory Focus",
            Self::NetworkFocus => "Network Focus",
            Self::ProcessFocus => "Process Focus",
        }
    }

    /// Resolve a layout mode from a display name (as stored in a
    /// [`LayoutDef`]); `None` for custom layouts outside the built-ins.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "Dashboard" => Some(Self::Dashboard),
            "Vertical" => Some(Self::Vertical),
            "Horizontal" => Some(Self::Horizontal),
            "CPU Focus" => Some(Self::CpuFocus),
            "Memory Focus" => Some(Self::MemoryFocus),
            "Network Focus" => Some(Self::NetworkFocus),
            "Process Focus" => Some(Self::ProcessFocus),
            _ => None,
        }
    }
}

/// Which layout the user asked for, possibly degraded to fit the terminal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectiveLayout {
    Dashboard,
    /// Dashboard squeezed below 100x28 (same visual, less padding).
    Compact,
    Vertical,
    Horizontal,
    CpuFocus,
    MemoryFocus,
    NetworkFocus,
    ProcessFocus,
    /// Terminal too small for any regular layout: minimal renderer only.
    Minimal,
}

/// Find the index of a built-in mode inside a layout list by matching names.
/// Returns 0 when the mode has no counterpart in `defs` (e.g. all defaults
/// replaced by user layouts).
pub fn layout_index_from_mode(mode: LayoutMode, defs: &[LayoutDef]) -> usize {
    let label = mode.label();
    defs.iter().position(|d| d.name == label).unwrap_or(0)
}

/// Legacy inverse mapping of [`layout_index_from_mode`] for the first 7
/// palette slots (the defaults). Custom layouts beyond index 6 have no mode.
pub fn mode_from_layout_index(index: usize) -> LayoutMode {
    match index {
        0 => LayoutMode::Dashboard,
        1 => LayoutMode::Vertical,
        2 => LayoutMode::Horizontal,
        3 => LayoutMode::CpuFocus,
        4 => LayoutMode::MemoryFocus,
        5 => LayoutMode::NetworkFocus,
        6 => LayoutMode::ProcessFocus,
        _ => LayoutMode::Dashboard,
    }
}

/// Map a layout name back to its mode; custom names fall back to `fallback`.
pub fn layout_mode_for_name(name: &str, fallback: LayoutMode) -> LayoutMode {
    LayoutMode::from_label(name).unwrap_or(fallback)
}

pub fn detect_effective_layout(width: u16, height: u16, user_mode: LayoutMode) -> EffectiveLayout {
    if width < 60 || height < 14 {
        return EffectiveLayout::Minimal;
    }
    match user_mode {
        LayoutMode::Dashboard => {
            if width < 80 {
                EffectiveLayout::Vertical
            } else if width < 100 || height < 28 {
                EffectiveLayout::Compact
            } else {
                EffectiveLayout::Dashboard
            }
        }
        LayoutMode::Vertical => EffectiveLayout::Vertical,
        LayoutMode::Horizontal => EffectiveLayout::Horizontal,
        LayoutMode::CpuFocus => EffectiveLayout::CpuFocus,
        LayoutMode::MemoryFocus => EffectiveLayout::MemoryFocus,
        LayoutMode::NetworkFocus => EffectiveLayout::NetworkFocus,
        LayoutMode::ProcessFocus => EffectiveLayout::ProcessFocus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_mode_next() {
        assert_eq!(LayoutMode::Dashboard.next(), LayoutMode::Vertical);
        assert_eq!(LayoutMode::Vertical.next(), LayoutMode::Horizontal);
        assert_eq!(LayoutMode::Horizontal.next(), LayoutMode::CpuFocus);
        assert_eq!(LayoutMode::CpuFocus.next(), LayoutMode::MemoryFocus);
        assert_eq!(LayoutMode::MemoryFocus.next(), LayoutMode::NetworkFocus);
        assert_eq!(LayoutMode::NetworkFocus.next(), LayoutMode::ProcessFocus);
        assert_eq!(LayoutMode::ProcessFocus.next(), LayoutMode::Dashboard);
    }

    #[test]
    fn test_layout_mode_label_roundtrip() {
        for mode in [
            LayoutMode::Dashboard,
            LayoutMode::Vertical,
            LayoutMode::Horizontal,
            LayoutMode::CpuFocus,
            LayoutMode::MemoryFocus,
            LayoutMode::NetworkFocus,
            LayoutMode::ProcessFocus,
        ] {
            assert_eq!(LayoutMode::from_label(mode.label()), Some(mode));
        }
        assert_eq!(LayoutMode::from_label("My Custom"), None);
    }

    #[test]
    fn test_layout_mode_helpers() {
        let defs = crate::loader::default_layouts();
        assert_eq!(layout_index_from_mode(LayoutMode::CpuFocus, &defs), 3);
        assert_eq!(mode_from_layout_index(3), LayoutMode::CpuFocus);
        assert_eq!(mode_from_layout_index(42), LayoutMode::Dashboard);
        assert_eq!(
            layout_mode_for_name("My Custom", LayoutMode::Vertical),
            LayoutMode::Vertical
        );
        assert_eq!(
            layout_mode_for_name("Dashboard", LayoutMode::Vertical),
            LayoutMode::Dashboard
        );
    }

    #[test]
    fn test_detect_effective_layout_large() {
        assert_eq!(
            detect_effective_layout(120, 40, LayoutMode::Dashboard),
            EffectiveLayout::Dashboard
        );
    }

    #[test]
    fn test_detect_effective_layout_compact() {
        assert_eq!(
            detect_effective_layout(90, 30, LayoutMode::Dashboard),
            EffectiveLayout::Compact
        );
    }

    #[test]
    fn test_detect_effective_layout_narrow() {
        assert_eq!(
            detect_effective_layout(70, 30, LayoutMode::Dashboard),
            EffectiveLayout::Vertical
        );
    }

    #[test]
    fn test_detect_effective_layout_minimal() {
        assert_eq!(
            detect_effective_layout(50, 15, LayoutMode::Dashboard),
            EffectiveLayout::Minimal
        );
    }

    #[test]
    fn test_detect_effective_layout_focus_respected() {
        assert_eq!(
            detect_effective_layout(80, 30, LayoutMode::CpuFocus),
            EffectiveLayout::CpuFocus
        );
        assert_eq!(
            detect_effective_layout(80, 30, LayoutMode::NetworkFocus),
            EffectiveLayout::NetworkFocus
        );
    }

    #[test]
    fn test_detect_effective_layout_focus_downgrade() {
        assert_eq!(
            detect_effective_layout(50, 30, LayoutMode::CpuFocus),
            EffectiveLayout::Minimal
        );
    }
}
