use std::fmt;

#[derive(Debug)]
/// An error type for the download process, encapsulating various error sources.
pub enum DownloadError {
    Reqwest(reqwest::Error),
    ReqwestMiddleware(reqwest_middleware::Error),
    Tokio(tokio::io::Error),
    Url(url::ParseError),
    Indicatif(indicatif::style::TemplateError),
    Join(tokio::task::JoinError),
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
