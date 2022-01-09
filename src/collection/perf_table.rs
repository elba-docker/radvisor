use serde::Serialize;
use std::collections::BTreeMap;

/// Contains all metadata used for perf table parsing
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TableMetadata {
    pub delimiter: &'static str,
    pub columns:   BTreeMap<String, Column>,
}

/// Contains the definitions for a single column
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Column {
    #[serde(rename_all = "PascalCase")]
    Scalar { r#type: ColumnType },
    #[serde(rename_all = "PascalCase")]
    Vector { r#type: ColumnType, count: usize },
}

/// Enum representing known variants of a column
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    /// Generic integer type
    Int,
    /// Nanosecond timestamp
    Epoch19,
}
