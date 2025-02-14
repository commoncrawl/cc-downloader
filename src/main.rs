use clap::Parser;

use crate::cli::Commands;

mod cli;
mod download;
mod errors;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();

    match &cli.command {
        Some(Commands::DownloadPaths {
            snapshot,
            data_type,
            dst,
        }) => {
            let options = download::DownloadOptions {
                snapshot: snapshot.to_string(),
                data_type: data_type.as_str(),
                dst,
                ..Default::default()
            };
            match download::download_paths(options).await {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Error downloading paths: {}", e);
                }
            };
        }
        Some(Commands::Download {
            path_file,
            dst,
            progress,
            threads,
            retries,
            numbered,
            files_only,
        }) => {
            if *numbered && *files_only {
                eprintln!("Numbered and Files Only flags are incompatible");
            } else {
                let options = download::DownloadOptions {
                    paths: path_file,
                    dst,
                    progress: *progress,
                    threads: *threads,
                    max_retries: *retries,
                    numbered: *numbered,
                    files_only: *files_only,
                    ..Default::default()
                };
                match download::download(options).await {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Error downloading paths: {}", e);
                    }
                };
            }
        }
        None => {
            eprintln!("No command specified");
        }
    }
}
