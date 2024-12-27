use pretty_assertions::assert_eq;
use rdb::{self, filter, formatter};
use rstest::rstest;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;

fn run_dump_test(input: PathBuf, format: &str) -> String {
    let file_stem = input
        .file_stem()
        .expect("File should have a name")
        .to_string_lossy();
    let temp_output = PathBuf::from(format!("/tmp/rdb_test_{}.{}", file_stem, format));

    let file = fs::File::open(&input).expect("Failed to open dump file");
    let reader = BufReader::new(file);
    match format {
        "json" => {
            let formatter = formatter::JSON::new(Some(temp_output.clone()));
            let filter = filter::Simple::new();
            rdb::parse(reader, formatter, filter).expect("Failed to parse RDB file");
        }
        "protocol" => {
            let formatter = formatter::Protocol::new(Some(temp_output.clone()));
            let filter = filter::Simple::new();
            rdb::parse(reader, formatter, filter).expect("Failed to parse RDB file");
        }
        "plain" => {
            let formatter = formatter::Plain::new(Some(temp_output.clone()));
            let filter = filter::Simple::new();
            rdb::parse(reader, formatter, filter).expect("Failed to parse RDB file");
        }
        _ => {
            panic!("Invalid format: {}", format);
        }
    }

    let actual =
        String::from_utf8_lossy(&fs::read(&temp_output).expect("Failed to read output file"))
            .into_owned();

    fs::remove_file(&temp_output).ok();

    actual
}

fn load_expected(path: PathBuf, format: &str) -> String {
    let file_stem = path
        .file_stem()
        .expect("File should have a name")
        .to_string_lossy();
    let expected_json_path = format!("tests/dumps/{}/{}.{}", format, file_stem, format);
    fs::read_to_string(&expected_json_path).unwrap_or_else(|_| {
        String::from_utf8_lossy(&fs::read(&expected_json_path).expect("Failed to read file"))
            .into_owned()
    })
}

#[rstest]
#[case("json")]
#[case("plain")]
#[case("protocol")]
fn test_dump_matches_expected(#[files("tests/dumps/*.rdb")] path: PathBuf, #[case] format: &str) {
    let actual = run_dump_test(path.clone(), format);

    let expected = load_expected(path.clone(), format);

    assert_eq!(
        actual,
        expected,
        "Output doesn't match for {}",
        path.display()
    );
}
