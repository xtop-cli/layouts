//! Loading layout definitions: embedded defaults and user-provided files.
//!
//! Files are JSON/JSONC with a single layout per file (see `model.rs`).

use std::fs;
use std::path::Path;

use crate::model::LayoutDef;

/// Default layout assets shipped with the crate: (file stem, raw source).
/// Order matters: it defines the stable default palette order (index 0-6).
pub const DEFAULT_LAYOUT_SOURCES: &[(&str, &str)] = &[
    (
        "dashboard",
        include_str!("../assets/layouts/dashboard.jsonc"),
    ),
    ("vertical", include_str!("../assets/layouts/vertical.jsonc")),
    (
        "horizontal",
        include_str!("../assets/layouts/horizontal.jsonc"),
    ),
    (
        "cpu_focus",
        include_str!("../assets/layouts/cpu_focus.jsonc"),
    ),
    (
        "memory_focus",
        include_str!("../assets/layouts/memory_focus.jsonc"),
    ),
    (
        "network_focus",
        include_str!("../assets/layouts/network_focus.jsonc"),
    ),
    (
        "process_focus",
        include_str!("../assets/layouts/process_focus.jsonc"),
    ),
];

/// Parse the embedded default layouts (in default palette order).
pub fn default_layouts() -> Vec<LayoutDef> {
    DEFAULT_LAYOUT_SOURCES
        .iter()
        .map(|(_, src)| parse_layout(src).expect("embedded default layout assets must parse"))
        .collect()
}

/// Parse one layout definition from raw JSON/JSONC source.
pub fn parse_layout(source: &str) -> Option<LayoutDef> {
    let cleaned = strip_jsonc_comments(source);
    serde_json::from_str::<LayoutDef>(&cleaned).ok()
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

    #[test]
    fn test_default_layouts_count_and_order() {
        let layouts = default_layouts();
        assert_eq!(layouts.len(), 7);
        let names: Vec<&str> = layouts.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Dashboard",
                "Vertical",
                "Horizontal",
                "CPU Focus",
                "Memory Focus",
                "Network Focus",
                "Process Focus"
            ]
        );
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
}
