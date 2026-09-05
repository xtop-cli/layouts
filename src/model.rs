//! Layout model for xtop: tree of areas that describes where widgets are
//! rendered inside the terminal.
//!
//! The model is UI-framework agnostic (no ratatui): the kernel translates it
//! to concrete constraints at render time.

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Split direction of a container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// Size constraint of an area inside a split.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutConstraint {
    /// Fixed number of terminal rows/columns.
    Length(u16),
    /// Percentage of the parent container.
    Percentage(u16),
    /// Fill the remaining space.
    Fill,
}

/// One child of a split: a constraint plus the node it contains.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutArea {
    pub constraint: LayoutConstraint,
    pub node: LayoutNode,
}

/// A node of the layout tree: either a nested split or a widget leaf.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutNode {
    Split {
        direction: Direction,
        areas: Vec<LayoutArea>,
    },
    Widget {
        name: String,
        /// Optional per-widget display options (DR-UX1): an opaque JSON
        /// object passed through verbatim to the widget renderer (see the
        /// widget-api contract). `None` when the layout has no `options`
        /// key on this node — renderers then use their default behavior.
        options: Option<serde_json::Value>,
    },
}

/// A named, user-visible layout definition (a full tree).
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutDef {
    pub name: String,
    pub root: LayoutNode,
}

// ---------------------------------------------------------------------------
// Deserialization helpers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LayoutDefRaw {
    name: String,
    root: LayoutAreaRaw,
}

#[derive(Deserialize)]
struct LayoutAreaRaw {
    #[serde(default)]
    size: Option<SizeRaw>,
    widget: Option<String>,
    direction: Option<String>,
    #[serde(default)]
    areas: Option<Vec<LayoutAreaRaw>>,
    /// Passthrough display options of a widget leaf (DR-UX1). Absent key and
    /// explicit `null` both deserialize to `None`; the model never
    /// interprets the content (renderers own the semantics), so unknown keys
    /// inside the object are preserved verbatim.
    #[serde(default)]
    options: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SizeRaw {
    Num(u16),
    Str(String),
}

impl TryFrom<LayoutAreaRaw> for LayoutArea {
    type Error = String;

    fn try_from(raw: LayoutAreaRaw) -> Result<Self, String> {
        let constraint = match raw.size {
            None => LayoutConstraint::Fill,
            Some(SizeRaw::Num(n)) => LayoutConstraint::Length(n),
            Some(SizeRaw::Str(s)) if s == "*" => LayoutConstraint::Fill,
            Some(SizeRaw::Str(s)) if s.ends_with('%') => {
                let pct = s
                    .trim_end_matches('%')
                    .parse::<u16>()
                    .map_err(|_| format!("invalid percentage: {s}"))?;
                LayoutConstraint::Percentage(pct)
            }
            Some(SizeRaw::Str(s)) => {
                return Err(format!("invalid size constraint: {s}"));
            }
        };

        let node = if let Some(name) = raw.widget {
            LayoutNode::Widget {
                name,
                options: raw.options,
            }
        } else if let Some(dir) = raw.direction {
            let direction = match dir.to_lowercase().as_str() {
                "horizontal" => Direction::Horizontal,
                "vertical" => Direction::Vertical,
                _ => return Err(format!("invalid direction: {dir}")),
            };
            let areas_raw = raw.areas.unwrap_or_default();
            let mut areas = Vec::with_capacity(areas_raw.len());
            for a in areas_raw {
                areas.push(a.try_into()?);
            }
            LayoutNode::Split { direction, areas }
        } else {
            return Err("layout area must have 'widget' or 'direction'".into());
        };

        Ok(LayoutArea { constraint, node })
    }
}

impl TryFrom<LayoutDefRaw> for LayoutDef {
    type Error = String;

    fn try_from(raw: LayoutDefRaw) -> Result<Self, String> {
        let area: LayoutArea = raw.root.try_into()?;
        Ok(LayoutDef {
            name: raw.name,
            root: area.node,
        })
    }
}

// Custom Deserialize for LayoutDef (handles jsonc-compatible parsing)
impl<'de> Deserialize<'de> for LayoutDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name,
            Root,
        }

        struct LayoutVisitor;
        impl<'de> Visitor<'de> for LayoutVisitor {
            type Value = LayoutDef;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct LayoutDef")
            }

            fn visit_map<V>(self, mut map: V) -> Result<LayoutDef, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut raw = LayoutDefRaw {
                    name: String::new(),
                    root: LayoutAreaRaw {
                        size: None,
                        widget: None,
                        direction: None,
                        areas: None,
                        options: None,
                    },
                };
                let mut found_name = false;
                let mut found_root = false;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Name => {
                            raw.name = map.next_value::<String>()?;
                            found_name = true;
                        }
                        Field::Root => {
                            raw.root = map.next_value::<LayoutAreaRaw>()?;
                            found_root = true;
                        }
                    }
                }

                if !found_name {
                    return Err(de::Error::missing_field("name"));
                }
                if !found_root {
                    return Err(de::Error::missing_field("root"));
                }

                LayoutDef::try_from(raw).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_struct("LayoutDef", &["name", "root"], LayoutVisitor)
    }
}

// Mirror serialization: `LayoutDef` serializes to the exact JSON shape the
// custom deserializer accepts (name/root, size as number|percent|"*", widget
// leaf or direction+areas split).

impl Serialize for LayoutArea {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        match &self.constraint {
            LayoutConstraint::Length(n) => {
                map.serialize_entry("size", n)?;
            }
            LayoutConstraint::Percentage(p) => {
                map.serialize_entry("size", &format!("{p}%"))?;
            }
            LayoutConstraint::Fill => {
                map.serialize_entry("size", "*")?;
            }
        }
        match &self.node {
            LayoutNode::Widget { name, options } => {
                map.serialize_entry("widget", name)?;
                // Only emitted when present: layouts without options
                // serialize byte-identically to the pre-DR-UX1 format.
                if let Some(options) = options {
                    map.serialize_entry("options", options)?;
                }
            }
            LayoutNode::Split { direction, areas } => {
                let dir = match direction {
                    Direction::Horizontal => "horizontal",
                    Direction::Vertical => "vertical",
                };
                map.serialize_entry("direction", dir)?;
                map.serialize_entry("areas", areas)?;
            }
        }
        map.end()
    }
}

impl Serialize for LayoutNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        match self {
            LayoutNode::Widget { name, options } => {
                map.serialize_entry("widget", name)?;
                // Only emitted when present: layouts without options
                // serialize byte-identically to the pre-DR-UX1 format.
                if let Some(options) = options {
                    map.serialize_entry("options", options)?;
                }
            }
            LayoutNode::Split { direction, areas } => {
                let dir = match direction {
                    Direction::Horizontal => "horizontal",
                    Direction::Vertical => "vertical",
                };
                map.serialize_entry("direction", dir)?;
                map.serialize_entry("areas", areas)?;
            }
        }
        map.end()
    }
}

impl Serialize for LayoutDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("root", &self.root)?;
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_layout_def_serde_roundtrip() {
        let def = LayoutDef {
            name: "RT".into(),
            root: LayoutNode::Split {
                direction: Direction::Vertical,
                areas: vec![LayoutArea {
                    constraint: LayoutConstraint::Length(3),
                    node: LayoutNode::Widget {
                        name: "header".into(),
                        options: None,
                    },
                }],
            },
        };
        let json = serde_json::to_string(&def).unwrap();
        let back: LayoutDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def, back);
    }

    #[test]
    fn test_widget_options_roundtrip_preserves_content() {
        // Options are an opaque passthrough: nested objects, arrays and
        // unknown keys must survive the round trip untouched.
        let options = json!({
            "cpu": "total",
            "cores": { "filter": [0, 2, "4-7"], "note": null },
            "show_freq": true,
        });
        let def = LayoutDef {
            name: "RT".into(),
            root: LayoutNode::Split {
                direction: Direction::Vertical,
                areas: vec![
                    LayoutArea {
                        constraint: LayoutConstraint::Length(3),
                        node: LayoutNode::Widget {
                            name: "header".into(),
                            options: None,
                        },
                    },
                    LayoutArea {
                        constraint: LayoutConstraint::Fill,
                        node: LayoutNode::Widget {
                            name: "cpu".into(),
                            options: Some(options.clone()),
                        },
                    },
                ],
            },
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("\"options\":"));
        let back: LayoutDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def, back);
        // Spot-check the passthrough content on the parsed side.
        let LayoutNode::Split { areas, .. } = &back.root else {
            panic!("expected split root");
        };
        let LayoutNode::Widget {
            name,
            options: parsed_options,
        } = &areas[1].node
        else {
            panic!("expected widget node");
        };
        assert_eq!(name, "cpu");
        assert_eq!(parsed_options.as_ref().unwrap(), &options);
    }

    #[test]
    fn test_widget_without_options_serializes_unchanged() {
        // Backward compatibility (DR-UX2): no options -> byte-identical JSON
        // to the pre-DR-UX1 mirror serializer.
        let def = LayoutDef {
            name: "RT".into(),
            root: LayoutNode::Split {
                direction: Direction::Vertical,
                areas: vec![LayoutArea {
                    constraint: LayoutConstraint::Length(3),
                    node: LayoutNode::Widget {
                        name: "header".into(),
                        options: None,
                    },
                }],
            },
        };
        let json = serde_json::to_string(&def).unwrap();
        assert_eq!(
            json,
            r#"{"name":"RT","root":{"direction":"vertical","areas":[{"size":3,"widget":"header"}]}}"#
        );
        let back: LayoutDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def, back);
    }

    #[test]
    fn test_widget_options_absent_and_null_parse_to_none() {
        let absent = parse_root_area(r#"{"widget":"cpu"}"#);
        assert!(matches!(absent, LayoutNode::Widget { options: None, .. }));
        let null = parse_root_area(r#"{"widget":"cpu","options":null}"#);
        assert!(matches!(null, LayoutNode::Widget { options: None, .. }));
    }

    /// Parse a bare root area object (the grammar every area shares).
    fn parse_root_area(src: &str) -> LayoutNode {
        let raw: LayoutAreaRaw = serde_json::from_str(src).unwrap();
        LayoutArea::try_from(raw).unwrap().node
    }
}
