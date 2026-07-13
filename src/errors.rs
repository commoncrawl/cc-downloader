use std::fmt;

/// Error type returned by all fallible operations in this library.
///
/// Each variant wraps the underlying error from the relevant subsystem, so
/// callers can pattern-match on specific failure modes or rely on the
/// [`Display`](std::fmt::Display) impl for a human-readable message.
///
/// # Examples
///
/// ```no_run
/// use cc_downloader::download::{DownloadOptions, download_paths};
/// use cc_downloader::errors::DownloadError;
///
/// # #[tokio::main]
/// # async fn main() {
/// let options = DownloadOptions {
///     snapshot: "CC-MAIN-2024-46".to_string(),
///     data_type: "wet",
///     dst: std::path::Path::new("./output"),
///     ..Default::default()
/// };
/// match download_paths(options).await {
///     Ok(_) => println!("Done"),
///     Err(DownloadError::Custom(msg)) => eprintln!("Resource not found: {msg}"),
///     Err(e) => eprintln!("Download failed: {e}"),
/// }
/// # }
/// ```
#[derive(Debug)]
pub enum DownloadError {
    /// An error from the underlying HTTP client ([`reqwest`]).
    Reqwest(reqwest::Error),
    /// An error from the retry middleware ([`reqwest_middleware`]).
    ReqwestMiddleware(reqwest_middleware::Error),
    /// An I/O error, either from [`tokio::io`] or the standard library.
    Tokio(tokio::io::Error),
    /// A URL parsing error.
    Url(url::ParseError),
    /// An error constructing a progress bar template ([`indicatif`]).
    Indicatif(indicatif::style::TemplateError),
    /// A task join error from the async runtime.
    Join(tokio::task::JoinError),
    /// A domain-level error, e.g. an unrecognised snapshot reference or an
    /// inaccessible paths URL. The inner `String` contains the full message
    /// printed to the user.
    Custom(String),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DownloadError::Reqwest(ref err) => err.fmt(f),
            DownloadError::ReqwestMiddleware(ref err) => err.fmt(f),
            DownloadError::Tokio(ref err) => err.fmt(f),
            DownloadError::Url(ref err) => err.fmt(f),
            DownloadError::Indicatif(ref err) => err.fmt(f),
            DownloadError::Join(ref err) => err.fmt(f),
            DownloadError::Custom(ref err) => err.fmt(f),
        }
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(err: reqwest::Error) -> Self {
        DownloadError::Reqwest(err)
    }
}

impl From<reqwest_middleware::Error> for DownloadError {
    fn from(err: reqwest_middleware::Error) -> Self {
        DownloadError::ReqwestMiddleware(err)
    }
}

impl From<tokio::io::Error> for DownloadError {
    fn from(err: tokio::io::Error) -> Self {
        DownloadError::Tokio(err)
    }
}

impl From<url::ParseError> for DownloadError {
    fn from(err: url::ParseError) -> Self {
        DownloadError::Url(err)
    }
}

impl From<indicatif::style::TemplateError> for DownloadError {
    fn from(err: indicatif::style::TemplateError) -> Self {
        DownloadError::Indicatif(err)
    }
}

impl From<tokio::task::JoinError> for DownloadError {
    fn from(err: tokio::task::JoinError) -> Self {
        DownloadError::Join(err)
    }
}

impl From<String> for DownloadError {
    fn from(s: String) -> DownloadError {
        DownloadError::Custom(s)
    }
}
