//! A polite and user-friendly client for downloading [Common Crawl](https://commoncrawl.org) data.
//!
//! The library implements a two-step workflow that mirrors the CLI:
//!
//! 1. **Fetch the index** — download a `.paths.gz` file that lists every file
//!    in a crawl snapshot for a given data type.
//! 2. **Download the data** — read that index and download every file it lists,
//!    concurrently, with exponential-backoff retries.
//!
//! All configuration for both steps is expressed through [`download::DownloadOptions`].
//!
//! # Quick start
//!
//! ```no_run
//! use cc_downloader::download::{DownloadOptions, download_paths, download};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), cc_downloader::errors::DownloadError> {
//!     // Step 1: fetch the paths index for WET files from a crawl snapshot.
//!     let paths_options = DownloadOptions {
//!         snapshot: "CC-MAIN-2024-46".to_string(),
//!         data_type: "wet",
//!         dst: std::path::Path::new("./output"),
//!         ..Default::default()
//!     };
//!     download_paths(paths_options).await?;
//!
//!     // Step 2: download every file listed in the index.
//!     let download_options = DownloadOptions {
//!         paths: std::path::Path::new("./output/wet.paths.gz"),
//!         dst: std::path::Path::new("./output"),
//!         threads: 10,
//!         progress: true,
//!         ..Default::default()
//!     };
//!     download(download_options).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Contributor datasets
//!
//! For datasets hosted under `https://data.commoncrawl.org/contrib/` that don't
//! follow the standard snapshot/data-type schema, use
//! [`download::download_contrib_paths`] instead of [`download::download_paths`]:
//!
//! ```no_run
//! use cc_downloader::download::download_contrib_paths;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), cc_downloader::errors::DownloadError> {
//! download_contrib_paths(
//!     "https://data.commoncrawl.org/contrib/my-dataset/paths.gz",
//!     std::path::Path::new("./output"),
//!     1000,
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Retries and politeness
//!
//! Every HTTP request is wrapped in an exponential-backoff retry policy (bounds:
//! 1 s – 1 h, bounded jitter, base 2). The default of 1 000 retries is intentionally
//! high: Common Crawl is a shared public resource and transient failures are common
//! under load. Reduce [`download::DownloadOptions::max_retries`] only if you have a
//! specific reason to fail fast.
//!
//! # Error handling
//!
//! All public functions return [`errors::DownloadError`], which wraps the
//! underlying error from whichever subsystem failed. See that type for the full
//! list of variants and a usage example.

/// Core download functionality: path resolution, file downloading, and subset filtering.
pub mod download;

/// Error types returned by all fallible operations in this library.
pub mod errors;
