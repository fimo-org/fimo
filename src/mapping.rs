// src/mapping.rs
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct FieldMapping(pub HashMap<String, FieldDef>);

#[derive(Debug, Deserialize)]
pub struct FieldDef {
    pub r#type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub truthy: Option<Vec<String>>,
    #[serde(default)]
    pub falsy: Option<Vec<String>>,
}

pub fn requires_extended_json(mapping: &FieldMapping) -> bool {
    const BSON_TYPES: [&str; 6] = ["objectId", "date", "decimal", "regex", "timestamp", "binary"];
    mapping.0.values().any(|f| BSON_TYPES.contains(&f.r#type.as_str()))
} 
