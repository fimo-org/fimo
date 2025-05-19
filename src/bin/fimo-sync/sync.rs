// --- sync.rs ---
use crate::cli::Cli;
use anyhow::{anyhow, Result};
use futures::StreamExt;

use mongodb::bson::Bson;
use mongodb::change_stream::event::OperationType;
use mongodb::change_stream::{
    event::{ChangeStreamEvent, ResumeToken},
    ChangeStream,
};
use mongodb::options::FullDocumentType;
use mongodb::options::ReplaceOneModel;
use mongodb::results::SummaryBulkWriteResult;
use mongodb::Collection;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};

use serde_json;
use std::fs;
use std::path::Path;

struct SyncContext {
    source_collection: Collection<Document>,
    target_collection: Collection<Document>,
    target_client: Client,
}

async fn prepare_sync_context(args: &Cli) -> Result<SyncContext> {
    let source_client_options = ClientOptions::parse(&args.source_uri).await?;
    let source_client = Client::with_options(source_client_options)?;
    let source_db = source_client.database(&args.source_db);
    let source_collection = source_db.collection::<Document>(&args.source_collection);

    let target_client_options = ClientOptions::parse(&args.target_uri).await?;
    let target_client = Client::with_options(target_client_options)?;
    let target_db = target_client.database(&args.target_db);
    let target_collection = target_db.collection::<Document>(&args.target_collection);

    Ok(SyncContext {
        source_collection,
        target_collection,
        target_client,
    })
}

pub async fn start_sync(args: Cli) -> Result<()> {
    if args.use_change_stream {
        println!("Starting sync using change streams");

        let ctx = prepare_sync_context(&args).await?;

        let resume_token: Option<ResumeToken> = if let Some(path) = &args.resume_file {
            if Path::new(path).exists() {
                let data = fs::read_to_string(path)?;
                let token_val: ResumeToken = serde_json::from_str(&data)?;
                Some(token_val)
            } else {
                None
            }
        } else {
            None
        };

        let mut stream: ChangeStream<ChangeStreamEvent<Document>> = ctx
            .source_collection
            .watch()
            .resume_after(resume_token)
            .full_document(FullDocumentType::UpdateLookup)
            .await?;

        let mut batch: Vec<Document> = Vec::new();
        let batch_size = args.limit.unwrap_or(100);

        while let Some(event) = stream.next().await {
            match event {
                Ok(change) => {
                    if let Some(doc) = process_change_event(&change) {
                        batch.push(doc);

                        if batch.len() >= batch_size {
                            if let Err(e) =
                                write_to_target(&ctx.target_client, &ctx.target_collection, &batch)
                                    .await
                            {
                                eprintln!("Target write error: {}", e);
                            }
                            batch.clear();
                        }
                    }

                    if args.store_resume {
                        let token = change.id;
                        if let Some(path) = &args.resume_file {
                            let serialized = serde_json::to_string(&token)?;
                            fs::write(path, serialized)?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Change stream error: {}", e);
                    break;
                }
            }
        }

        if !batch.is_empty() {
            write_to_target(&ctx.target_client, &ctx.target_collection, &batch).await?;
        }

        Ok(())
    } else if let Some(field) = &args.sync_field {
        println!("Starting sync using field '{}'", field);

        let ctx = prepare_sync_context(&args).await?;

        let resume_bson: Option<Bson> = if let Some(path) = &args.resume_file {
            if Path::new(path).exists() {
                let data = fs::read_to_string(path)?;
                let parsed: Bson = serde_json::from_str(&data)?;
                Some(parsed)
            } else {
                None
            }
        } else if let Some(val) = &args.resume_value {
            match args.resume_type.as_deref() {
                Some("int") => val.parse::<i64>().ok().map(Bson::Int64),
                Some("objectid") => mongodb::bson::oid::ObjectId::parse_str(val)
                    .ok()
                    .map(Bson::ObjectId),
                Some("date") => mongodb::bson::DateTime::parse_rfc3339_str(val)
                    .ok()
                    .map(Bson::DateTime),
                Some("string") | None => Some(Bson::String(val.clone())),
                Some(t) => return Err(anyhow!(format!("Unsupported resume_type '{}'", t))),
            }
        } else {
            None
        };

        let filter = if let Some(val) = resume_bson {
            doc! { field: { "$gt": val } }
        } else {
            doc! {}
        };

        let mut cursor = ctx
            .source_collection
            .find(filter)
            .sort(doc! { field: 1 })
            .limit(args.limit.unwrap_or(100) as i64)
            .await?;
        let mut batch: Vec<Document> = Vec::new();

        while let Some(doc) = cursor.next().await {
            match doc {
                Ok(document) => {
                    batch.push(document);
                }
                Err(e) => {
                    eprintln!("Document error: {}", e);
                }
            }
        }

        if !batch.is_empty() {
            write_to_target(&ctx.target_client, &ctx.target_collection, &batch).await?;

            if args.store_resume {
                if let Some(max_doc) = batch.last() {
                    if let Some(val) = max_doc.get(field) {
                        if let Some(path) = &args.resume_file {
                            let serialized = serde_json::to_string(val)?;
                            fs::write(path, serialized)?;
                        }
                    }
                }
            }
        }

        Ok(())
    } else {
        Err(anyhow!(
            "Either --use-change-stream or --sync-field must be provided"
        ))
    }
}

fn process_change_event(change: &ChangeStreamEvent<Document>) -> Option<Document> {
    match change.operation_type {
        OperationType::Insert | OperationType::Replace => change.full_document.clone(),
        OperationType::Update => {
            if let (Some(update_desc), Some(document_key)) = (
                change.update_description.as_ref(),
                change.document_key.as_ref(),
            ) {
                let mut doc = document_key.clone();
                for (k, v) in update_desc.updated_fields.iter() {
                    doc.insert(k, v.clone());
                }
                Some(doc)
            } else {
                None
            }
        }
        _ => None,
    }
}

async fn write_to_target(
    client: &Client,
    collection: &Collection<Document>,
    docs: &[Document],
) -> Result<SummaryBulkWriteResult> {
    let models: Vec<ReplaceOneModel> = docs
        .iter()
        .filter_map(|doc| {
            doc.get("_id").map(|id| {
                ReplaceOneModel::builder()
                    .namespace(collection.namespace())
                    .filter(doc! {"_id": id.clone()})
                    .replacement(doc.clone())
                    .upsert(true)
                    .build()
            })
        })
        .collect();

    let result = client.bulk_write(models).await?;
    Ok(result)
}
