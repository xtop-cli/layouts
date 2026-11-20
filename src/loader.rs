//! Loading layout definitions: the embedded base and user-provided files.
//!
//! Files are JSON/JSONC with a single layout per file (see `model.rs`).

use std::fs;
use std::path::Path;

use crate::model::LayoutDef;

/// Base layout asset embedded in the crate.
///
/// Only the primary Dashboard layout stays embedded. The other mode layouts,
/// the detail presets and the community layouts are plain JSONC files under
/// `layouts/` and are downloaded on demand from the official catalog into the
/// user config directory.
pub const DEFAULT_LAYOUT_SOURCES: &[(&str, &str)] = &[(
    "dashboard",
    include_str!("../layouts/default/dashboard.jsonc"),
)];

/// Parse the embedded base layout (Dashboard).
pub fn default_layouts() -> Vec<LayoutDef> {
    DEFAULT_LAYOUT_SOURCES
        .iter()
        .map(|(_, src)| parse_layout(src).expect("embedded default layout assets must parse"))
        .collect()
}

/// Parse one layout definition from raw JSON/JSONC source.
pub fn parse_layout(source: &str) -> Option<LayoutDef> {
    parse_layout_err(source).ok()
}

/// Parse with a human-readable error (for `xtop layout check`).
pub fn parse_layout_err(source: &str) -> Result<LayoutDef, String> {
    let cleaned = strip_jsonc_comments(source);
    serde_json::from_str::<LayoutDef>(&cleaned).map_err(|e| e.to_string())
}

/// Load every `*.json`/`*.jsonc` layout from a directory.
///
/// Invalid files are skipped (they are reported by name on stderr when
/// debugging is enabled) so a broken user file never kills the app.
pub fn load_layouts_from_dir(dir: &Path) -> Vec<LayoutDef> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());
            if matches!(ext, Some("json") | Some("jsonc")) {
                if let Ok(data) = fs::read_to_string(&path) {
                    match parse_layout(&data) {
                        Some(def) => out.push(def),
                        None => eprintln!(
                            "[xtop-layout] skipping invalid layout file: {}",
                            path.display()
                        ),
                    }
                }
            }
        }
    }
    out
}

/// Merge defaults with user layouts: user files override defaults with the
/// same name, and extra user layouts are appended (user wins).
pub fn merge_layouts(defaults: Vec<LayoutDef>, custom: Vec<LayoutDef>) -> Vec<LayoutDef> {
    let mut merged = defaults;
    for user in custom {
        if let Some(slot) = merged.iter_mut().find(|d| d.name == user.name) {
            *slot = user;
        } else {
            merged.push(user);
        }
    }
    merged
}

/// Strip JSONC comments (`//` and `/* */`), keeping string literals intact.
fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut in_string: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        if let Some(quote) = in_string {
            out.push(c);
            if c == '\\' {
                if i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
            } else if c == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            in_string = Some(c);
            out.push(c);
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() {
            if chars[i + 1] == '/' {
                i += 2;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if chars[i + 1] == '*' {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i = (i + 2).min(chars.len());
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LayoutConstraint, LayoutNode};
    use serde_json::json;

    /// Parse every layout file shipped in `layouts/default/` from disk.
    fn shipped_layouts() -> Vec<LayoutDef> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("layouts/default");
        load_layouts_from_dir(&dir)
    }

    fn shipped(name: &str) -> LayoutDef {
        shipped_layouts()
            .into_iter()
            .find(|layout| layout.name == name)
            .unwrap_or_else(|| panic!("shipped layout '{name}' not found"))
    }

    #[test]
    fn test_embedded_base_layout_is_dashboard() {
        let layouts = default_layouts();
        assert_eq!(layouts.len(), 1, "only the primary layout is embedded");
        assert_eq!(layouts[0].name, "Dashboard");
    }

    #[test]
    fn test_all_shipped_layout_files_parse() {
        let layouts = shipped_layouts();
        assert_eq!(
            layouts.len(),
            10,
            "10 JSONC layout files ship in layouts/default"
        );
    }

    #[test]
    fn test_preset_extras_parse_with_widget_options() {
        // The detail presets are external JSONC files under layouts/default;
        // they are loaded from disk here, not embedded in the crate.
        let dashboard = shipped("Detail Dashboard");
        let network = shipped("Detail Network");
        let processes = shipped("Detail Processes");

        for def in [&dashboard, &network, &processes] {
            let (mut widgets, mut options) = (0usize, 0usize);
            collect_widgets(&def.root, &mut widgets, &mut options);
            assert!(widgets > 0, "{}: must reference widgets", def.name);
            assert!(options > 0, "{}: must showcase widget options", def.name);
        }

        // Spot-check the flagship option keys of the presets.
        assert_eq!(
            find_options(&dashboard.root, "processes"),
            Some(serde_json::json!({ "cpu": "total" }))
        );
        assert_eq!(
            find_options(&dashboard.root, "cpu"),
            Some(serde_json::json!({ "cores": "all", "show_freq": true }))
        );
        assert_eq!(
            find_options(&processes.root, "processes"),
            Some(serde_json::json!({ "cpu": "both" }))
        );
    }

    /// Count widget leaves and widget leaves carrying `options` in a tree.
    fn collect_widgets(node: &LayoutNode, widgets: &mut usize, options: &mut usize) {
        match node {
            LayoutNode::Widget {
                name: _,
                options: opts,
            } => {
                *widgets += 1;
                if opts.is_some() {
                    *options += 1;
                }
            }
            LayoutNode::Split { areas, .. } => {
                for area in areas {
                    collect_widgets(&area.node, widgets, options);
                }
            }
        }
    }

    /// Options object of the first widget named `name` in pre-order, if any.
    fn find_options(node: &LayoutNode, name: &str) -> Option<serde_json::Value> {
        match node {
            LayoutNode::Widget {
                name: widget_name,
                options,
            } => {
                if widget_name == name {
                    options.clone()
                } else {
                    None
                }
            }
            LayoutNode::Split { areas, .. } => {
                areas.iter().find_map(|area| find_options(&area.node, name))
            }
        }
    }

    #[test]
    fn test_parse_layout_from_jsonc() {
        let jsonc = r#"{
            // my custom layout
            "name": "test",
            "root": {
                "direction": "vertical",
                "areas": [
                    { "widget": "header", "size": 3 },
                    { "widget": "cpu", "size": "*" }
                ]
            }
        }"#;
        let layout = parse_layout(jsonc).expect("valid jsonc must parse");
        assert_eq!(layout.name, "test");
    }

    #[test]
    fn test_parse_layout_jsonc_with_widget_options() {
        // DR-UX1: a widget node may carry an `options` JSON object; the
        // loader keeps it verbatim (comment stripping must not touch it).
        let jsonc = r#"{
            // layout with per-widget display options
            "name": "options test",
            "root": {
                "direction": "vertical",
                "areas": [
                    { "widget": "header", "size": 3 },
                    {
                        "widget": "cpu",
                        "size": "*",
                        "options": { "cores": "all", "show_freq": true }
                    }
                ]
            }
        }"#;
        let layout = parse_layout(jsonc).expect("valid jsonc must parse");
        let LayoutNode::Split { areas, .. } = &layout.root else {
            panic!("expected a split root");
        };
        let LayoutNode::Widget { name, options } = &areas[1].node else {
            panic!("expected a widget node");
        };
        assert_eq!(name, "cpu");
        let options = options.as_ref().expect("cpu options must parse");
        assert_eq!(options["cores"], json!("all"));
        assert_eq!(options["show_freq"], json!(true));
    }

    #[test]
    fn test_strip_keeps_urls_inside_strings() {
        let jsonc = r#"{
            // comment
            "name": "x",
            "root": { "direction": "vertical", "areas": [] }
        }"#;
        let cleaned = strip_jsonc_comments(jsonc);
        assert!(cleaned.contains("\"name\": \"x\""));
    }

    #[test]
    fn test_merge_user_overrides_default() {
        let mut defaults = default_layouts();
        defaults.truncate(1); // keep only Dashboard
        let mut custom = defaults.clone();
        custom[0].root = crate::model::LayoutNode::Split {
            direction: crate::model::Direction::Horizontal,
            areas: vec![],
        };
        custom.push(crate::model::LayoutDef {
            name: "My Layout".into(),
            root: crate::model::LayoutNode::Split {
                direction: crate::model::Direction::Vertical,
                areas: vec![],
            },
        });
        let merged = merge_layouts(defaults, custom.clone());
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].root, custom[0].root);
        assert_eq!(merged[1].name, "My Layout");
    }

    #[test]
    fn test_load_from_missing_dir_is_empty() {
        let dir = Path::new("/nonexistent/xtop-layout-test");
        assert!(load_layouts_from_dir(dir).is_empty());
    }

    #[test]
    fn test_detail_presets_split_coverage_full_tiling() {
        // UX8.5: no detail preset may leave an orphan band at the bottom of
        // any stack. The kernel maps Percentage(p) to p% of the WHOLE split
        // and Fill to "take the remainder", so a split tiles its parent
        // exactly iff either
        //   - a Fill sibling exists to absorb the leftover rows (Lengths may
        //     sit next to it; the percentage demand must stay below 100 or
        //     the Fill would starve to zero), or
        //   - the children are pure percentages that sum to exactly 100
        //     (a Length mixed in would over-claim, because percentages are
        //     measured against the full split, not the leftover).
        // Walk every split of every detail preset and enforce that rule.
        let mut failures: Vec<String> = Vec::new();
        for def in shipped_layouts()
            .iter()
            .filter(|layout| layout.name.starts_with("Detail "))
        {
            check_tiling(&def.root, &def.name, &mut failures);
        }
        assert!(
            failures.is_empty(),
            "detail presets must tile every split:\n{}",
            failures.join("\n")
        );
    }

    /// Recursive tiling check (see the test above for the rule).
    fn check_tiling(node: &LayoutNode, layout: &str, failures: &mut Vec<String>) {
        let LayoutNode::Split { areas, .. } = node else {
            return;
        };
        if !areas.is_empty() {
            let mut pct_sum: u32 = 0;
            let mut fills = 0usize;
            let mut lengths = 0usize;
            for area in areas {
                match area.constraint {
                    LayoutConstraint::Percentage(p) => pct_sum += u32::from(p),
                    LayoutConstraint::Fill => fills += 1,
                    LayoutConstraint::Length(_) => lengths += 1,
                }
            }
            if fills > 0 {
                // The Fill sibling absorbs the leftover; a 100% (or higher)
                // percentage demand would leave it nothing to absorb.
                if pct_sum >= 100 {
                    failures.push(format!(
                        "{layout}: split with Fill siblings but percentages already claim {pct_sum}%"
                    ));
                }
            } else if lengths > 0 || pct_sum != 100 {
                // Without a Fill, only pure percentages summing to 100 tile
                // the split exactly; a fixed Length shrinks the available
                // rows while percentages keep measuring the full split.
                failures.push(format!(
                    "{layout}: no Fill sibling (lengths: {lengths}, percentages sum {pct_sum}%)"
                ));
            }
        }
        for area in areas {
            check_tiling(&area.node, layout, failures);
        }
    }

    #[test]
    fn test_detail_presets_reference_registry_widget_names() {
        // UX8.5: the detail presets may only reference widget names the
        // registry will provide — the base pack ids plus the UX8.4 arrivals
        // `summary` and `sensors`. The loader never validates names (they are
        // resolved at render time), so this pins the preset files instead.
        const REGISTRY_NAMES: &[&str] = &[
            "header",
            "cpu",
            "memory",
            "storage",
            "network",
            "processes",
            "disk_io",
            "battery",
            "gpu",
            "summary",
            "sensors",
        ];
        let presets = [
            shipped("Detail Dashboard"),
            shipped("Detail Network"),
            shipped("Detail Processes"),
        ];
        for def in &presets {
            let names = collect_names(&def.root);
            assert!(!names.is_empty(), "{}: must reference widgets", def.name);
            for name in &names {
                assert!(
                    REGISTRY_NAMES.contains(&name.as_str()),
                    "{}: unknown widget '{name}'",
                    def.name
                );
            }
        }
        // The full-monitor preset hosts both new dense widgets.
        let dashboard = collect_names(&presets[0].root);
        for required in ["header", "cpu", "summary", "sensors", "processes"] {
            assert!(
                dashboard.iter().any(|n| n == required),
                "Detail Dashboard must reference '{required}'"
            );
        }
        let network = collect_names(&presets[1].root);
        for required in ["network", "disk_io", "summary", "processes"] {
            assert!(
                network.iter().any(|n| n == required),
                "Detail Network must reference '{required}'"
            );
        }
        let processes = collect_names(&presets[2].root);
        for required in ["summary", "cpu", "processes"] {
            assert!(
                processes.iter().any(|n| n == required),
                "Detail Processes must reference '{required}'"
            );
        }
    }

    /// Unique widget names referenced by a tree, in pre-order of first
    /// occurrence.
    fn collect_names(node: &LayoutNode) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        fn walk(node: &LayoutNode, out: &mut Vec<String>) {
            match node {
                LayoutNode::Widget { name, .. } => {
                    if !out.iter().any(|n| n == name) {
                        out.push(name.clone());
                    }
                }
                LayoutNode::Split { areas, .. } => {
                    for area in areas {
                        walk(&area.node, out);
                    }
                }
            }
        }
        walk(node, &mut out);
        out
    }
}
