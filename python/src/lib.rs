use ::cc_downloader::download::{
    DownloadOptions, download as cc_download, download_contrib_paths as cc_download_contrib_paths,
    download_paths as cc_download_paths,
};
use pyo3::prelude::*;
use tokio::runtime::Runtime;

/// Downloads the paths index file for a Common Crawl snapshot and data type.
///
/// For `cc-index-table`, pass a list of subset names to `subsets` to restrict
/// the downloaded index to only those subsets. Valid values are
/// ``"crawldiagnostics"``, ``"robotstxt"``, and ``"warc"``. Omit `subsets`
/// (or pass ``None``) to download all three.
///
/// :param snapshot: Crawl snapshot identifier, e.g. ``"CC-MAIN-2024-46"``
///     or ``"CC-NEWS-2025-01"``.
/// :param data_type: Data type to download, e.g. ``"warc"``, ``"wet"``,
///     ``"cc-index-table"``.
/// :param dst: Destination directory where the paths file will be written.
/// :param subsets: Optional list of ``cc-index-table`` subset names to keep.
/// :param max_retries: Maximum per-request retry attempts (default ``1000``).
/// :raises RuntimeError: If the download fails.
#[pyfunction]
#[pyo3(signature = (snapshot, data_type, dst, subsets=None, max_retries=1000))]
fn download_paths(
    py: Python<'_>,
    snapshot: String,
    data_type: String,
    dst: String,
    subsets: Option<Vec<String>>,
    max_retries: usize,
) -> PyResult<()> {
    py.detach(|| {
        let rt = Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            let options = DownloadOptions {
                snapshot,
                data_type: &data_type,
                dst: std::path::Path::new(&dst),
                cc_index_table_subsets: subsets.unwrap_or_default(),
                max_retries,
                ..Default::default()
            };
            cc_download_paths(options).await
        })
        .map_err(|e| e.to_string())
    })
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
}

/// Downloads a paths index file from a contributor dataset URL.
///
/// The URL must point to a gzip-compressed paths file hosted under
/// ``https://data.commoncrawl.org/contrib/``.
///
/// :param url: Direct URL to the contributor paths file.
/// :param dst: Destination directory where the paths file will be written.
/// :param max_retries: Maximum per-request retry attempts (default ``1000``).
/// :raises RuntimeError: If the download fails.
#[pyfunction]
#[pyo3(signature = (url, dst, max_retries=1000))]
fn download_contrib_paths(
    py: Python<'_>,
    url: String,
    dst: String,
    max_retries: usize,
) -> PyResult<()> {
    py.detach(|| {
        let rt = Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(cc_download_contrib_paths(
            &url,
            std::path::Path::new(&dst),
            max_retries,
        ))
        .map_err(|e| e.to_string())
    })
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
}

/// Downloads every file listed in a previously obtained paths index.
///
/// Internally, up to `threads` files are downloaded concurrently with
/// exponential-backoff retries. The call responds to Ctrl+C: pressing it
/// raises ``KeyboardInterrupt`` and stops the download.
///
/// :param paths: Path to a ``.paths.gz`` index file.
/// :param dst: Destination directory where downloaded files will be written.
/// :param threads: Maximum number of concurrent downloads (default ``10``).
/// :param max_retries: Maximum per-file retry attempts (default ``1000``).
/// :param numbered: Rename output files sequentially (``0.txt.gz``,
///     ``1.txt.gz``, …). Only for WET files. Mutually exclusive with
///     `files_only`.
/// :param files_only: Write files flat into `dst` with no subdirectory
///     structure. Only for WARC/WET/WAT files. Mutually exclusive with
///     `numbered`.
/// :param progress: Show a per-file progress bar.
/// :raises ValueError: If both `numbered` and `files_only` are ``True``.
/// :raises RuntimeError: If the download fails.
#[pyfunction]
#[pyo3(signature = (paths, dst, threads=10, max_retries=1000, numbered=false, files_only=false, progress=false))]
fn download(
    py: Python<'_>,
    paths: String,
    dst: String,
    threads: usize,
    max_retries: usize,
    numbered: bool,
    files_only: bool,
    progress: bool,
) -> PyResult<()> {
    if numbered && files_only {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "numbered and files_only are mutually exclusive",
        ));
    }
    py.detach(|| {
        let rt = Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            let options = DownloadOptions {
                paths: std::path::Path::new(&paths),
                dst: std::path::Path::new(&dst),
                threads,
                max_retries,
                numbered,
                files_only,
                progress,
                ..Default::default()
            };
            cc_download(options).await
        })
        .map_err(|e| e.to_string())
    })
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
}

#[pymodule]
fn cc_downloader(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(download_paths, m)?)?;
    m.add_function(wrap_pyfunction!(download_contrib_paths, m)?)?;
    m.add_function(wrap_pyfunction!(download, m)?)?;
    Ok(())
}
