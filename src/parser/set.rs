use super::common::utils::{read_blob, read_sequence};
use super::common::read_list_pack_entry_as_string;
use crate::types::{RdbError, RdbResult, RdbValue};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

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

    let mut members = Vec::with_capacity(intset_length as usize);

    for _ in 0..intset_length {
        let val = match byte_size {
            2 => reader.read_i16::<LittleEndian>()? as i64,
            4 => reader.read_i32::<LittleEndian>()? as i64,
            8 => reader.read_i64::<LittleEndian>()?,
            _ => panic!("unhandled byte size in intset: {}", byte_size),
        };

        members.push(val.to_string().as_bytes().to_vec());
    }

    Ok(RdbValue::Set {
        key: key.to_vec(),
        members: members.into_iter().collect(),
        expiry,
    })
}

pub fn read_set_list_pack<R: Read>(
    input: &mut R,
    key: &[u8],
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let listpack = read_blob(input)?;
    let mut reader = Cursor::new(listpack);

    // Read total bytes and number of elements
    let total_bytes = reader.read_u32::<LittleEndian>()?;
    let num_elements = reader.read_u16::<LittleEndian>()?;

    let mut members = Vec::with_capacity(num_elements as usize);

    // Read until we reach the end of the listpack
    while reader.position() < total_bytes as u64 - 1 {
        let entry = read_list_pack_entry_as_string(&mut reader)?;
        members.push(entry);
    }

    // Verify end byte
    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(RdbError::ParsingError {
            context: "read_set_list_pack",
            message: format!("Unknown encoding value: {}", last_byte),
        });
    }

    Ok(RdbValue::Set {
        key: key.to_vec(),
        members: members.into_iter().collect(),
        expiry,
    })
}
