// src/cli.rs
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long)]
    pub input: String,

    #[arg(long)]
    pub mapping: String,

    #[arg(long)]
    pub mongo_uri: String,

    #[arg(long)]
    pub db: String,

    #[arg(long)]
    pub collection: String,

    #[arg(long)]
    pub operation: Option<String>,

    #[arg(long)]
    pub template_dir: Option<String>,

    #[arg(long)]
    pub no_header: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub debug: bool,

    #[arg(long)]
    pub validate_only: bool,

    #[arg(long)]
    pub raw_insert: bool,

    #[arg(long)]
    pub batch_size: Option<usize>,

    #[arg(long)]
    pub extended_json: bool,
}
