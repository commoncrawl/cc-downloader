use clap::Parser;
use std::process::ExitCode;

use crate::cli::{Commands, DataType, DownloadPathsSource};
use cc_downloader::download;
mod cli;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = cli::Cli::parse();

    match cli.command {
        Commands::DownloadPaths { source } => match source {
            DownloadPathsSource::Crawl {
                snapshot,
                data_type,
                dst,
                subsets,
            } => {
                if !subsets.is_empty() && data_type != DataType::CcIndexTable {
                    eprintln!("Error: --subset is only valid with the `cc-index-table` data type");
                    return ExitCode::FAILURE;
                }
                let options = download::DownloadOptions {
                    snapshot: snapshot.to_string(),
                    data_type: data_type.as_str(),
                    dst: &dst,
                    cc_index_table_subsets: subsets
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect(),
                    ..Default::default()
                };
                if let Err(e) = download::download_paths(options).await {
                    eprintln!("Error downloading paths: {e}");
                    return ExitCode::FAILURE;
                }
            }
            DownloadPathsSource::Contrib { url, dst } => {
                if let Err(e) = download::download_contrib_paths(&url, &dst, 1000).await {
                    eprintln!("Error downloading contrib paths: {e}");
                    return ExitCode::FAILURE;
                }
            }
        },
        Commands::Download {
            path_file,
            dst,
            progress,
            threads,
            retries,
            numbered,
            files_only,
        } => {
            let options = download::DownloadOptions {
                paths: &path_file,
                dst: &dst,
                progress,
                threads,
                max_retries: retries,
                numbered,
                files_only,
                ..Default::default()
            };
            if let Err(e) = download::download(options).await {
                eprintln!("Error downloading files: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
