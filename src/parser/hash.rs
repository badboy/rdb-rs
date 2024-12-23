use super::common::utils::{other_error, read_blob, read_exact, read_length};
use super::common::{read_ziplist_entry_string, read_ziplist_metadata};
use crate::types::{RdbOk, RdbResult, RdbValue};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::{Cursor, Read};

pub fn read_hash<R: Read>(input: &mut R, key: &[u8], expiry: Option<u64>) -> RdbResult<RdbValue> {
    let mut hash_items = read_length(input)?;
    let mut values = HashMap::new();

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

    let mut values = HashMap::new();

    for _ in 0..zllen {
        let field = read_ziplist_entry_string(&mut reader)?;
        let value = read_ziplist_entry_string(&mut reader)?;
        values.insert(field, value);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(other_error("Invalid end byte of ziplist"));
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
    let size;
    if zmlen <= 254 {
        length = zmlen as i32;
        size = zmlen
    } else {
        length = -1;
        size = 0;
    }

    let mut values = HashMap::new();

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
                return Err(other_error("Invalid end byte of zipmap"));
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
            panic!("Invalid length value in zipmap: {}", next_byte)
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
    cursor += 4;
    let size = u16::from_le_bytes(listpack[cursor..cursor + 2].try_into().unwrap()) as u32;
    cursor += 2;

    assert!(size % 2 == 0);
    let num_pairs = size / 2;

    let mut values = HashMap::new();

    let mut reader = Cursor::new(listpack);
    reader.set_position(cursor as u64);

    for _ in 0..num_pairs {
        let field = read_list_pack_entry_as_string(&mut reader)?;
        let value = read_list_pack_entry_as_string(&mut reader)?;

        values.insert(field, value);
    }

    let last_byte = reader.read_u8()?;
    if last_byte != 0xFF {
        return Err(other_error("Invalid end byte of listpack"));
    }

    Ok(RdbValue::Hash {
        key: key.to_vec(),
        values,
        expiry,
    })
}

fn read_list_pack_entry_as_string<R: Read>(reader: &mut R) -> RdbResult<Vec<u8>> {
    let header = reader.read_u8()?;

    match header >> 6 {
        0 | 1 => {
            let val = (header & 0x7F) as i8;
            Ok(val.to_string().into_bytes())
        }
        2 => {
            let str_len = (header & 0x3F) as usize;
            let mut result = vec![0; str_len];
            reader.read_exact(&mut result)?;

            let content_len = 1 + str_len;
            skip_backlen(reader, content_len as u32)?;

            Ok(result)
        }
        3 => match header >> 4 {
            12 | 13 => {
                let next = reader.read_u8()?;
                let mut val = (((header & 0x1F) as u16) << 8) | (next as u16);
                if val >= 1 << 12 {
                    val = !(8191 - val);
                }
                skip_backlen(reader, 2)?;
                Ok(val.to_string().into_bytes())
            }
            14 => {
                let len_high = (header & 0x0F) as u16;
                let len_low = reader.read_u8()? as u16;
                let str_len = ((len_high << 8) | len_low) as usize;

                let mut result = vec![0; str_len];
                reader.read_exact(&mut result)?;

                skip_backlen(reader, (2 + str_len) as u32)?;
                Ok(result)
            }
            _ => match header & 0x0F {
                0 => {
                    let mut len_bytes = [0u8; 4];
                    reader.read_exact(&mut len_bytes)?;
                    let str_len = u32::from_le_bytes(len_bytes) as usize;

                    let mut result = vec![0; str_len];
                    reader.read_exact(&mut result)?;

                    skip_backlen(reader, (5 + str_len) as u32)?;
                    Ok(result)
                }
                1..=4 => {
                    let size = match header & 0x0F {
                        1 => 2,
                        2 => 3,
                        3 => 4,
                        4 => 8,
                        _ => unreachable!(),
                    };
                    let mut int_bytes = vec![0; size];
                    reader.read_exact(&mut int_bytes)?;

                    let val = match size {
                        2 => i16::from_le_bytes(int_bytes.try_into().unwrap()) as i64,
                        3 => {
                            let mut bytes = [0u8; 4];
                            bytes[..3].copy_from_slice(&int_bytes);
                            i32::from_le_bytes(bytes) as i64 >> 8
                        }
                        4 => i32::from_le_bytes(int_bytes.try_into().unwrap()) as i64,
                        8 => i64::from_le_bytes(int_bytes.try_into().unwrap()),
                        _ => unreachable!(),
                    };

                    skip_backlen(reader, (size + 1) as u32)?;
                    Ok(val.to_string().into_bytes())
                }
                15 => Err(other_error("Unexpected end of listpack entry")),
                _ => Err(other_error("Unknown listpack entry header")),
            },
        },
        _ => unreachable!(),
    }
}

fn skip_backlen<R: Read>(reader: &mut R, element_len: u32) -> RdbResult<()> {
    let backlen = if element_len <= 127 {
        1
    } else if element_len < (1 << 14) - 1 {
        2
    } else if element_len < (1 << 21) - 1 {
        3
    } else if element_len < (1 << 28) - 1 {
        4
    } else {
        5
    };

    let mut buf = vec![0; backlen];
    reader.read_exact(&mut buf)?;
    Ok(())
}
