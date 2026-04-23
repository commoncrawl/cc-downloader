"""Python bindings for the Common Crawl downloader."""

from .cc_downloader import download, download_contrib_paths, download_paths

__all__ = ["download_paths", "download_contrib_paths", "download"]
