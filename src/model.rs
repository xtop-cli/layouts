//! Layout model for xtop: tree of areas that describes where widgets are
//! rendered inside the terminal.
//!
//! The model is UI-framework agnostic (no ratatui): the kernel translates it
//! to concrete constraints at render time.

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Split direction of a container.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
            LayoutNode::Widget { name }
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
