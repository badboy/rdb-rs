#![cfg(feature = "integration_tests")]

use rstest::rstest;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

fn run_rdb_json(dump_path: &Path) -> String {
    let cargo_path = env!("CARGO_MANIFEST_DIR");
    let binary = if cfg!(debug_assertions) {
        format!("{}/target/debug/rdb", cargo_path)
    } else {
        format!("{}/target/release/rdb", cargo_path)
    };

    let output = Command::new(&binary)
        .args(["--format", "json"])
        .arg(dump_path)
        .output()
        .expect("Failed to execute rdb command");

    String::from_utf8(output.stdout)
        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned())
}

#[rstest]
fn test_dump_matches_expected_json(#[files("tests/dumps/*.rdb")] path: PathBuf) {
    // Build the project first
    assert!(Command::new("cargo")
        .arg("build")
        .status()
        .expect("Failed to build project")
        .success());

    let file_stem = path
        .file_stem()
        .expect("File should have a name")
        .to_string_lossy();
        
    println!("Testing dump: {}", file_stem);

    // Get the expected JSON file path
    let expected_json_path = path
        .with_file_name(format!("{}.json", file_stem))
        .parent()
        .unwrap()
        .join("json")
        .join(format!("{}.json", file_stem));

    // Read expected JSON
    let expected = fs::read_to_string(&expected_json_path)
        .unwrap_or_else(|_| String::from_utf8_lossy(&fs::read(&expected_json_path)
            .expect("Failed to read file")).into_owned());

    // Run rdb and get actual JSON
    let actual = run_rdb_json(&path);

    // Compare JSON contents
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "Output doesn't match for {}",
        path.display()
    );
}
