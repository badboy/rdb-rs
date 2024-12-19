use super::common::utils::{other_error, read_blob, read_exact, read_length};
use super::common::{read_ziplist_entry_string, read_ziplist_metadata};
use crate::formatter::Formatter;
use crate::types::{EncodingType, RdbOk};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};
use std::str;

pub fn read_set_intset<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let intset = read_blob(input)?;
    let raw_length = intset.len() as u64;

    let mut reader = Cursor::new(intset);
    let byte_size = reader.read_u32::<LittleEndian>()?;
    let intset_length = reader.read_u32::<LittleEndian>()?;

    formatter.start_set(
        key,
        intset_length,
        last_expiretime,
        EncodingType::Intset(raw_length),
    );

    for _ in 0..intset_length {
        let val = match byte_size {
            2 => reader.read_i16::<LittleEndian>()? as i64,
            4 => reader.read_i32::<LittleEndian>()? as i64,
            8 => reader.read_i64::<LittleEndian>()?,
            _ => panic!("unhandled byte size in intset: {}", byte_size),
        };

        formatter.set_element(key, val.to_string().as_bytes());
    }

    formatter.end_set(key);

    Ok(())
}

pub fn read_sorted_set<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let mut set_items = read_length(input)?;

    formatter.start_sorted_set(key, set_items, last_expiretime, EncodingType::Hashtable);

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

        formatter.sorted_set_element(key, score, &val);

        set_items -= 1;
    }

    formatter.end_sorted_set(key);

    Ok(())
}

pub fn read_sortedset_ziplist<R: Read, F: Formatter>(
    input: &mut R,
    formatter: &mut F,
    key: &[u8],
    last_expiretime: Option<u64>,
) -> RdbOk {
    let ziplist = read_blob(input)?;
    let raw_length = ziplist.len() as u64;

    let mut reader = Cursor::new(ziplist);
    let (_zlbytes, _zltail, zllen) = read_ziplist_metadata(&mut reader)?;

    formatter.start_sorted_set(
        key,
        zllen as u32,
        last_expiretime,
        EncodingType::Ziplist(raw_length),
    );

    assert!(zllen % 2 == 0);
    let zllen = zllen / 2;

    for _ in 0..zllen {
        let entry = read_ziplist_entry_string(&mut reader)?;
        let score = read_ziplist_entry_string(&mut reader)?;
        let score = str::from_utf8(&score).unwrap().parse::<f64>().unwrap();
        formatter.sorted_set_element(key, score, &entry);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(other_error("Invalid end byte of ziplist"));
    }

    formatter.end_sorted_set(key);

    Ok(())
}
