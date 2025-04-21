mod cli;
mod mapping;
mod mongo;
mod template;
mod transform;

use crate::cli::Cli;
use crate::mapping::{requires_extended_json, FieldMapping};
use crate::mongo::connect;
use crate::template::load_templates;
use crate::transform::{apply_mapping, render_operation, validate_required_fields};

use anyhow::{anyhow, Result};
use bson::{Bson, Document};
use clap::Parser;
use csv::ReaderBuilder;
use mongodb::{
    options::InsertOneModel, options::UpdateOneModel, options::WriteModel, Collection,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let file = File::open(&args.input)?;
    let reader = BufReader::new(file);
    let mut rdr = ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(!args.no_header)
        .from_reader(reader);

    let mapping_text = std::fs::read_to_string(&args.mapping)?;
    let field_mapping: FieldMapping = serde_yaml::from_str(&mapping_text)?;

    if !args.extended_json && requires_extended_json(&field_mapping) {
        eprintln!(
            "❗️ Error: BSON types detected in mapping file, but --extended-json was not provided."
        );
        std::process::exit(1);
    }

    let env = if let Some(dir) = &args.template_dir {
        load_templates(dir)?
    } else {
        minijinja::Environment::new()
    };

    let collection: Collection<Document> =
        connect(&args.mongo_uri, &args.db, &args.collection).await?;

    let mut bulk_buffer: Vec<WriteModel> = Vec::new();
    let batch_size = args.batch_size.unwrap_or(0);

    let operation = args.operation.as_deref().unwrap_or("insert");
    let strip_set_on_insert = operation == "update";

    let headers = if !args.no_header {
        Some(rdr.headers()?.clone())
    } else {
        None
    };
    
    let mut row_num = 0;
    for result in rdr.records() {
        row_num += 1;
        let result = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Row {}: CSV read error: {}", row_num, e);
                continue;
            }
        };
    
        let record: HashMap<String, String> = if args.no_header {
            result
                .iter()
                .enumerate()
                .map(|(i, val)| (format!("col_{}", i), val.to_string()))
                .collect()
        } else {
            match &headers {
                Some(hdrs) => hdrs
                    .iter()
                    .zip(result.iter())
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                None => {
                    eprintln!("Row {}: Header extraction failed", row_num);
                    continue;
                }
            }
        };

        if let Err(e) = validate_required_fields(&record, &field_mapping) {
            eprintln!("Row {}: {}", row_num, e);
            continue;
        }

        let mapped = match apply_mapping(&record, &field_mapping, row_num) {
            Ok(doc) => doc,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        if args.validate_only {
            continue;
        }

        let rendered_json = match render_operation(&env, operation, &mapped, args.raw_insert) {
            Ok(Some(doc)) => doc,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("Row {}: Template error: {}", row_num, e);
                continue;
            }
        };

        let mut rendered: Document = if args.extended_json {
            match bson::to_bson(&rendered_json) {
                Ok(Bson::Document(doc)) => doc,
                _ => {
                    eprintln!("Row {}: Rendered JSON is not a document", row_num);
                    continue;
                }
            }
        } else {
            match serde_json::to_string(&rendered_json)
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s))
            {
                Ok(json_val) => match bson::to_document(&json_val) {
                    Ok(doc) => doc,
                    Err(e) => {
                        eprintln!("Row {}: JSON to BSON error: {}", row_num, e);
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("Row {}: Template render error: {}", row_num, e);
                    continue;
                }
            }
        };

        if strip_set_on_insert {
            if let Some(update_doc) = rendered.get_document_mut("update").ok() {
                update_doc.remove("$setOnInsert");
            }
        }

        if args.dry_run || args.debug {
            println!("Row {}: {:?}", row_num, rendered);
        }

        if !args.dry_run {
            if batch_size > 0 {
                let model = match operation {
                    "insert" => WriteModel::InsertOne(
                        InsertOneModel::builder()
                            .namespace(collection.namespace())
                            .document(rendered)
                            .build(),
                    ),
                    "upsert" | "update" => {
                        let filter = rendered.get_document("filter").cloned().unwrap_or_default();
                        let update = rendered.get_document("update").cloned().unwrap_or_default();
                        WriteModel::UpdateOne(
                            UpdateOneModel::builder()
                                .namespace(collection.namespace())
                                .filter(filter)
                                .update(update)
                                .upsert(operation == "upsert")
                                .build(),
                        )
                    }
                    _ => {
                        eprintln!("Row {}: Unsupported operation '{}'.", row_num, operation);
                        continue;
                    }
                };

                bulk_buffer.push(model);

                if bulk_buffer.len() >= batch_size {
                    let client = collection.client();
                    let ops: Vec<WriteModel> = bulk_buffer.drain(..).collect();
                    if let Err(e) = client.bulk_write(ops).await {
                        eprintln!("Bulk write error at row {}: {}", row_num, e);
                    }
                }
            } else {
                let result = match operation {
                    "insert" => collection.insert_one(rendered).await.map(|_| ()),
                    "upsert" | "update" => {
                        let filter = rendered.get_document("filter").cloned().unwrap_or_default();
                        let update = rendered.get_document("update").cloned().unwrap_or_default();
                        collection
                            .update_one(filter, update)
                            .upsert(operation == "upsert")
                            .await
                            .map(|_| ())
                    }
                    _ => return Err(anyhow!("Unsupported operation: {}", operation)),
                };

                if let Err(e) = result {
                    eprintln!("Row {}: MongoDB write error: {}", row_num, e);
                }
            }
        }
    }

    if !bulk_buffer.is_empty() {
        let client = collection.client();
        let ops: Vec<WriteModel> = bulk_buffer;
        if let Err(e) = client.bulk_write(ops).await {
            eprintln!("Final bulk write error: {}", e);
        }
    }

    println!("✅ Completed import process.");
    Ok(())
}
