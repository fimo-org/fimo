// src/transform.rs
use crate::mapping::FieldMapping;
use bson::{Bson, DateTime, Decimal128, Document, oid::ObjectId, Regex, Timestamp};
use serde_json::Value;
use anyhow::{anyhow, Result};
use minijinja::{Environment, context};
use std::collections::HashMap;
use std::str::FromStr;
use chrono::{NaiveDateTime, TimeZone, Utc};

pub fn validate_required_fields(record: &HashMap<String, String>, mapping: &FieldMapping) -> Result<()> {
    for (key, field_def) in &mapping.0 {
        if field_def.required && !record.contains_key(key) {
            return Err(anyhow!("Missing required field: {}", key));
        }
    }
    Ok(())
}

pub fn apply_mapping(record: &HashMap<String, String>, mapping: &FieldMapping, row_num: usize) -> Result<Document> {
    let mut doc = Document::new();

    for (key, value) in record {
        let field_def = mapping.0.get(key);

        let bson_value = if let Some(def) = field_def {
            match def.r#type.as_str() {
                "string" => Bson::String(value.to_string()),
                "int" => value.parse::<i32>().map(Bson::Int32)
                    .map_err(|_| anyhow!("Row {}: Failed to convert '{}' to int for field '{}'", row_num, value, key))?,
                "long" => value.parse::<i64>().map(Bson::Int64)
                    .map_err(|_| anyhow!("Row {}: Failed to convert '{}' to long for field '{}'", row_num, value, key))?,
                "double" => value.parse::<f64>().map(Bson::Double)
                    .map_err(|_| anyhow!("Row {}: Failed to convert '{}' to double for field '{}'", row_num, value, key))?,
                "decimal" => Decimal128::from_str(value)
                    .map(Bson::Decimal128)
                    .map_err(|_| anyhow!("Row {}: Failed to convert '{}' to decimal128 for field '{}'", row_num, value, key))?,
                "bool" => {
                    let is_true = def.truthy.as_ref()
                        .map(|list| list.iter().any(|t| t.eq_ignore_ascii_case(value)))
                        .unwrap_or_else(|| matches!(value.to_lowercase().as_str(), "true" | "t" | "yes" | "1" | "y"));

                    let is_false = def.falsy.as_ref()
                        .map(|list| list.iter().any(|f| f.eq_ignore_ascii_case(value)))
                        .unwrap_or_else(|| matches!(value.to_lowercase().as_str(), "false" | "f" | "no" | "0" | "n"));

                    if is_true {
                        Bson::Boolean(true)
                    } else if is_false {
                        Bson::Boolean(false)
                    } else {
                        return Err(anyhow!("Row {}: Invalid value '{}' for bool field '{}'", row_num, value, key));
                    }
                },
                "objectId" => ObjectId::parse_str(value)
                    .map(Bson::ObjectId)
                    .map_err(|_| anyhow!("Row {}: Failed to parse '{}' as ObjectId for field '{}'", row_num, value, key))?,
                "date" => {
                    if let Some(formats) = &def.formats {
                        let mut parsed = None;
                        for fmt in formats {
                            if let Ok(ndt) = NaiveDateTime::parse_from_str(value, fmt) {
                                parsed = Some(Bson::DateTime(DateTime::from_chrono(Utc.from_utc_datetime(&ndt))));
                                break;
                            }
                        }
                        if let Some(date) = parsed {
                            date
                        } else {
                            return Err(anyhow!("Row {}: Could not parse '{}' with any format for field '{}'", row_num, value, key));
                        }
                    } else {
                        DateTime::parse_rfc3339_str(value)
                            .map(Bson::DateTime)
                            .map_err(|_| anyhow!("Row {}: Failed to parse '{}' as ISODate for field '{}'", row_num, value, key))?
                    }
                },
                "timestamp" => {
                    let ts = value.parse::<u32>()?;
                    Bson::Timestamp(Timestamp { time: ts, increment: 1 })
                },
                "regex" => Bson::RegularExpression(Regex { pattern: value.clone(), options: "".to_string() }),
                _ => Bson::String(value.to_string())
            }
        } else {
            Bson::String(value.to_string())
        };

        doc.insert(key, bson_value);
    }

    Ok(doc)
}

pub fn render_operation(
    env: &Environment<'_>,
    operation: &str,
    bson_doc: &Document,
    raw_insert: bool,
) -> Result<Option<Value>> {
    if raw_insert {
        let json = serde_json::to_value(bson_doc)?;
        return Ok(Some(json));
    }

    if let Some(tmpl) = env.get_template(operation).ok() {
        let json = serde_json::to_value(bson_doc)?;
        let ctx = context! { row => json };
        let rendered = tmpl.render(ctx)?;
        let result: Value = serde_json::from_str(&rendered)?;
        Ok(Some(result))
    } else {
        Err(anyhow!("Missing template for operation '{}'.", operation))
    }
}
