#![allow(unstable)]
extern crate rdb;
use std::io::MemReader;
use rdb::{
    read_length,
    read_length_with_encoding,
    verify_magic,
    verify_version,
    read_blob
};

#[test]
fn test_read_length() {
    assert_eq!(
        Ok((0, false)),
        read_length_with_encoding(&mut MemReader::new(vec!(0x0)))
        );

    assert_eq!(
        Ok((16383, false)),
        read_length_with_encoding(&mut MemReader::new(vec!(0x7f, 0xff)))
        );

    assert_eq!(
        Ok((4294967295, false)),
        read_length_with_encoding(&mut MemReader::new(
                vec!(0x80, 0xff, 0xff, 0xff, 0xff)))
        );

    assert_eq!(
        Ok((0, true)),
        read_length_with_encoding(&mut MemReader::new(vec!(0xC0))));

    assert_eq!(
        Ok(16383),
        read_length(&mut MemReader::new(vec!(0x7f, 0xff)))
        );
}

#[test]
fn test_read_blob() {
    assert_eq!(
        Ok(vec![0x61, 0x62, 0x63, 0x64]),
        read_blob(&mut MemReader::new(vec![4, 0x61, 0x62, 0x63, 0x64])));
}

#[test]
fn test_verify_version() {
    assert_eq!(
        Ok(()),
        verify_version(&mut MemReader::new(vec![0x30, 0x30, 0x30, 0x33])));

    match verify_version(&mut MemReader::new(vec![0x30, 0x30, 0x30, 0x3a])) {
        Ok(_) => assert!(false),
        Err(_) => assert!(true)
    }
}

#[test]
fn test_verify_magic() {
    assert_eq!(
        Ok(()),
        verify_magic(&mut MemReader::new(vec![0x52, 0x45, 0x44, 0x49, 0x53])));

    match verify_magic(&mut MemReader::new(vec![0x51, 0x0, 0x0, 0x0, 0x0])) {
        Ok(_) => assert!(false),
        Err(_) => assert!(true)
    }
}
