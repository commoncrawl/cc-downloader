use flate2::read::GzDecoder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{header, Client, Url};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::Jitter;
use reqwest_retry::RetryTransientMiddleware;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::errors::DownloadError;

const BASE_URL: &str = "https://data.commoncrawl.org/";

pub async fn download_paths(
    snapshot: &String,
    data_type: &str,
    dst: &PathBuf,
) -> Result<(), DownloadError> {
    let paths = format!("{}crawl-data/{}/{}.paths.gz", BASE_URL, snapshot, data_type);
    println!("Downloading paths from: {}", paths);
    let url = Url::parse(&paths)?;

    let client = Client::new();

    let filename = url
        .path_segments() // Splits into segments of the URL
        .and_then(|segments| segments.last()) // Retrieves the last segment
        .unwrap_or("file.download"); // Fallback to generic filename

    let request = client.get(url.as_str());

    let mut dst = dst.clone();

    dst.push(filename);

    let outfile = tokio::fs::File::create(dst.clone()).await?;
    let mut outfile = BufWriter::new(outfile);

    let mut download = request.send().await?;

    while let Some(chunk) = download.chunk().await? {
        outfile.write(&chunk).await?; // Write chunk to output file
    }

    outfile.flush().await?;

    println!("Downloaded paths to: {}", dst.to_str().unwrap());

    Ok(())
}

// Based on: https://github.com/benkay86/async-applied/blob/master/indicatif-reqwest-tokio/src/bin/indicatif-reqwest-tokio-multi.rs

async fn download_task(
    client: ClientWithMiddleware,
    download_url: String,
    number: usize,
    multibar: Arc<MultiProgress>,
    dst: PathBuf,
    numbered: bool,
    files_only: bool,
    progress: bool,
) -> Result<(), DownloadError> {
    // Parse URL into Url type
    let url = Url::parse(&download_url)?;

    // We need to determine the file size before we download, so we can create a ProgressBar
    // A Header request for the CONTENT_LENGTH header gets us the file size
    let download_size = {
        let resp = client.head(url.as_str()).send().await?;
        if resp.status().is_success() {
            resp.headers() // Gives us the HeaderMap
                .get(header::CONTENT_LENGTH) // Gives us an Option containing the HeaderValue
                .and_then(|ct_len| ct_len.to_str().ok()) // Unwraps the Option as &str
                .and_then(|ct_len| ct_len.parse().ok()) // Parses the Option as u64
                .unwrap_or(0) // Fallback to 0
        } else {
            // We return an Error if something goes wrong here
            return Err(
                format!("Couldn't download URL: {}. Error: {:?}", url, resp.status(),).into(),
            );
        }
    };

    // Parse the filename from the given URL
    let filename = if numbered {
        &format!("{}{}", number.to_string(), ".txt.gz")
    } else if files_only {
        url.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("file.download")
    } else {
        url.path().strip_prefix("/").unwrap_or("file.download")
    };

    let mut dst = dst.clone();

    dst.push(filename);

    // Here we build the actual Request with a RequestBuilder from the Client
    let request = client.get(url.as_str());

    // Create the ProgressBar with the aquired size from before
    // and add it to the multibar
    let progress_bar = multibar.add(ProgressBar::new(download_size));

    if progress {
        // Set Style to the ProgressBar
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("[{bar:40.cyan/blue}] {bytes}/{total_bytes} - {msg}")?
                .progress_chars("#>-"),
        );

        // Set the filename as message part of the progress bar
        progress_bar.set_message(filename.to_owned());
    } else {
        println!("Downloading: {}", url.as_str());
    }

    // Create the directory if it doesn't exist
    if !numbered {
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    // Create the output file with tokio's async fs lib
    let outfile = tokio::fs::File::create(dst.clone()).await?;
    let mut outfile = BufWriter::new(outfile);

    // Do the actual request to download the file
    let mut download = request.send().await?;

    // Do an asynchronous, buffered copy of the download to the output file.
    //
    // We use the part from the reqwest-tokio example here on purpose
    // This way, we are able to increase the ProgressBar with every downloaded chunk
    while let Some(chunk) = download.chunk().await? {
        if progress {
            progress_bar.inc(chunk.len() as u64); // Increase ProgressBar by chunk size
        }
        outfile.write(&chunk).await?; // Write chunk to output file
    }

    if progress {
        // Finish the progress bar to prevent glitches
        progress_bar.finish();

        // Remove the progress bar from the multibar
        multibar.remove(&progress_bar);
    } else {
        multibar.remove(&progress_bar);
        println!("Downloaded file to: {}", dst.to_str().unwrap());
    }

    // Must flush tokio::io::BufWriter manually.
    // It will *not* flush itself automatically when dropped.
    outfile.flush().await?;

    Ok(())
}

pub async fn download(
    paths: &PathBuf,
    dst: &PathBuf,
    threads: usize,
    max_retries: usize,
    numbered: bool,
    files_only: bool,
    progress: bool,
) -> Result<(), DownloadError> {
    // A vector containing all the URLs to download

    let file = {
        let gzip_file = File::open(paths)?;
        let file_decoded = GzDecoder::new(gzip_file);
        BufReader::new(file_decoded)
    };

    let paths: Vec<(usize, String)> = file
        .lines()
        .map(|line| {
            let line = line.unwrap();
            format!("{}{}", BASE_URL, line)
        })
        .enumerate()
        .collect();

    // Set up a new multi-progress bar.
    // The bar is stored in an `Arc` to facilitate sharing between threads.
    let multibar = std::sync::Arc::new(indicatif::MultiProgress::new());

    // Add an overall progress indicator to the multibar.
    // It has as many steps as the download_links Vector and will increment on completion of each task.
    let main_pb = std::sync::Arc::new(
        multibar
            .clone()
            .add(indicatif::ProgressBar::new(paths.len() as u64)),
    );

    // Only set the style if we are showing progress
    if progress {
        main_pb.set_style(
            indicatif::ProgressStyle::default_bar().template("{msg} {bar:10} {pos}/{len}")?,
        );
        main_pb.set_message("total  ");

        // Make the main progress bar render immediately rather than waiting for the
        // first task to finish.
        main_pb.tick();
    }

    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_secs(1), Duration::from_secs(3600))
        .jitter(Jitter::Bounded)
        .base(2)
        .build_with_max_retries(u32::try_from(max_retries).unwrap());

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    let semaphore = Arc::new(Semaphore::new(threads));
    let mut set = JoinSet::new();

    for (number, path) in paths {
        // Clone multibar and main_pb.  We will move the clones into each task.
        let multibar = multibar.clone();
        let main_pb = main_pb.clone();
        let client = client.clone();
        let dst = dst.clone();
        let numbered = numbered.clone();
        let files_only = files_only.clone();
        let semaphore = semaphore.clone();
        let progress = progress.clone();
        set.spawn(async move {
            let _permit = semaphore.acquire().await;
            let res = download_task(
                client, path, number, multibar, dst, numbered, files_only, progress,
            )
            .await;
            if progress {
                // Increment the main progress bar.
                main_pb.inc(1);
            }
            res
        });
    }

    // Set up a future to manage rendering of the multiple progress bars.
    let multibar = {
        // Create a clone of the multibar, which we will move into the task.
        let multibar = multibar.clone();

        // multibar.join() is *not* async and will block until all the progress
        // bars are done, therefore we must spawn it on a separate scheduler
        // on which blocking behavior is allowed.
        tokio::task::spawn_blocking(move || multibar)
    };

    // Wait for the tasks to finish.
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("Error: {:?}", e),
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }

    if progress {
        // Change the message on the overall progress indicator.
        main_pb.finish_with_message("done");

        // Wait for the progress bars to finish rendering.
        // The first ? unwraps the outer join() in which we are waiting for the
        // future spawned by tokio::task::spawn_blocking to finish.
        // The second ? unwraps the inner multibar.join().
        multibar.await?;
    } else {
        println!("All downloads completed");
    }
    Ok(())
}
