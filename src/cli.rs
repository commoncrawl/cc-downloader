use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use regex::Regex;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download paths for a given crawl
    DownloadPaths {
        /// Crawl reference, e.g. CC-MAIN-2021-04 or CC-NEWS-2025-01
        #[arg(value_name = "CRAWL", value_parser = crawl_name_format)]
        snapshot: String,

        /// Data type
        #[arg(value_name = "SUBSET")]
        data_type: DataType,

        /// Destination folder
        #[arg(value_name = "DESTINATION")]
        dst: PathBuf,
    },

    /// Download files from a crawl
    Download {
        /// Path file
        #[arg(value_name = "PATHS")]
        path_file: PathBuf,

        /// Destination folder
        #[arg(value_name = "DESTINATION")]
        dst: PathBuf,

        /// Download files without the folder structure. This only works for WARC/WET/WAT files
        #[arg(short, long)]
        files_only: bool,

        ///Enumerate output files for compatibility with Ungoliant Pipeline. This only works for WET files
        #[arg(short, long)]
        numbered: bool,

        /// Number of threads to use
        #[arg(short, long, default_value = "10", value_name = "NUMBER OF THREADS")]
        threads: usize,

        /// Maximum number of retries per file
        #[arg(
            short,
            long,
            default_value = "1000",
            value_name = "MAX RETRIES PER FILE"
        )]
        retries: usize,

        /// Print progress
        #[arg(short, long, action)]
        progress: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DataType {
    Segment,
    Warc,
    Wat,
    Wet,
    Robotstxt,
    Non200responses,
    CcIndex,
    CcIndexTable,
}

impl DataType {
    pub fn as_str(&self) -> &str {
        match self {
            DataType::Segment => "segment",
            DataType::Warc => "warc",
            DataType::Wat => "wat",
            DataType::Wet => "wet",
            DataType::Robotstxt => "robotstxt",
            DataType::Non200responses => "non200responses",
            DataType::CcIndex => "cc-index",
            DataType::CcIndexTable => "cc-index-table",
        }
    }
}

fn crawl_name_format(crawl: &str) -> Result<String, String> {
    let main_re = Regex::new(r"^(CC\-MAIN)\-([0-9]{4})\-([0-9]{2})$").unwrap();
    let news_re = Regex::new(r"^(CC\-NEWS)\-([0-9]{4})\-([0-9]{2})$").unwrap();

    if !(main_re.is_match(crawl) || news_re.is_match(crawl)) {
        return Err("Please use the CC-MAIN-YYYY-WW or the CC-NEWS-YYYY-MM format, make sure your input is propely capitalized".to_string());
    } else {
        return Ok(crawl.to_owned());
    }
}
