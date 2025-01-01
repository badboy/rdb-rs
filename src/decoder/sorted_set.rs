use super::common::utils::{read_blob, read_exact, read_length};
use super::common::{read_list_pack_length, read_ziplist_entry_string, read_ziplist_metadata};
use crate::decoder::common::read_list_pack_entry_as_string;
use crate::types::{RdbError, RdbResult, RdbValue};
use byteorder::ReadBytesExt;
use std::io::{Cursor, Read};
use std::str;

pub fn read_sorted_set<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
    is_zset2: bool,
) -> RdbResult<RdbValue> {
    let mut set_items = read_length(input)?;
    let mut values = Vec::with_capacity(set_items as usize);

    while set_items > 0 {
        let val = read_blob(input)?;

        let score = if is_zset2 {
            // ZSET2 format uses binary encoding of float64
            input.read_f64::<byteorder::LittleEndian>()?
        } else {
            // Original format uses string representation
            let score_length = input.read_u8()?;
            match score_length {
                253 => f64::NAN,
                254 => f64::INFINITY,
                255 => f64::NEG_INFINITY,
                _ => {
                    let tmp = read_exact(input, score_length as usize)?;
                    unsafe { str::from_utf8_unchecked(&tmp) }
                        .parse::<f64>()
                        .unwrap()
                }
            }
        };

        values.push((score, val));
        set_items -= 1;
    }

    Ok(RdbValue::SortedSet {
        key: key.to_vec(),
        values,
        expiry,
    })
}

pub fn read_sorted_set_ziplist<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let ziplist = read_blob(input)?;
    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    assert!(zllen % 2 == 0);
    let zllen = zllen / 2;
    let mut values = Vec::with_capacity(zllen as usize);

    for _ in 0..zllen {
        let entry = read_ziplist_entry_string(&mut reader)?;
        let score = read_ziplist_entry_string(&mut reader)?;
        let score = str::from_utf8(&score).unwrap().parse::<f64>().unwrap();
        values.push((score, entry));
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(RdbError::ParsingError {
            context: "read_sortedset_ziplist",
            message: format!("Unknown encoding value: {}", last_byte),
        });
    }

    Ok(RdbValue::SortedSet {
        key: key.to_vec(),
        values,
        expiry,
    })
}

pub fn read_sorted_set_listpack<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let listpack = read_blob(input)?;
    let mut reader = Cursor::new(&listpack);
    let mut values = Vec::new();

    // Read number of elements (size)
    let buf = reader.get_ref();
    let mut cursor = 0;
    let size = read_list_pack_length(buf, &mut cursor);
    reader.set_position(cursor as u64);

    assert!(size % 2 == 0);
    let num_entries = size / 2;

    for _ in 0..num_entries {
        let member = read_list_pack_entry_as_string(&mut reader)?;
        let score_str = read_list_pack_entry_as_string(&mut reader)?;

        let score = unsafe { str::from_utf8_unchecked(&score_str) }
            .parse::<f64>()
            .map_err(|_| RdbError::ParsingError {
                context: "read_sorted_set_listpack",
                message: format!(
                    "Failed to parse score: {:?}",
                    String::from_utf8_lossy(&score_str)
                ),
            })?;

        values.push((score, member));
    }

    Ok(RdbValue::SortedSet {
        key: key.to_vec(),
        values,
        expiry,
    })
}
