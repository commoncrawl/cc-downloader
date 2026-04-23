# cc-downloader

A polite and user-friendly command-line tool for downloading [Common Crawl](https://commoncrawl.org) data, written in Rust.

Common Crawl has seen a significant increase in data downloads in recent months, which has made it difficult for some users to successfully retrieve data from the storage bucket. CC-Downloader was built to address this: it implements a polite retry mechanism with exponential backoff and jitter to avoid overwhelming the infrastructure, while ensuring that every requested file is downloaded successfully.

Key features:

- **Polite retry with exponential backoff and jitter** -- prevents synchronized request patterns and gradually increases wait times between retries
- **Concurrent downloads** -- processes multiple files simultaneously (default: 10 threads)
- **Folder structure preservation** -- maintains the internal tree structure of Common Crawl data by default
- **Cross-platform** -- pre-compiled binaries for Linux, macOS, and Windows

This tool is intended for use outside of AWS. You can monitor Common Crawl infrastructure traffic on the [Infrastructure Status Webpage](https://status.commoncrawl.org).

## Quick Start

Install via cargo:

```bash
cargo install cc-downloader
```

The workflow has two steps. First, download the file paths for a given crawl and data type:

```bash
cc-downloader download-paths crawl CC-MAIN-2024-46 wet path/to/folder
```

This produces a `wet.paths.gz` file. Then, download the actual data:

```bash
cc-downloader download path/to/folder/wet.paths.gz path/to/folder
```

For `cc-index-table` data you can optionally filter to one or more subsets (`crawldiagnostics`, `robotstxt`, `warc`). Without `--subset` all three are downloaded:

```bash
cc-downloader download-paths crawl CC-MAIN-2024-46 cc-index-table path/to/folder --subset warc robotstxt
```

To download paths from a [contributor dataset](https://data.commoncrawl.org/contrib/index.html), use the `contrib` subcommand with the direct URL to the paths file:

```bash
cc-downloader download-paths contrib https://data.commoncrawl.org/contrib/<dataset>/paths.gz path/to/folder
```

## Installation

You can install `cc-downloader` via our pre-built binaries, or by compiling it from source.

### Pre-built binaries

You can find our pre-built binaries on our [GitHub releases page](https://github.com/commoncrawl/cc-downloader/releases). They are available for `Linux`, `macOS`, and `Windows`, in `x86_64` and `aarch64` architectures (Windows is only supported in `x86_64`). In order to use them please select and download the correct binary for your system.

```bash
wget https://github.com/commoncrawl/cc-downloader/releases/download/[VERSION]/cc-downloader-[VERSION]-[ARCH]-[OS].[COMPRESSION-FORMAT]
```

After downloading it, please verify the checksum of the binary. You can find the checksum file in the same location as the binary. The checksum is generated using `sha512sum`. You can verify it by running the following command:

```bash
wget https://github.com/commoncrawl/cc-downloader/releases/download/[VERSION]/cc-downloader-[VERSION]-[ARCH]-[OS].sha512
sha512sum -c cc-downloader-[VERSION]-[ARCH]-[OS].sha512
```

If the checksum is valid, which will be indicated by and `OK` message, you can proceed to extract the binary. For `tar.gz` files you can use the following command:

```bash
tar -xzf cc-downloader-[VERSION]-[ARCH]-[OS].tar.gz
```

For `zip` files you can use the following command:

```bash
unzip cc-downloader-[VERSION]-[ARCH]-[OS].zip
```

This will extract the binary, the licenses and the `README.md` file **in the current folder**. After extracting the binary, you can run it by executing the following command:

```bash
./cc-downloader
```

If you want to use the binary from anywhere, you can move it to a folder in your `PATH`. For more information on how to do this, please refer to the documentation of your operating system. For example, on `Linux` and `macOS` you can move it to `~/.bin`:

```bash
mv cc-downloader ~/.bin
```

And then add the following line to your `~/.bashrc` or `~/.zshrc` file:

```bash
export PATH=$PATH:~/.bin
```

then run the following command to apply the changes:

```bash
source ~/.bashrc
```

or

```bash
source ~/.zshrc
```

Then, you can run the binary from anywhere. If you want to update the binary, you can repeat the process and download the new version. Make sure to replace the binary that is stored in the folder that you added to your `PATH`. If you want to remove the binary, you can simply delete from this folder.

### Compiling from source

For this you need to have `rust` installed. You can install `rust` by following the instructions on the [official website](https://www.rust-lang.org/tools/install).

Or by running the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Even if you have `rust` a system-wide installation, we recommend the linked installation method. A system-wide installation and a user installation can co-exist without any problems.

When compiling from source, please make sure you have the latest version of `rust` installed by running the following command:

```bash
rustup update
```

Now you can install the `cc-downloader` tool by running the following command:

```bash
cargo install cc-downloader
```

## Usage

```text
➜ cc-downloader -h
A polite and user-friendly downloader for Common Crawl data.

Usage: cc-downloader <COMMAND>

Commands:
  download-paths  Download paths for a given crawl and data type, or from a contributor dataset
  download        Download files from a crawl
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

------

➜ cc-downloader download-paths -h
Download paths for a given crawl and data type, or from a contributor dataset

Usage: cc-downloader download-paths <COMMAND>

Commands:
  crawl    Download paths for a standard crawl snapshot
  contrib  Download paths from a contributor dataset
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help

------

➜ cc-downloader download-paths crawl -h
Download paths for a standard crawl snapshot

Usage: cc-downloader download-paths crawl [OPTIONS] <CRAWL> <DATA_TYPE> <DESTINATION>

Arguments:
  <CRAWL>        Crawl reference, e.g. CC-MAIN-2021-04 or CC-NEWS-2025-01
  <DATA_TYPE>    Data type [possible values: segment, warc, wat, wet, robotstxt, non200responses, cc-index, cc-index-table]
  <DESTINATION>  Destination folder

Options:
      --subset <SUBSETS>...  Subsets to download (only valid for cc-index-table). Defaults to all three if omitted: crawldiagnostics, robotstxt, warc [possible values: crawldiagnostics, robotstxt, warc]
  -h, --help                 Print help

------

➜ cc-downloader download-paths contrib -h
Download paths from a contributor dataset

Usage: cc-downloader download-paths contrib <URL> <DESTINATION>

Arguments:
  <URL>          URL of the contributor paths file. Must start with https://data.commoncrawl.org/contrib/
  <DESTINATION>  Destination folder

Options:
  -h, --help  Print help

------

➜ cc-downloader download -h
Download files from a crawl

Usage: cc-downloader download [OPTIONS] <PATHS> <DESTINATION>

Arguments:
  <PATHS>        Path file
  <DESTINATION>  Destination folder

Options:
  -f, --files-only                      Download files without the folder structure. This only works for WARC/WET/WAT files
  -n, --numbered                        Enumerate output files for compatibility with Ungoliant Pipeline. This only works for WET files
  -t, --threads <NUMBER OF THREADS>     Number of threads to use [default: 10]
  -r, --retries <MAX RETRIES PER FILE>  Maximum number of retries per file [default: 1000]
  -p, --progress                        Print progress
  -h, --help                            Print help
```

## Number of threads

The number of threads can be set using the `-t` flag. The default value is 10. It is advised to use the default value to avoid being blocked by the server. If you make too many requests in a short period of time, you will start receiving `403` errors which are unrecoverable and cannot be retried by the downloader.

## As a Rust library

`cc-downloader` is also available as a library crate on [crates.io](https://crates.io/crates/cc-downloader). Add it to your project:

```toml
[dependencies]
cc-downloader = "0.6"
tokio = { version = "1", features = ["full"] }
```

The workflow mirrors the CLI: fetch a paths index first, then download the listed files.

```rust
use cc_downloader::download::{DownloadOptions, download_paths, download};

#[tokio::main]
async fn main() -> Result<(), cc_downloader::errors::DownloadError> {
    // Step 1: fetch the paths index for WET files.
    let paths_options = DownloadOptions {
        snapshot: "CC-MAIN-2024-46".to_string(),
        data_type: "wet",
        dst: std::path::Path::new("./output"),
        ..Default::default()
    };
    download_paths(paths_options).await?;

    // Step 2: download every file listed in the index.
    let download_options = DownloadOptions {
        paths: std::path::Path::new("./output/wet.paths.gz"),
        dst: std::path::Path::new("./output"),
        threads: 10,
        progress: true,
        ..Default::default()
    };
    download(download_options).await?;

    Ok(())
}
```

For `cc-index-table`, filter to specific subsets via `cc_index_table_subsets`:

```rust
let options = DownloadOptions {
    snapshot: "CC-MAIN-2024-46".to_string(),
    data_type: "cc-index-table",
    dst: std::path::Path::new("./output"),
    cc_index_table_subsets: vec!["warc".to_string(), "robotstxt".to_string()],
    ..Default::default()
};
download_paths(options).await?;
```

For contributor datasets, use `download_contrib_paths` instead:

```rust
use cc_downloader::download::download_contrib_paths;

download_contrib_paths(
    "https://data.commoncrawl.org/contrib/my-dataset/paths.gz",
    std::path::Path::new("./output"),
    1000, // max retries
).await?;
```

Full API documentation is available on [docs.rs](https://docs.rs/cc-downloader).

## Python bindings

Python bindings are available as [`cc-downloader`](https://pypi.org/project/cc-downloader/) on PyPI. See the [Python README](python/README.md) for installation and usage instructions.

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to get involved.

## Todo

- [ ] Add more tests
- [ ] Handle unrecoverable errors
