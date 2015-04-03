extern crate rdb;
use std::io::Cursor;
use rdb::parser::{
    read_length,
    read_length_with_encoding,
    verify_magic,
    verify_version,
    read_blob
};

#[test]
fn test_read_length() {
    assert_eq!(
        (0, false),
        read_length_with_encoding(&mut Cursor::new(vec!(0x0))).unwrap()
        );

    assert_eq!(
        (16383, false),
        read_length_with_encoding(&mut Cursor::new(vec!(0x7f, 0xff))).unwrap()
        );

    assert_eq!(
        (4294967295, false),
        read_length_with_encoding(&mut Cursor::new(
                vec!(0x80, 0xff, 0xff, 0xff, 0xff))).unwrap()
        );

    assert_eq!(
        (0, true),
        read_length_with_encoding(&mut Cursor::new(vec!(0xC0))).unwrap()
        );

    assert_eq!(
        16383,
        read_length(&mut Cursor::new(vec!(0x7f, 0xff))).unwrap()
        );
}

#[test]
fn test_read_blob() {
    assert_eq!(
        vec![0x61, 0x62, 0x63, 0x64],
        read_blob(&mut Cursor::new(vec![4, 0x61, 0x62, 0x63, 0x64])).unwrap()
        );
}

#[test]
fn test_verify_version() {
    assert_eq!(
        (),
        verify_version(&mut Cursor::new(vec![0x30, 0x30, 0x30, 0x33])).unwrap()
        );

    match verify_version(&mut Cursor::new(vec![0x30, 0x30, 0x30, 0x3a])) {
        Ok(_) => assert!(false),
        Err(_) => assert!(true)
    }
}

#[test]
fn test_verify_magic() {
    assert_eq!(
        (),
        verify_magic(&mut Cursor::new(vec![0x52, 0x45, 0x44, 0x49, 0x53])).unwrap()
        );

    match verify_magic(&mut Cursor::new(vec![0x51, 0x0, 0x0, 0x0, 0x0])) {
        Ok(_) => assert!(false),
        Err(_) => assert!(true)
    }
}
