use clap::Parser;

/// CLI arguments for fimo-sync
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Source MongoDB URI
    #[arg(long)]
    pub source_uri: String,

    /// Source database name
    #[arg(long)]
    pub source_db: String,

    /// Source collection name
    #[arg(long)]
    pub source_collection: String,

    /// Target MongoDB URI
    #[arg(long)]
    pub target_uri: String,

    /// Target database name
    #[arg(long)]
    pub target_db: String,

    /// Target collection name
    #[arg(long)]
    pub target_collection: String,

    /// Use change stream for sync
    #[arg(long, default_value_t = false)]
    pub use_change_stream: bool,

    /// Field to use for field-based sync (e.g., date, ObjectId, number, string)
    #[arg(long)]
    pub sync_field: Option<String>,

    /// Resume token or last synced field value (overrides resume file)
    #[arg(long)]
    pub resume_value: Option<String>,

    ///--resume-type <string|int|objectid|date>
    #[arg(long)]
    pub resume_type: Option<String>,

    /// Path to the resume file (token or field value)
    #[arg(long)]
    pub resume_file: Option<String>,

    /// Overwrite resume file with latest token or field value
    #[arg(long, default_value_t = false)]
    pub store_resume: bool,

    /// Limit number of documents per sync batch
    #[arg(long)]
    pub limit: Option<usize>,
}