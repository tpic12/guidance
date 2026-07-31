use serde::{Deserialize, Serialize};

/// A block of rules text. Mirrors the recursive entry structure of the
/// imported JSON, minus cross-references to content types we can't resolve
/// yet (those are dropped at transform time).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    Text { text: String },
    List { items: Vec<Entry> },
    /// A named list item ("Bolstering Music. While you can hear the...").
    Item { name: String, entries: Vec<Entry> },
    /// A named (or anonymous) group of nested entries.
    Section { name: Option<String>, entries: Vec<Entry> },
    Table { caption: Option<String>, col_labels: Vec<String>, rows: Vec<Vec<String>> },
}
