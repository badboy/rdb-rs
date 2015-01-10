extern crate rdb;
use rdb::*;
use std::io::MemReader;

#[test]
fn test_read_length() {
    assert_eq!(
        (0, false),
        read_length_with_encoding(&mut MemReader::new(vec!(0x0)))
        );

    assert_eq!(
        (16383, false),
        read_length_with_encoding(&mut MemReader::new(vec!(0x7f, 0xff)))
        );

    assert_eq!(
        (4294967295, false),
        read_length_with_encoding(&mut MemReader::new(
                vec!(0x80, 0xff, 0xff, 0xff, 0xff)))
        );

    assert_eq!(
        (0, true),
        read_length_with_encoding(&mut MemReader::new(vec!(0xC0))));

    assert_eq!(
        16383,
        read_length(&mut MemReader::new(vec!(0x7f, 0xff)))
        );
}

#[test]
fn test_read_blob() {
    assert_eq!(
        vec!(0x61, 0x62, 0x63, 0x64),
        read_blob(&mut MemReader::new(vec!(4, 0x61, 0x62, 0x63, 0x64))));
}

#[test]
fn test_verify_version() {
    let mut parser = RdbParser::new(MemReader::new(vec![0x30, 0x30, 0x30, 0x33]), NilFormatter);
    assert_eq!(
        true,
        parser.verify_version());

    let mut parser = RdbParser::new(MemReader::new(vec![0x30, 0x30, 0x30, 0x3a]), NilFormatter);
    assert_eq!(
        false,
        parser.verify_version());
}

#[test]
fn test_verify_magic() {
    let mut parser = RdbParser::new(MemReader::new(vec![0x52, 0x45, 0x44, 0x49, 0x53]), NilFormatter);
    assert_eq!(
        true,
        parser.verify_magic());

    let mut parser = RdbParser::new(MemReader::new(vec![0x51, 0x0, 0x0, 0x0, 0x0]), NilFormatter);
    assert_eq!(
        false,
        parser.verify_magic());
}
