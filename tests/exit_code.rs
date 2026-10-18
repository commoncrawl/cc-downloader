use std::process::Command;

fn cc_downloader() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cc-downloader"))
}

#[test]
fn missing_paths_file_exits_with_failure() {
    let status = cc_downloader()
        .args(["download", "does-not-exist.paths.gz", "out"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn numbered_and_files_only_is_a_usage_error() {
    let status = cc_downloader()
        .args(["download", "-n", "-f", "does-not-exist.paths.gz", "out"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}
