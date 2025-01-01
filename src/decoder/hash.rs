use super::common::utils::{read_blob, read_exact, read_length};
use super::common::{
    read_list_pack_entry_as_string, read_list_pack_length, read_ziplist_entry_string,
    read_ziplist_metadata,
};
use crate::types::{RdbError, RdbResult, RdbValue};
use byteorder::{LittleEndian, ReadBytesExt};
use indexmap::IndexMap;
use std::io::{Cursor, Read};

pub fn read_hash<R: Read>(input: &mut R, key: &[u8], expiry: Option<u64>) -> RdbResult<RdbValue> {
    let mut hash_items = read_length(input)?;
    let mut values = IndexMap::new();

    while hash_items > 0 {
        let field = read_blob(input)?;
        let val = read_blob(input)?;
        values.insert(field, val);
        hash_items -= 1;
    }

    Ok(RdbValue::Hash {
        key: key.to_vec(),
        values,
        expiry,
    })
}

pub fn read_hash_ziplist<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let ziplist = read_blob(input)?;
    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    assert!(zllen % 2 == 0);
    let zllen = zllen / 2;

    let mut values = IndexMap::new();

    for _ in 0..zllen {
        let field = read_ziplist_entry_string(&mut reader)?;
        let value = read_ziplist_entry_string(&mut reader)?;
        values.insert(field, value);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(RdbError::ParsingError {
            context: "read_hash_ziplist",
            message: format!("Unknown encoding value: {}", last_byte),
        });
    }

    Ok(RdbValue::Hash {
        key: key.to_vec(),
        values,
        expiry,
    })
}

pub fn read_hash_zipmap<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let zipmap = read_blob(input)?;
    let mut reader = Cursor::new(zipmap);

    let zmlen = reader.read_u8()?;

    let mut length: i32;
    if zmlen <= 254 {
        length = zmlen as i32;
    } else {
        length = -1;
    }

    let mut values = IndexMap::new();

    loop {
        let next_byte = reader.read_u8()?;

        if next_byte == 0xFF {
            break; // End of list.
        }

        let field = read_zipmap_entry(next_byte, &mut reader)?;

        let next_byte = reader.read_u8()?;
        let _free = reader.read_u8()?;
        let value = read_zipmap_entry(next_byte, &mut reader)?;

        values.insert(field, value);

        if length > 0 {
            length -= 1;
        }

        if length == 0 {
            let last_byte = reader.read_u8()?;

            if last_byte != 0xFF {
                return Err(RdbError::ParsingError {
                    context: "read_hash_zipmap",
                    message: format!("Unknown encoding value: {}", last_byte),
                });
            }
            break;
        }
    }

    Ok(RdbValue::Hash {
        key: key.to_vec(),
        values,
        expiry,
    })
}

fn read_zipmap_entry<T: Read>(next_byte: u8, zipmap: &mut T) -> RdbResult<Vec<u8>> {
    let elem_len;
    match next_byte {
        253 => elem_len = zipmap.read_u32::<LittleEndian>().unwrap(),
        254 | 255 => {
            return Err(RdbError::ParsingError {
                context: "read_zipmap_entry",
                message: format!("Unknown encoding value: {}", next_byte),
            });
        }
        _ => elem_len = next_byte as u32,
    }

    read_exact(zipmap, elem_len as usize)
}

pub fn read_hash_list_pack<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let listpack = read_blob(input)?;
    let mut cursor = 0;
    let size = read_list_pack_length(&listpack, &mut cursor);

    let mut values = IndexMap::new();
    let mut reader = Cursor::new(listpack);
    reader.set_position(cursor as u64);

    for _ in 0..size / 2 {
        let field = read_list_pack_entry_as_string(&mut reader)?;
        let value = read_list_pack_entry_as_string(&mut reader)?;
        values.insert(field, value);
    }

    Ok(RdbValue::Hash {
        key: key.to_vec(),
        values,
        expiry,
    })
}
