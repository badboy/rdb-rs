use super::common::utils::{other_error, read_blob, read_exact, read_length};
use super::common::{read_ziplist_entry_string, read_ziplist_metadata};
use crate::formatter::Formatter;
use crate::types::{EncodingType, RdbOk, RdbResult};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

pub fn read_hash<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let mut hash_items = read_length(input)?;

    formatter.start_hash(key, hash_items, last_expiretime, EncodingType::Hashtable);

    while hash_items > 0 {
        let field = read_blob(input)?;
        let val = read_blob(input)?;

        formatter.hash_element(key, &field, &val);

        hash_items -= 1;
    }

    formatter.end_hash(key);

    Ok(())
}

pub fn read_hash_ziplist<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let ziplist = read_blob(input)?;
    let raw_length = ziplist.len() as u64;

    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    assert!(zllen % 2 == 0);
    let zllen = zllen / 2;

    formatter.start_hash(
        key,
        zllen as u32,
        last_expiretime,
        EncodingType::Ziplist(raw_length),
    );

    for _ in 0..zllen {
        let field = read_ziplist_entry_string(&mut reader)?;
        let value = read_ziplist_entry_string(&mut reader)?;
        formatter.hash_element(key, &field, &value);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(other_error("Invalid end byte of ziplist"));
    }

    formatter.end_hash(key);

    Ok(())
}

pub fn read_hash_zipmap<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let zipmap = read_blob(input)?;
    let raw_length = zipmap.len() as u64;

    let mut reader = Cursor::new(zipmap);

    let zmlen = reader.read_u8()?;

    let mut length: i32;
    let size;
    if zmlen <= 254 {
        length = zmlen as i32;
        size = zmlen
    } else {
        length = -1;
        size = 0;
    }

    formatter.start_hash(
        key,
        size as u32,
        last_expiretime,
        EncodingType::Zipmap(raw_length),
    );

    loop {
        let next_byte = reader.read_u8()?;

        if next_byte == 0xFF {
            break; // End of list.
        }

        let field = read_zipmap_entry(next_byte, &mut reader)?;

        let next_byte = reader.read_u8()?;
        let _free = reader.read_u8()?;
        let value = read_zipmap_entry(next_byte, &mut reader)?;

        formatter.hash_element(key, &field, &value);

        if length > 0 {
            length -= 1;
        }

        if length == 0 {
            let last_byte = reader.read_u8()?;

            if last_byte != 0xFF {
                return Err(other_error("Invalid end byte of zipmap"));
            }
            break;
        }
    }

    formatter.end_hash(key);

    Ok(())
}

fn read_zipmap_entry<T: Read>(next_byte: u8, zipmap: &mut T) -> RdbResult<Vec<u8>> {
    let elem_len;
    match next_byte {
        253 => elem_len = zipmap.read_u32::<LittleEndian>().unwrap(),
        254 | 255 => {
            panic!("Invalid length value in zipmap: {}", next_byte)
        }
        _ => elem_len = next_byte as u32,
    }

    read_exact(zipmap, elem_len as usize)
}
