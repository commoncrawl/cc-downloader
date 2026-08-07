# Accessing Contributor and Project Datasets in Common Crawl

Common Crawl hosts additional datasets created by the community in the `/contrib/` and `/projects/` directories. These datasets may have different formats and structures than the main crawl data, and may not always follow the `.paths.gz` convention.

This guide explains how to discover, download, and use these datasets with `cc-downloader` and other common tools.

---

## Listing Available Datasets

You can list available datasets using the AWS CLI:

```bash
aws s3 ls s3://commoncrawl/contrib/
aws s3 ls s3://commoncrawl/projects/
```

Or browse via HTTP:

- [https://data.commoncrawl.org/contrib/](https://data.commoncrawl.org/contrib/)
- [https://data.commoncrawl.org/projects/](https://data.commoncrawl.org/projects/)

---

## Downloading a `.paths.gz` File

Many datasets provide a `.paths.gz` file listing the available data objects. For example, to download a paths file from a project dataset:

```bash
wget https://data.commoncrawl.org/projects/host-index-testing/v1.paths.gz
gunzip v1.paths.gz
head v1.paths
```

---

## Downloading Files Listed in a `.paths` File

You can use `cc-downloader` to download files listed in a `.paths` file:

```bash
cc-downloader download v1.paths /your/target/directory/
```

---

## Notes and Limitations

- Some contributor datasets may not use the `.paths.gz` format. For these, manual download or custom scripts may be needed.
- If you encounter unsupported formats or have suggestions for improvement, please open an issue or contribute enhancements!
- For more information on data formats and access methods, see the [Common Crawl Get Started Guide](https://commoncrawl.org/get-started).

---

## Example: Python Script for Inspecting a `.paths.gz` File

For users who wish to prototype or inspect datasets before using Rust tools, here is a simple Python example:

```python
import gzip
import requests

url = "https://data.commoncrawl.org/projects/host-index-testing/v1.paths.gz"
local_gz = "v1.paths.gz"
local_txt = "v1.paths"

# Download .paths.gz
with requests.get(url, stream=True) as r:
    r.raise_for_status()
    with open(local_gz, 'wb') as f:
        for chunk in r.iter_content(chunk_size=8192):
            f.write(chunk)

# Unzip
with gzip.open(local_gz, 'rt') as gz, open(local_txt, 'w') as out:
    for line in gz:
        out.write(line)

# Print first 10 file paths
with open(local_txt) as f:
    for i, line in enumerate(f):
        print(line.strip())
        if i >= 9:
            break
```

---

## Feedback

We welcome feedback and contributions to improve support for contributor and project

