use super::common::utils::{other_error, read_blob, read_exact, read_length, read_sequence};
use super::common::{read_ziplist_entry_string, read_ziplist_metadata};
use super::value::RdbValue;
use crate::types::RdbResult;
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashSet;
use std::io::{Cursor, Read};
use std::str;

pub fn read_set<R: Read>(input: &mut R, key: &[u8], expiry: Option<u64>) -> RdbResult<RdbValue> {
    let values = read_sequence(input, |input| read_blob(input))?;
    let members = values.into_iter().collect();

    Ok(RdbValue::Set {
        key: key.to_vec(),
        members,
        expiry,
    })
}

pub fn read_set_intset<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let intset = read_blob(input)?;

    let mut reader = Cursor::new(intset);
    let byte_size = reader.read_u32::<LittleEndian>()?;
    let intset_length = reader.read_u32::<LittleEndian>()?;

    let mut members = HashSet::new();

    for _ in 0..intset_length {
        let val = match byte_size {
            2 => reader.read_i16::<LittleEndian>()? as i64,
            4 => reader.read_i32::<LittleEndian>()? as i64,
            8 => reader.read_i64::<LittleEndian>()?,
            _ => panic!("unhandled byte size in intset: {}", byte_size),
        };

        members.insert(val.to_string().as_bytes().to_vec());
    }

    Ok(RdbValue::Set {
        key: key.to_vec(),
        members,
        expiry,
    })
}

pub fn read_sorted_set<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let mut set_items = read_length(input)?;
    let mut values = Vec::with_capacity(set_items as usize);

    while set_items > 0 {
        let val = read_blob(input)?;
        let score_length = input.read_u8()?;
        let score = match score_length {
            253 => f64::NAN,
            254 => f64::INFINITY,
            255 => f64::NEG_INFINITY,
            _ => {
                let tmp = read_exact(input, score_length as usize)?;
                unsafe { str::from_utf8_unchecked(&tmp) }
                    .parse::<f64>()
                    .unwrap()
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

pub fn read_sortedset_ziplist<R: Read>(
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
        return Err(other_error("Invalid end byte of ziplist"));
    }

    Ok(RdbValue::SortedSet {
        key: key.to_vec(),
        values,
        expiry,
    })
}
