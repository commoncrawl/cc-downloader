use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use regex::Regex;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download paths for a given crawl and data type, or from a contributor dataset
    DownloadPaths {
        #[command(subcommand)]
        source: DownloadPathsSource,
    },

    /// Download files from a crawl
    #[command(arg_required_else_help = true)]
    Download {
        /// Path file
        #[arg(value_name = "PATHS")]
        path_file: PathBuf,

        /// Destination folder
        #[arg(value_name = "DESTINATION")]
        dst: PathBuf,

        /// Download files without the folder structure. This only works for WARC/WET/WAT files
        #[arg(short, long, conflicts_with = "numbered")]
        files_only: bool,

        ///Enumerate output files for compatibility with Ungoliant Pipeline. This only works for WET files
        #[arg(short, long)]
        #[arg(
            short,
            long,
            default_value = "10",
            value_name = "NUMBER OF THREADS",
            value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..)
        )]
        numbered: bool,

        /// Number of threads to use
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

#[derive(Subcommand)]
pub enum DownloadPathsSource {
    /// Download paths for a standard crawl archive
    #[command(arg_required_else_help = true)]
    Crawl {
        /// Crawl archive identifier, e.g. CC-MAIN-2026-39 or CC-NEWS-2026-09
        #[arg(value_name = "CRAWL", value_parser = crawl_name_format)]
        crawl: String,

        /// Data type
        #[arg(value_name = "DATA_TYPE")]
        data_type: DataType,

        /// Destination folder
        #[arg(value_name = "DESTINATION")]
        dst: PathBuf,

        /// Subsets to download (only valid for cc-index-table).
        /// Repeat the flag or separate values with commas, e.g. --subset warc,robotstxt.
        /// Defaults to all three if omitted: crawldiagnostics, robotstxt, warc.
        #[arg(
            long = "subset",
            value_name = "SUBSET",
            value_enum,
            num_args = 1,
            action = ArgAction::Append,
            value_delimiter = ','
        )]
        subsets: Vec<CcIndexTableSubset>,

        // Backward compatibility: `... <DESTINATION> --subset warc robotstxt`.
        // `--subset` takes one value, so the extra space-separated values land here.
        #[arg(value_name = "SUBSET", value_enum, hide = true, requires = "subsets")]
        trailing_subsets: Vec<CcIndexTableSubset>,
    },

    /// Download paths from a contributor dataset
    #[command(arg_required_else_help = true)]
    Contrib {
        /// URL of the contributor paths file.
        /// Must start with https://data.commoncrawl.org/contrib/
        #[arg(value_name = "URL", value_parser = contrib_url_validator)]
        url: String,

        /// Destination folder
        #[arg(value_name = "DESTINATION")]
        dst: PathBuf,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum CcIndexTableSubset {
    Crawldiagnostics,
    Robotstxt,
    Warc,
}

impl CcIndexTableSubset {
    pub fn as_str(&self) -> &str {
        match self {
            CcIndexTableSubset::Crawldiagnostics => "crawldiagnostics",
            CcIndexTableSubset::Robotstxt => "robotstxt",
            CcIndexTableSubset::Warc => "warc",
        }
    }
}

fn contrib_url_validator(url: &str) -> Result<String, String> {
    if url.starts_with("https://data.commoncrawl.org/contrib/") {
        Ok(url.to_string())
    } else {
        Err("URL must start with https://data.commoncrawl.org/contrib/".to_string())
    }
}

fn crawl_name_format(crawl: &str) -> Result<String, String> {
    let main_re = Regex::new(r"^(CC\-MAIN)\-([0-9]{4})\-([0-9]{2})$").unwrap();
    let news_re = Regex::new(r"^(CC\-NEWS)\-([0-9]{4})\-([0-9]{2})$").unwrap();

    let crawl_ref = crawl.to_uppercase();

    if !(main_re.is_match(&crawl_ref) || news_re.is_match(&crawl_ref)) {
        Err("Please use the CC-MAIN-YYYY-WW or the CC-NEWS-YYYY-MM format.".to_string())
    } else {
        Ok(crawl_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    use CcIndexTableSubset::{Crawldiagnostics, Robotstxt, Warc};

    fn parse_subsets(args: &[&str]) -> Vec<CcIndexTableSubset> {
        let argv = ["cc-downloader", "download-paths", "crawl"]
            .iter()
            .chain(args)
            .copied();
        match Cli::try_parse_from(argv).unwrap().command {
            Commands::DownloadPaths {
                source:
                    DownloadPathsSource::Crawl {
                        mut subsets,
                        trailing_subsets,
                        ..
                    },
            } => {
                subsets.extend(trailing_subsets);
                subsets
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn subset_before_positionals() {
        let pos = ["CC-MAIN-2026-39", "cc-index-table", "out"];
        for opts in [
            &["--subset", "warc,robotstxt"][..],
            &["--subset=warc,robotstxt"],
            &["--subset", "warc", "--subset", "robotstxt"],
        ] {
            let args: Vec<&str> = opts.iter().chain(&pos).copied().collect();
            assert_eq!(parse_subsets(&args), [Warc, Robotstxt], "{args:?}");
        }
    }

    #[test]
    fn subset_after_positionals() {
        let pos = ["CC-MAIN-2026-39", "cc-index-table", "out"];
        for opts in [
            &["--subset", "warc", "robotstxt"][..],
            &["--subset", "warc,robotstxt"],
            &["--subset", "warc", "--subset", "robotstxt"],
        ] {
            let args: Vec<&str> = pos.iter().chain(opts).copied().collect();
            assert_eq!(parse_subsets(&args), [Warc, Robotstxt], "{args:?}");
        }
    }

    #[test]
    fn bare_trailing_subset_requires_flag() {
        let argv = [
            "cc-downloader",
            "download-paths",
            "crawl",
            "CC-MAIN-2026-39",
            "cc-index-table",
            "out",
            "warc",
        ];
        assert!(Cli::try_parse_from(argv).is_err());
    }

    #[test]
    fn all_three_subsets_in_any_position() {
        let pos = ["CC-MAIN-2026-39", "cc-index-table", "out"];
        let all = ["warc", "robotstxt", "crawldiagnostics"];
        let joined = all.join(",");
        let eq = format!("--subset={joined}");
        let mut flags: Vec<&str> = Vec::new();
        for s in all {
            flags.extend(["--subset", s]);
        }
        let mut forms: Vec<Vec<&str>> = Vec::new();
        for opts in [vec!["--subset", joined.as_str()], vec![eq.as_str()], flags] {
            forms.push(opts.iter().chain(&pos).copied().collect());
            forms.push(pos.iter().chain(&opts).copied().collect());
        }
        forms.push(
            pos.iter()
                .chain(&["--subset"])
                .chain(&all)
                .copied()
                .collect(),
        );
        for args in forms {
            let mut subsets = parse_subsets(&args);
            subsets.sort();
            assert_eq!(subsets, [Crawldiagnostics, Robotstxt, Warc], "{args:?}");
        }
    }
}
