# CC-Downloader

This is an experimental polite downloader for Common Crawl data written in `rust`. This tool is intended for use outside of AWS.

## Todo

- [ ] Add Python bindings
- [ ] Add more tests
- [ ] Handle unrecoverable errors

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

This will extract the binary, the licenses and the readme file **in the current folder**. After extracting the binary, you can run it by executing the following command:

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
  <CRAWL>        Crawl reference, e.g. CC-MAIN-2021-04 or CC-NEWS-2025-01
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

The number of threads can be set using the `-t` flag. The default value is 10. It is advised to use the default value to avoid being blocked by the server. If you make too many requests in a short period of time, you will start receiving `403` errors which are unrecoverable and cannot be retried by the downloader.
