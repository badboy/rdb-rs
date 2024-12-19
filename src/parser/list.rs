use super::common::utils::{other_error, read_blob, read_length};
use super::common::{read_ziplist_entry_string, read_ziplist_metadata};
use crate::formatter::Formatter;
use crate::types::{EncodingType, RdbOk, Type};
use byteorder::ReadBytesExt;
use std::io::{Cursor, Read};

pub fn read_linked_list<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
    typ: Type,
) -> RdbOk {
    let mut len = read_length(input)?;

    match typ {
        Type::List => {
            formatter.start_list(key, len, last_expiretime, EncodingType::LinkedList);
        }
        Type::Set => {
            formatter.start_set(key, len, last_expiretime, EncodingType::LinkedList);
        }
        _ => {
            panic!("Unknown encoding type for linked list")
        }
    }

    while len > 0 {
        let blob = read_blob(input)?;
        formatter.list_element(key, &blob);
        len -= 1;
    }

    formatter.end_list(key);
    Ok(())
}

pub fn read_list_ziplist<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let ziplist = read_blob(input)?;
    let raw_length = ziplist.len() as u64;

    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    formatter.start_list(
        key,
        zllen as u32,
        last_expiretime,
        EncodingType::Ziplist(raw_length),
    );

    for _ in 0..zllen {
        let entry = read_ziplist_entry_string(&mut reader)?;
        formatter.list_element(key, &entry);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(other_error("Invalid end byte of ziplist"));
    }

    formatter.end_list(key);

    Ok(())
}

pub fn read_quicklist<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let len = read_length(input)?;

    formatter.start_set(key, 0, last_expiretime, EncodingType::Quicklist);
    for _ in 0..len {
        read_quicklist_ziplist(input, formatter, key)?;
    }
    formatter.end_set(key);

    Ok(())
}

pub fn read_quicklist_ziplist<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
) -> RdbOk {
    let ziplist = read_blob(input)?;

    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    for _ in 0..zllen {
        let entry = read_ziplist_entry_string(&mut reader)?;
        formatter.list_element(key, &entry);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(other_error("Invalid end byte of ziplist (quicklist)"));
    }

    Ok(())
}
