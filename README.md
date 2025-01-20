# CC-Downloader

This is an experimental polite downloader for Common Crawl data writter in `rust`. This tool is intended for use outside of AWS.

## Todo

- [ ] Add Python bindings
- [ ] Add tests
- [ ] Handle unrecoverable errors

## Installation

For now, the only supported way to install the tool is to use `cargo`. For this you need to have `rust` installed. You can install `rust` by following the instructions on the [official website](https://www.rust-lang.org/tools/install).

After installing `rust`, ``cc-downloader`` can be installed with the following command:

```bash
cargo install cc-downloader
```

## Usage

```text
➜ cc-downloader -h
A polite and user-friendly downloader for Common Crawl data.

Usage: cc-downloader [COMMAND]

Commands:
  download-paths  Download paths for a given crawl
  download        Download files from a crawl
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

------

➜ cc-downloader download-paths -h
Download paths for a given crawl

Usage: cc-downloader download-paths <CRAWL> <SUBSET> <DESTINATION>

Arguments:
  <CRAWL>        Crawl reference, e.g. CC-MAIN-2021-04
  <SUBSET>       Data type [possible values: segment, warc, wat, wet, robotstxt, non200responses, cc-index, cc-index-table]
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

The number of threads can be set using the `-t` flag. The default value is 10. It is advised to use the default value to avoid being blocked by the server. If you make too many requests in a short period of time, you will satrt receiving `403` errors which are unrecoverable and cannot be retried by the downloader.
