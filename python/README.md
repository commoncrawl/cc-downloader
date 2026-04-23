# cc-downloader

Python bindings for [cc-downloader](https://github.com/commoncrawl/cc-downloader), a polite and user-friendly client for downloading [Common Crawl](https://commoncrawl.org) data.

The library is implemented in Rust and exposes a synchronous Python API. All downloads run inside a multi-threaded Tokio runtime, so concurrent file fetching works exactly as in the CLI — the `threads` parameter controls how many files are downloaded in parallel.

## Requirements

- Python ≥ 3.12

## Installation

```bash
pip install cc-downloader
```

```bash
uv add cc-downloader
```

## Quick start

The workflow has two steps: fetch a paths index for a crawl snapshot, then download every file it lists.

```python
import cc_downloader

# Step 1: download the paths index for WET files.
cc_downloader.download_paths(
    snapshot="CC-MAIN-2024-46",
    data_type="wet",
    dst="./output",
)

# Step 2: download every file listed in the index.
cc_downloader.download(
    paths="./output/wet.paths.gz",
    dst="./output",
    threads=10,
    progress=True,
)
```

## API reference

### `download_paths`

```python
cc_downloader.download_paths(
    snapshot: str,
    data_type: str,
    dst: str,
    subsets: list[str] | None = None,
    max_retries: int = 1000,
) -> None
```

Downloads the gzip-compressed paths index file for a given crawl snapshot and data type to `dst`.

| Parameter | Description |
| --- | --- |
| `snapshot` | Crawl identifier, e.g. `"CC-MAIN-2024-46"` or `"CC-NEWS-2025-01"` |
| `data_type` | Data type: `"warc"`, `"wet"`, `"wat"`, `"robotstxt"`, `"non200responses"`, `"cc-index"`, `"cc-index-table"` |
| `dst` | Destination directory |
| `subsets` | For `cc-index-table` only: list of subsets to keep. Valid values: `"crawldiagnostics"`, `"robotstxt"`, `"warc"`. Defaults to all three. |
| `max_retries` | Maximum retry attempts per request (default `1000`) |

**Example — `cc-index-table` with subset filtering:**

```python
cc_downloader.download_paths(
    snapshot="CC-MAIN-2024-46",
    data_type="cc-index-table",
    dst="./output",
    subsets=["warc", "robotstxt"],
)
```

---

### `download_contrib_paths`

```python
cc_downloader.download_contrib_paths(
    url: str,
    dst: str,
    max_retries: int = 1000,
) -> None
```

Downloads a paths index file directly from a [contributor dataset](https://data.commoncrawl.org/contrib/index.html) URL.

| Parameter | Description |
| --- | --- |
| `url` | Direct URL to the paths file. Must start with `https://data.commoncrawl.org/contrib/` |
| `dst` | Destination directory |
| `max_retries` | Maximum retry attempts per request (default `1000`) |

**Example:**

```python
cc_downloader.download_contrib_paths(
    url="https://data.commoncrawl.org/contrib/my-dataset/paths.gz",
    dst="./output",
)
```

---

### `download`

```python
cc_downloader.download(
    paths: str,
    dst: str,
    threads: int = 10,
    max_retries: int = 1000,
    numbered: bool = False,
    files_only: bool = False,
    progress: bool = False,
) -> None
```

Downloads every file listed in a `.paths.gz` index. Files are fetched concurrently up to `threads` at a time, with exponential-backoff retries on transient failures.

| Parameter | Description |
| --- | --- |
| `paths` | Path to a `.paths.gz` index file |
| `dst` | Destination directory |
| `threads` | Maximum concurrent downloads (default `10`) |
| `max_retries` | Maximum retry attempts per file (default `1000`) |
| `numbered` | Rename output files sequentially (`0.txt.gz`, `1.txt.gz`, …). Only for WET files. Mutually exclusive with `files_only`. |
| `files_only` | Write files flat into `dst` with no subdirectory structure. Only for WARC/WET/WAT files. Mutually exclusive with `numbered`. |
| `progress` | Show a per-file progress bar |

**Example:**

```python
cc_downloader.download(
    paths="./output/wet.paths.gz",
    dst="./output",
    threads=10,
    progress=True,
)
```

## Politeness and retries

Every HTTP request uses an exponential-backoff retry policy (bounds: 1 s – 1 h, bounded jitter, base 2). The default of 1 000 retries is intentionally high: Common Crawl is a shared public resource and transient failures are common under load. This tool is intended for use **outside of AWS**. You can monitor Common Crawl infrastructure traffic on the [Infrastructure Status Webpage](https://status.commoncrawl.org).

Avoid setting `threads` much higher than the default of 10. Too many concurrent requests in a short period can trigger `403` errors, which are unrecoverable and cannot be retried.

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](https://github.com/commoncrawl/cc-downloader/blob/main/CONTRIBUTING.md) for guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/commoncrawl/cc-downloader/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/commoncrawl/cc-downloader/blob/main/LICENSE-MIT) at your option.
