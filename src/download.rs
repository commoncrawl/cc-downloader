use flate2::read::GzDecoder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::{Client, Url, header};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{Jitter, RetryTransientMiddleware, policies::ExponentialBackoff};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process, str,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    sync::Semaphore,
    task::JoinSet,
};

use crate::errors::DownloadError;

const BASE_URL: &str = "https://data.commoncrawl.org/";

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub struct DownloadOptions<'a> {
    pub snapshot: String,
    pub data_type: &'a str,
    pub paths: &'a Path,
    pub dst: &'a Path,
    pub threads: usize,
    pub max_retries: usize,
    pub numbered: bool,
    pub files_only: bool,
    pub progress: bool,
}

pub struct TaskOptions {
    pub number: usize,
    pub path: String,
    pub dst: PathBuf,
    pub numbered: bool,
    pub files_only: bool,
    pub progress: bool,
}

impl Default for DownloadOptions<'_> {
    fn default() -> Self {
        DownloadOptions {
            snapshot: "".to_string(),
            data_type: "",
            paths: Path::new(""),
            dst: Path::new(""),
            threads: 1,
            max_retries: 1000,
            numbered: false,
            files_only: false,
            progress: false,
        }
    }
}

fn new_client(max_retries: usize) -> Result<ClientWithMiddleware, DownloadError> {
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_secs(1), Duration::from_secs(3600))
        .jitter(Jitter::Bounded)
        .base(2)
        .build_with_max_retries(u32::try_from(max_retries).unwrap());

    let client_base = Client::builder().user_agent(APP_USER_AGENT).build()?;

    Ok(ClientBuilder::new(client_base)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build())
}

pub async fn download_paths(mut options: DownloadOptions<'_>) -> Result<(), DownloadError> {
    let news_re = Regex::new(r"^(CC\-NEWS)\-([0-9]{4})\-([0-9]{2})$").unwrap();

    // Check if the snapshot is a news snapshot and reformat it
    // The format of the main crawl urls is different from the news crawl urls
    // https://data.commoncrawl.org/crawl-data/CC-NEWS/2025/01/warc.paths.gz
    // https://data.commoncrawl.org/crawl-data/CC-MAIN-2025-08/warc.paths.gz
    let snapshot_original_ref = options.snapshot.clone();
    if news_re.is_match(&options.snapshot) {
        let caps = news_re.captures(&options.snapshot).unwrap();
        options.snapshot = format!("{}/{}/{}", &caps[1], &caps[2], &caps[3]);
    }
    let paths = format!(
        "{}crawl-data/{}/{}.paths.gz",
        BASE_URL, options.snapshot, options.data_type
    );
    println!("Downloading paths from: {}", paths);
    let url = Url::parse(&paths)?;

    let client = new_client(options.max_retries)?;

    let filename = url
        .path_segments() // Splits into segments of the URL
        .and_then(|segments| segments.last()) // Retrieves the last segment
        .unwrap_or("file.download"); // Fallback to generic filename

    let resp = client.head(url.as_str()).send().await?;
    match resp.status() {
        status if status.is_success() => (),
        status if status.as_u16() == 404 => {
            return Err(format!(
                "\n\nThe reference combination you requested:\n\tCRAWL: {}\n\tSUBSET: {}\n\tURL: {}\n\nDoesn't seem to exist or it is currently not accessible.\n\tError code: {} {}",
                snapshot_original_ref, options.data_type, url, status.as_str(), status.canonical_reason().unwrap_or("")
            )
            .into());
        }
        status => {
            return Err(format!(
                "Couldn't download URL: {}. Error code: {} {}",
                url,
                status.as_str(),
                status.canonical_reason().unwrap_or("")
            )
            .into());
        }
    }

    let request = client.get(url.as_str());

    let mut dst = options.dst.to_path_buf();

    dst.push(filename);

    let outfile = tokio::fs::File::create(dst.clone()).await?;
    let mut outfile = BufWriter::new(outfile);

    let mut download = request.send().await?;

    while let Some(chunk) = download.chunk().await? {
        outfile.write_all(&chunk).await?; // Write chunk to output file
    }

    outfile.flush().await?;

    println!("Downloaded paths to: {}", dst.to_str().unwrap());

    Ok(())
}

// Based on: https://github.com/benkay86/async-applied/blob/master/indicatif-reqwest-tokio/src/bin/indicatif-reqwest-tokio-multi.rs

async fn download_task(
    client: ClientWithMiddleware,
    multibar: Arc<MultiProgress>,
    task_options: TaskOptions,
) -> Result<(), DownloadError> {
    // Parse URL into Url type
    let url = Url::parse(&task_options.path)?;

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
                format!("Couldn't download URL: {}. Error: {:?}", url, resp.status()).into(),
            );
        }
    };

    // Parse the filename from the given URL
    let filename = if task_options.numbered {
        &format!("{}{}", task_options.number, ".txt.gz")
    } else if task_options.files_only {
        url.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("file.download")
    } else {
        url.path().strip_prefix("/").unwrap_or("file.download")
    };

    let mut dst = task_options.dst.clone();

    dst.push(filename);

    // Here we build the actual Request with a RequestBuilder from the Client
    let request = client.get(url.as_str());

    // Create the ProgressBar with the aquired size from before
    // and add it to the multibar
    let progress_bar = multibar.add(ProgressBar::new(download_size));

    if task_options.progress {
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
    if !task_options.numbered {
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
        if task_options.progress {
            progress_bar.inc(chunk.len() as u64); // Increase ProgressBar by chunk size
        }
        outfile.write_all(&chunk).await?; // Write chunk to output file
    }

    if task_options.progress {
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

pub async fn download(options: DownloadOptions<'_>) -> Result<(), DownloadError> {
    // A vector containing all the URLs to download

    let file = {
        let gzip_file = match File::open(options.paths) {
            Ok(file) => file,
            Err(e) => {
                eprintln!(
                    "Could not open file {}\nError: {}",
                    options.paths.display(),
                    e
                );
                process::exit(1)
            }
        };
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
    if options.progress {
        main_pb.set_style(
            indicatif::ProgressStyle::default_bar().template("{msg} {bar:10} {pos}/{len}")?,
        );
        main_pb.set_message("total  ");

        // Make the main progress bar render immediately rather than waiting for the
        // first task to finish.
        main_pb.tick();
    }

    let client = new_client(options.max_retries)?;

    let semaphore = Arc::new(Semaphore::new(options.threads));
    let mut set = JoinSet::new();

    for (number, path) in paths {
        // Clone multibar and main_pb.  We will move the clones into each task.
        let multibar = multibar.clone();
        let main_pb = main_pb.clone();
        let client = client.clone();
        let dst = options.dst.to_path_buf();
        let semaphore = semaphore.clone();
        set.spawn(async move {
            let _permit = semaphore.acquire().await;
            let task_options = TaskOptions {
                path,
                number,
                dst,
                numbered: options.numbered,
                files_only: options.files_only,
                progress: options.progress,
            };
            let res = download_task(client, multibar, task_options).await;
            if options.progress {
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

    if options.progress {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize, Debug)]
    pub struct HeadersEcho {
        pub headers: HashMap<String, String>,
    }

    #[test]
    fn user_agent_format() {
        assert_eq!(
            APP_USER_AGENT,
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),)
        );
    }

    #[tokio::test]
    async fn user_agent_test() -> Result<(), DownloadError> {
        let client = new_client(1000)?;
        let response = client.get("http://httpbin.org/headers").send().await?;

        let out: HeadersEcho = response.json().await?;
        assert_eq!(out.headers["User-Agent"], APP_USER_AGENT);
        Ok(())
    }
}
