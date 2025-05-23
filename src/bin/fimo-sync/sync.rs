// --- sync.rs ---
use crate::cli::Cli;
use anyhow::{anyhow, Result};

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

use futures::{
    future::join_all,
    stream::{FuturesUnordered, StreamExt},
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

#[derive(Serialize, Deserialize)]
struct ResumeCheckpoint {
    value: Bson,
    _id: Bson,
}

struct SyncContext {
    health_file: Option<String>,
    source_collection: Collection<Document>,
    target_collection: Collection<Document>,
    target_client: Client,
    is_target_mongo_8_or_higher: bool,
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

    let is_target_mongo_8_or_higher = is_mongo_8_or_higher(&target_client).await?;

    let health_file = &args.health_file;

    Ok(SyncContext {
        health_file: health_file.clone(),
        source_collection,
        target_collection,
        target_client,
        is_target_mongo_8_or_higher,
    })
}

pub async fn start_sync(args: Cli) -> Result<()> {
    if args.use_change_stream {
        println!("Starting sync using change streams");

        let ctx = prepare_sync_context(&args).await?;
        println!("Connected to source and target collections");

        let resume_token: Option<ResumeToken> = if let Some(path) = &args.resume_file {
            if Path::new(path).exists() {
                println!("Loading resume token from {}", path);
                let data = fs::read_to_string(path)?;
                let token_val: ResumeToken = serde_json::from_str(&data)?;
                Some(token_val)
            } else {
                println!("Resume file not found at {}", path);
                None
            }
        } else {
            println!("No resume file specified");
            None
        };

        let mut stream = if let Some(token) = resume_token {
            println!("Resuming from token: {:?}", token);
            ctx.source_collection
                .watch()
                .resume_after(token)
                .full_document(FullDocumentType::UpdateLookup)
                .await?
        } else {
            println!("Starting new change stream");
            ctx.source_collection
                .watch()
                .full_document(FullDocumentType::UpdateLookup)
                .await?
        };

        let mut batch: Vec<Document> = Vec::new();
        let batch_size = args.limit.unwrap_or(100);

        println!("Waiting for changes...");

        while let Some(event) = stream.next().await {
            match event {
                Ok(change) => {
                    if let Some(doc) = process_change_event(&change) {
                        batch.push(doc);

                        if batch.len() >= batch_size {
                            write_to_target(
                                &ctx.target_client,
                                &ctx.target_collection,
                                &batch,
                                args.concurrency.unwrap_or(10),
                                ctx.is_target_mongo_8_or_higher,
                            )
                            .await?;
                            batch.clear();

                            if let Some(ref path) = &ctx.health_file {
                                fs::write(
                                    path,
                                    format!("{}", chrono::Utc::now().timestamp_millis()),
                                )?;
                            }

                            if args.store_resume {
                                if let Some(path) = &args.resume_file {
                                    let serialized = serde_json::to_string(&change.id)?;
                                    fs::write(path, serialized)?;
                                }
                            }
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
            write_to_target(
                &ctx.target_client,
                &ctx.target_collection,
                &batch,
                args.concurrency.unwrap_or(10),
                ctx.is_target_mongo_8_or_higher,
            )
            .await?;
            if let Some(ref path) = &ctx.health_file {
                fs::write(path, format!("{}", chrono::Utc::now().timestamp_millis()))?;
            }
        }

        Ok(())
    } else if let Some(field) = &args.sync_field {
        println!("Starting sync using field '{}'", field);

        let ctx = prepare_sync_context(&args).await?;

        let mut resume_value: Option<Bson> = None;
        let mut last_id: Option<Bson> = None;

        if let Some(path) = &args.resume_file {
            if Path::new(path).exists() {
                let data = fs::read_to_string(path)?;
                let checkpoint: ResumeCheckpoint = serde_json::from_str(&data)?;
                resume_value = Some(checkpoint.value);
                last_id = Some(checkpoint._id);
            }
        }

        let mut delay = 10_000;
        let max_delay = 60_000;

        loop {
            let filter = if let Some(value) = &resume_value {
                if field == "_id" {
                    doc! { field: { "$gt": value.clone() } }
                } else {
                    doc! {
                        "$or": [
                            { field: { "$gt": value.clone() } },
                            { "$and": [ { field: value.clone() }, { "_id": { "$gt": last_id.clone().unwrap() } } ] }
                        ]
                    }
                }
            } else {
                doc! {}
            };

            let sort = if field == "_id" {
                doc! { field: 1 }
            } else {
                doc! { field: 1, "_id": 1 }
            };

            let mut cursor = ctx
                .source_collection
                .find(filter)
                .sort(sort)
                .limit(args.limit.unwrap_or(100) as i64)
                .await?;

            let mut batch: Vec<Document> = Vec::new();
            let mut last_doc: Option<Document> = None;

            while let Some(doc) = cursor.next().await {
                match doc {
                    Ok(document) => {
                        batch.push(document.clone());
                        last_doc = Some(document);
                    }
                    Err(e) => {
                        eprintln!("Document error: {}", e);
                    }
                }
            }

            if !batch.is_empty() {
                write_to_target(
                    &ctx.target_client,
                    &ctx.target_collection,
                    &batch,
                    args.concurrency.unwrap_or(10),
                    ctx.is_target_mongo_8_or_higher,
                )
                .await?;
                if let Some(ref path) = &ctx.health_file {
                    fs::write(path, format!("{}", chrono::Utc::now().timestamp_millis()))?;
                }

                if let Some(doc) = &last_doc {
                    if field == "_id" {
                        if let Some(id) = doc.get_object_id("_id").ok() {
                            resume_value = Some(Bson::ObjectId(id));
                            let serialized = serde_json::to_string(&ResumeCheckpoint {
                                value: Bson::ObjectId(id.clone()),
                                _id: Bson::ObjectId(id),
                            })?;
                            if let Some(path) = &args.resume_file {
                                fs::write(path, serialized)?;
                            }
                        }
                    } else {
                        if let (Some(value), Some(id)) =
                            (doc.get(field), doc.get_object_id("_id").ok())
                        {
                            resume_value = Some(value.clone());
                            last_id = Some(Bson::ObjectId(id.clone()));
                            let checkpoint = ResumeCheckpoint {
                                value: value.clone(),
                                _id: Bson::ObjectId(id.clone()),
                            };
                            if let Some(path) = &args.resume_file {
                                let serialized = serde_json::to_string(&checkpoint)?;
                                fs::write(path, serialized)?;
                            }
                        }
                    }
                }
                delay = 10_000;
            } else {
                println!("⏳ No new data. Sleeping for {} ms...", delay);
                sleep(Duration::from_millis(delay)).await;
                delay = std::cmp::min(delay * 2, max_delay);
            }
        }
    } else {
        Err(anyhow!(
            "Either --use-change-stream or --sync-field must be provided"
        ))
    }
}

fn process_change_event(change: &ChangeStreamEvent<Document>) -> Option<Document> {
    match change.operation_type {
        OperationType::Insert | OperationType::Replace | OperationType::Update => {
            change.full_document.clone()
        }
        _ => None,
    }
}

fn is_version_8_or_higher(version_str: &str) -> bool {
    let parts: Vec<u32> = version_str
        .split('.')
        .filter_map(|x| x.parse::<u32>().ok())
        .collect();
    matches!(parts.as_slice(), [major, ..] if *major >= 8)
}

async fn is_mongo_8_or_higher(client: &Client) -> Result<bool> {
    let admin_db = client.database("admin");
    let result = admin_db.run_command(doc! { "buildInfo": 1 }).await?;
    if let Some(Bson::String(version_str)) = result.get("version") {
        Ok(is_version_8_or_higher(version_str))
    } else {
        Err(anyhow!("Could not determine MongoDB version"))
    }
}

pub async fn write_to_target(
    client: &Client,
    collection: &Collection<Document>,
    docs: &[Document],
    concurrency_limit: usize,
    is_mongo_8_or_higher: bool,
) -> Result<()> {
    if is_mongo_8_or_higher {
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
        client.bulk_write(models).await?;
    } else {
        let semaphore = Arc::new(Semaphore::new(concurrency_limit));
        let mut tasks = FuturesUnordered::new();

        for doc in docs.iter().cloned() {
            if let Some(id) = doc.get("_id").cloned() {
                let collection = collection.clone();
                let permit = semaphore.clone().acquire_owned().await.unwrap();

                tasks.push(tokio::spawn(async move {
                    let _permit = permit;
                    let filter = doc! { "_id": id };
                    match collection.replace_one(filter, doc).upsert(true).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            eprintln!("Per-doc write error: {}", e);
                            Err(e)
                        }
                    }
                }));
            }
        }

        while let Some(res) = tasks.next().await {
            if let Err(e) = res {
                eprintln!("Task join error: {}", e);
            }
        }
    }

    Ok(())
}
