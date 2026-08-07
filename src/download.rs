use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use ratelimit::{Ratelimiter, TryWaitError};
use regex::Regex;
use reqwest::{Client, Url, header};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{Jitter, RetryTransientMiddleware, policies::ExponentialBackoff};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
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

async fn rate_limit_acquire(limiter: &Ratelimiter) {
    loop {
        match limiter.try_wait() {
            Ok(()) => return,
            Err(TryWaitError::Insufficient(wait)) => tokio::time::sleep(wait).await,
            Err(TryWaitError::ExceedsCapacity) => unreachable!("max_tokens > 0"),
            Err(_) => unreachable!(),
        }
    }
}

/// Configuration for a [`download_paths`] or [`download`] call.
///
/// Construct with [`DownloadOptions::new`] (for the common CLI use-case) or
/// with a struct literal and [`Default`] for the fields you don't need:
///
/// ```no_run
/// use cc_downloader::download::DownloadOptions;
///
/// let options = DownloadOptions {
///     snapshot: "CC-MAIN-2024-46".to_string(),
///     data_type: "wet",
///     dst: std::path::Path::new("./output"),
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug)]
pub struct DownloadOptions<'a> {
    /// Crawl snapshot identifier, e.g. `"CC-MAIN-2024-46"` or `"CC-NEWS-2025-01"`.
    pub snapshot: String,
    /// Data type to download, e.g. `"warc"`, `"wet"`, `"cc-index-table"`.
    pub data_type: &'a str,
    /// Path to a `.paths.gz` file that lists the files to download.
    /// Only used by [`download`]; ignored by [`download_paths`].
    pub paths: &'a Path,
    /// Destination directory where downloaded files will be written.
    pub dst: &'a Path,
    /// Maximum number of concurrent downloads. Defaults to `1` (via [`Default`]),
    /// `10` via [`DownloadOptions::new`].
    pub threads: usize,
    /// Maximum number of per-file retry attempts before giving up. Defaults to `1000`.
    pub max_retries: usize,
    /// If `true`, output files are named sequentially (`0.txt.gz`, `1.txt.gz`, …).
    /// Only meaningful for WET files. Mutually exclusive with [`files_only`](Self::files_only).
    pub numbered: bool,
    /// If `true`, files are written flat into `dst` with no subdirectory structure.
    /// Only meaningful for WARC/WET/WAT files. Mutually exclusive with [`numbered`](Self::numbered).
    pub files_only: bool,
    /// If `true`, renders a per-file progress bar and an overall completion indicator.
    pub progress: bool,
    /// Subset filter for `cc-index-table` downloads.
    /// Valid values: `"crawldiagnostics"`, `"robotstxt"`, `"warc"`.
    /// An empty `Vec` means no filtering — all subsets are retained.
    pub cc_index_table_subsets: Vec<String>,
}

struct TaskOptions {
    pub number: usize,
    pub path: String,
    pub dst: PathBuf,
    pub numbered: bool,
    pub files_only: bool,
    pub progress: bool,
    pub rate_limiter: Arc<Ratelimiter>,
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
            cc_index_table_subsets: Vec::new(),
        }
    }
}

impl<'a> DownloadOptions<'a> {
    /// Creates a `DownloadOptions` with sensible defaults for downloading data files.
    ///
    /// Validates `snapshot` against the expected crawl name format
    /// (`CC-MAIN-YYYY-WW` or `CC-NEWS-YYYY-MM`) and exits the process with an
    /// error message if the format is invalid.
    ///
    /// Defaults: `threads = 10`, `max_retries = 1000`, `cc_index_table_subsets = []`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_downloader::download::DownloadOptions;
    ///
    /// let mut options = DownloadOptions::new(
    ///     "CC-MAIN-2024-46",
    ///     "wet",
    ///     "./output/wet.paths.gz",
    ///     "./output",
    ///     false, // numbered
    ///     false, // files_only
    ///     true,  // progress
    /// );
    /// options.set_threads(5);
    /// ```
    pub fn new(
        snapshot: &'a str,
        data_type: &'a str,
        paths: &'a str,
        dst: &'a str,
        numbered: bool,
        files_only: bool,
        progress: bool,
    ) -> Self {
        // Validate the snapshot format
        let snapshot = crawl_name_format(snapshot).unwrap_or_else(|err| {
            eprintln!("Error: {err}");
            process::exit(1);
        });
        DownloadOptions {
            snapshot,
            data_type,
            paths: Path::new(paths),
            dst: Path::new(dst),
            threads: 10,
            max_retries: 1000,
            numbered,
            files_only,
            progress,
            cc_index_table_subsets: Vec::new(),
        }
    }
    /// Sets the number of concurrent download threads.
    pub fn set_threads(&mut self, threads: usize) {
        self.threads = threads;
    }
    /// Sets the maximum number of per-file retry attempts.
    pub fn set_max_retries(&mut self, max_retries: usize) {
        self.max_retries = max_retries;
    }
}

fn filter_cc_index_table_paths(path: &Path, subsets: &[String]) -> Result<(), DownloadError> {
    // Read and decompress the downloaded paths file
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    // Keep only lines that match one of the requested subsets.
    // Each line looks like:
    //   cc-index/table/cc-main/warc/crawl=CC-MAIN-2026-08/subset=crawldiagnostics/part-...
    // so a simple `contains("subset=<name>")` is sufficient.
    let filtered: Vec<String> = reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let keep = subsets
                .iter()
                .any(|s| line.contains(&format!("subset={s}")));
            if keep { Some(line) } else { None }
        })
        .collect();

    println!(
        "Filtered to {} paths ({} subsets requested)",
        filtered.len(),
        subsets.len()
    );

    // Overwrite the same file with the filtered + re-compressed content
    let out = File::create(path)?;
    let mut encoder = GzEncoder::new(out, Compression::default());
    for line in &filtered {
        encoder.write_all(line.as_bytes())?;
        encoder.write_all(b"\n")?;
    }
    encoder.finish()?;

    Ok(())
}

fn crawl_name_format(crawl: &str) -> Result<String, String> {
    let main_re = Regex::new(r"^(CC\-MAIN)\-([0-9]{4})\-([0-9]{2})$").unwrap();
    let news_re = Regex::new(r"^(CC\-NEWS)\-([0-9]{4})\-([0-9]{2})$").unwrap();

    let crawl_ref = crawl.to_uppercase();

    if !(main_re.is_match(&crawl_ref) || news_re.is_match(&crawl_ref)) {
        Err("Please use the CC-MAIN-YYYY-WW or the CC-NEWS-YYYY-MM format.".to_string())
    } else {
        Ok(crawl_ref)
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

/// Downloads a paths file directly from a contributor dataset URL.
///
/// Unlike [`download_paths`], this function accepts an arbitrary URL rather
/// than constructing one from a snapshot and data type. The URL should point
/// to a gzip-compressed paths file hosted under
/// `https://data.commoncrawl.org/contrib/`.
///
/// # Errors
///
/// Returns [`DownloadError`] if:
/// - The URL cannot be parsed
/// - The server returns a non-success HTTP status
/// - Writing the destination file fails
///
/// # Examples
///
/// ```no_run
/// use cc_downloader::download::download_contrib_paths;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), cc_downloader::errors::DownloadError> {
/// download_contrib_paths(
///     "https://data.commoncrawl.org/contrib/my-dataset/paths.gz",
///     std::path::Path::new("./output"),
///     1000,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn download_contrib_paths(
    url: &str,
    dst: &Path,
    max_retries: usize,
) -> Result<(), DownloadError> {
    let url = Url::parse(url)?;
    let client = new_client(max_retries)?;

    let filename = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or("file.download");

    println!("Downloading contrib paths from: {url}");

    let resp = client.head(url.as_str()).send().await?;
    match resp.status() {
        status if status.is_success() => (),
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

    let mut dst = dst.to_path_buf();
    dst.push(filename);

    if let Some(parent) = dst.parent()
        && !parent.exists()
    {
        println!("Creating directory: {}", parent.to_str().unwrap());
        tokio::fs::create_dir_all(parent).await?;
    }

    let outfile = tokio::fs::File::create(dst.clone()).await?;
    let mut outfile = BufWriter::new(outfile);

    let mut download = client.get(url.as_str()).send().await?;
    while let Some(chunk) = download.chunk().await? {
        outfile.write_all(&chunk).await?;
    }
    outfile.flush().await?;

    println!("Downloaded contrib paths to: {}", dst.to_str().unwrap());
    Ok(())
}

/// Downloads the paths index file for a Common Crawl snapshot and data type.
///
/// Constructs the URL from `options.snapshot` and `options.data_type`, performs
/// a HEAD request to verify the resource exists (returning a descriptive error
/// for 404s), then streams the gzip-compressed file to `options.dst`.
///
/// For `cc-index-table` downloads, if `options.cc_index_table_subsets` is
/// non-empty the saved file is filtered in place so it only contains paths
/// for the requested subsets (`"crawldiagnostics"`, `"robotstxt"`, `"warc"`).
///
/// # Errors
///
/// Returns [`DownloadError`] if:
/// - The constructed URL cannot be parsed
/// - The server returns a non-success HTTP status (404 produces a descriptive message)
/// - Writing the destination file fails
/// - Subset filtering fails (I/O error re-compressing the file)
///
/// # Examples
///
/// Download all paths for a WET crawl:
///
/// ```no_run
/// use cc_downloader::download::{DownloadOptions, download_paths};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), cc_downloader::errors::DownloadError> {
/// let options = DownloadOptions {
///     snapshot: "CC-MAIN-2024-46".to_string(),
///     data_type: "wet",
///     dst: std::path::Path::new("./output"),
///     ..Default::default()
/// };
/// download_paths(options).await?;
/// # Ok(())
/// # }
/// ```
///
/// Download only the `warc` and `robotstxt` subsets of a `cc-index-table` crawl:
///
/// ```no_run
/// use cc_downloader::download::{DownloadOptions, download_paths};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), cc_downloader::errors::DownloadError> {
/// let options = DownloadOptions {
///     snapshot: "CC-MAIN-2024-46".to_string(),
///     data_type: "cc-index-table",
///     dst: std::path::Path::new("./output"),
///     cc_index_table_subsets: vec!["warc".to_string(), "robotstxt".to_string()],
///     ..Default::default()
/// };
/// download_paths(options).await?;
/// # Ok(())
/// # }
/// ```
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
    println!("Downloading paths from: {paths}");
    let url = Url::parse(&paths)?;

    let client = new_client(options.max_retries)?;

    let filename = url
        .path_segments() // Splits into segments of the URL
        .and_then(|mut segments| segments.next_back()) // Retrieves the last segment
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

    if let Some(parent) = dst.parent()
        && !parent.exists()
    {
        println!("Creating directory: {}", parent.to_str().unwrap());
        tokio::fs::create_dir_all(parent).await?;
    }

    let outfile = tokio::fs::File::create(dst.clone()).await?;
    let mut outfile = BufWriter::new(outfile);

    let mut download = request.send().await?;

    while let Some(chunk) = download.chunk().await? {
        outfile.write_all(&chunk).await?; // Write chunk to output file
    }

    outfile.flush().await?;

    // Filter the downloaded paths file if subset filtering was requested
    if options.data_type == "cc-index-table" && !options.cc_index_table_subsets.is_empty() {
        filter_cc_index_table_paths(&dst, &options.cc_index_table_subsets)?;
    }

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

    // Acquire a permit from the rate limiter before making the request.
    rate_limit_acquire(&task_options.rate_limiter).await;
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
            .and_then(|mut segments| segments.next_back())
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
    if !task_options.numbered
        && let Some(parent) = dst.parent()
    {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Create the output file with tokio's async fs lib
    let outfile = tokio::fs::File::create(dst.clone()).await?;
    let mut outfile = BufWriter::new(outfile);

    // Acquire a permit from the rate limiter before making the request.
    rate_limit_acquire(&task_options.rate_limiter).await;
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

/// Downloads every file listed in a previously obtained `.paths.gz` index.
///
/// Reads the gzip-compressed paths file at `options.paths`, prepends the
/// Common Crawl base URL to each entry, then downloads all files concurrently
/// up to `options.threads` at a time. Each file is retried up to
/// `options.max_retries` times with exponential backoff and jitter.
///
/// # Errors
///
/// Returns [`DownloadError`] if the paths file
/// cannot be opened, or if the progress bar template is invalid. Individual
/// file errors are printed to stderr but do not abort the remaining downloads.
///
/// # Examples
///
/// ```no_run
/// use cc_downloader::download::{DownloadOptions, download};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), cc_downloader::errors::DownloadError> {
/// let options = DownloadOptions {
///     paths: std::path::Path::new("./output/wet.paths.gz"),
///     dst: std::path::Path::new("./output"),
///     threads: 10,
///     progress: true,
///     ..Default::default()
/// };
/// download(options).await?;
/// # Ok(())
/// # }
/// ```
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
            format!("{BASE_URL}{line}")
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

    let rate_limiter = Arc::new(
        Ratelimiter::builder(1499)
            .period(Duration::from_secs(300))
            .build()
            .expect("invalid rate limit config"),
    );

    let semaphore = Arc::new(Semaphore::new(options.threads));
    let mut set = JoinSet::new();

    for (number, path) in paths {
        // Clone multibar and main_pb.  We will move the clones into each task.
        let multibar = multibar.clone();
        let main_pb = main_pb.clone();
        let client = client.clone();
        let dst = options.dst.to_path_buf();
        let semaphore = semaphore.clone();
        let rate_limiter = rate_limiter.clone();
        set.spawn(async move {
            let _permit = semaphore.acquire().await;
            let task_options = TaskOptions {
                path,
                number,
                dst,
                numbered: options.numbered,
                files_only: options.files_only,
                progress: options.progress,
                rate_limiter,
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
    let mut had_error = false;
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                eprintln!("Error: {e:?}");
                had_error = true;
            }
            Err(e) => {
                eprintln!("Error: {e:?}");
                had_error = true;
            }
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
    if had_error {
        return Err(String::from("one or more downloads failed").into());
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
