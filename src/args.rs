use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub(crate) struct Args {
    storage_path: PathBuf,
    mount_path: PathBuf,
    #[clap(long)]
    auto_version: bool,
    #[clap(long)]
    max_version: u16,
    #[clap(long)]
    exclude: String,
    #[clap(long)]
    min_interval: String,
    #[clap(long)]
    storage_limit: String,
}