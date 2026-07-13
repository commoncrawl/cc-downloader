"""Usage examples for the cc-downloader Python bindings."""

import cc_downloader

# Step 1: download the paths index for WET files
cc_downloader.download_paths(
    snapshot="CC-MAIN-2024-46",
    data_type="wet",
    dst="./output",
)

# Step 2: download every file listed in the index (10 concurrent threads)
cc_downloader.download(
    paths="./output/wet.paths.gz",
    dst="./output",
    threads=10,
    progress=True,
)

# --- cc-index-table: only the warc and robotstxt subsets ---
cc_downloader.download_paths(
    snapshot="CC-MAIN-2024-46",
    data_type="cc-index-table",
    dst="./output",
    subsets=["warc", "robotstxt"],
)

# --- contributor dataset ---
cc_downloader.download_contrib_paths(
    url="https://data.commoncrawl.org/contrib/my-dataset/paths.gz",
    dst="./output",
)
