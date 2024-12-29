use super::common::utils::{read_blob, read_length, read_sequence};
use super::common::{
    read_list_pack_entry_as_string, read_ziplist_entry_string, read_ziplist_metadata,
};
use crate::types::{RdbError, RdbResult, RdbValue};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

pub fn read_linked_list<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let values = read_sequence(input, |input| read_blob(input))?;

    Ok(RdbValue::List {
        key: key.to_vec(),
        values,
        expiry,
    })
}

pub fn read_list_ziplist<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let ziplist = read_blob(input)?;
    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    let mut values = Vec::with_capacity(zllen as usize);

    for _ in 0..zllen {
        let entry = read_ziplist_entry_string(&mut reader)?;
        values.push(entry);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(RdbError::ParsingError {
            context: "read_list_ziplist",
            message: format!("Unknown encoding value: {}", last_byte),
        });
    }

    Ok(RdbValue::List {
        key: key.to_vec(),
        values,
        expiry,
    })
}

pub fn read_quicklist<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let len = read_length(input)?;
    let mut values = Vec::new();

    for _ in 0..len {
        let mut ziplist_values = read_quicklist_ziplist(input, key)?;
        values.append(&mut ziplist_values);
    }

    Ok(RdbValue::List {
        key: key.to_vec(),
        values,
        expiry,
    })
}

pub fn read_quicklist_2<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let len = read_length(input)?;
    let mut values = Vec::new();

    for _ in 0..len {
        let container_type = read_length(input)?;
        match container_type {
            1 => {
                // QUICKLIST_NODE_CONTAINER_PLAIN
                let entry = read_blob(input)?;
                values.push(entry);
            }
            2 => {
                // QUICKLIST_NODE_CONTAINER_PACKED
                let mut listpack_values = read_quicklist_listpack(input)?;
                values.append(&mut listpack_values);
            }
            _ => {
                return Err(RdbError::ParsingError {
                    context: "read_quicklist_2",
                    message: format!("Unknown encoding value: {}", container_type),
                })
            }
        }
    }

    Ok(RdbValue::List {
        key: key.to_vec(),
        values,
        expiry,
    })
}

fn read_quicklist_ziplist<R: Read>(input: &mut R, _key: &[u8]) -> RdbResult<Vec<Vec<u8>>> {
    let ziplist = read_blob(input)?;
    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    let mut values = Vec::with_capacity(zllen as usize);

    for _ in 0..zllen {
        let entry = read_ziplist_entry_string(&mut reader)?;
        values.push(entry);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(RdbError::ParsingError {
            context: "read_quicklist_ziplist",
            message: format!("Unknown encoding value: {}", last_byte),
        });
    }

    Ok(values)
}

fn read_quicklist_listpack<R: Read>(input: &mut R) -> RdbResult<Vec<Vec<u8>>> {
    let listpack = read_blob(input)?;
    let mut reader = Cursor::new(listpack);
    let total_bytes = reader.read_u32::<LittleEndian>()?;
    let num_elements = reader.read_u16::<LittleEndian>()?;

    let mut values = Vec::with_capacity(num_elements as usize);

    // Read until we reach the end of the listpack
    while reader.position() < total_bytes as u64 - 1 {
        let entry = read_list_pack_entry_as_string(&mut reader)?;
        values.push(entry);
    }

    // Verify end byte
    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(RdbError::ParsingError {
            context: "read_quicklist_listpack",
            message: format!("Unknown encoding value: {}", last_byte),
        });
    }

    Ok(values)
}
