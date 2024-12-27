use super::common::utils::{read_blob, read_length, read_sequence};
use super::common::{read_ziplist_entry_string, read_ziplist_metadata};
use crate::types::{RdbError, RdbResult, RdbValue};
use byteorder::ReadBytesExt;
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
        return Err(RdbError::UnknownEncoding(last_byte));
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
        return Err(RdbError::UnknownEncoding(last_byte));
    }

    Ok(values)
}
