"""Python bindings for the Common Crawl downloader."""

import multiprocessing as mp

from .cc_downloader import (
    download as _download,
    download_contrib_paths as _download_contrib_paths,
    download_paths as _download_paths,
)


def _run_in_subprocess(fn, args, kwargs):
    ctx = mp.get_context("spawn")
    p = ctx.Process(target=fn, args=args, kwargs=kwargs)
    p.start()
    try:
        p.join()
    except KeyboardInterrupt:
        p.terminate()
        p.join()
        raise
    if p.exitcode != 0:
        raise RuntimeError(
            f"Download failed (exit code {p.exitcode}). Check stderr for details."
        )


def download_paths(snapshot, data_type, dst, subsets=None, max_retries=1000):
    _run_in_subprocess(
        _download_paths,
        (snapshot, data_type, dst),
        {"subsets": subsets, "max_retries": max_retries},
    )


def download_contrib_paths(url, dst, max_retries=1000):
    _run_in_subprocess(
        _download_contrib_paths,
        (url, dst),
        {"max_retries": max_retries},
    )


def download(paths, dst, threads=10, max_retries=1000, numbered=False, files_only=False, progress=False):
    _run_in_subprocess(
        _download,
        (paths, dst),
        {
            "threads": threads,
            "max_retries": max_retries,
            "numbered": numbered,
            "files_only": files_only,
            "progress": progress,
        },
    )


__all__ = ["download_paths", "download_contrib_paths", "download"]
