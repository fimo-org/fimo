// src/mongodb.rs
use mongodb::{Client, Collection};
use mongodb::options::ClientOptions;
use bson::Document;
use anyhow::Result;

pub async fn connect(uri: &str, db: &str, collection: &str) -> Result<Collection<Document>> {
    let client_options = ClientOptions::parse(uri).await?;
    let client = Client::with_options(client_options)?;
    let db = client.database(db);
    Ok(db.collection::<Document>(collection))
}