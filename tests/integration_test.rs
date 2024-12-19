use rdb::{self, filter, formatter};
use rstest::rstest;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;

#[rstest]
fn test_dump_matches_expected_json(#[files("tests/dumps/*.rdb")] path: PathBuf) {
    let file_stem = path
        .file_stem()
        .expect("File should have a name")
        .to_string_lossy();

    println!("Testing dump: {}", file_stem);

    let temp_output = format!("/tmp/rdb_test_{}.json", file_stem);

    let file = fs::File::open(&path).expect("Failed to open dump file");
    let reader = BufReader::new(file);
    let formatter = formatter::JSON::new(Some(&temp_output));
    let filter = filter::Simple::new();
    rdb::parse(reader, formatter, filter).expect("Failed to parse RDB file");

    let actual = fs::read_to_string(&temp_output).expect("Failed to read output file");

    fs::remove_file(&temp_output).ok();

    let expected_json_path = path
        .with_file_name(format!("{}.json", file_stem))
        .parent()
        .unwrap()
        .join("json")
        .join(format!("{}.json", file_stem));

    let expected = fs::read_to_string(&expected_json_path).unwrap_or_else(|_| {
        String::from_utf8_lossy(&fs::read(&expected_json_path).expect("Failed to read file"))
            .into_owned()
    });

    assert_eq!(
        actual.trim(),
        expected.trim(),
        "Output doesn't match for {}",
        path.display()
    );
}
