use std::io::Read;

use super::utils::read_exact;
use crate::types::{RdbError, RdbResult};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

#[derive(Debug, Clone)]
pub enum ZiplistEntry {
    String(Vec<u8>),
    Number(i64),
}

pub fn read_ziplist_metadata<T: Read>(input: &mut T) -> RdbResult<(u32, u32, u16)> {
    let zlbytes = input.read_u32::<LittleEndian>()?;
    let zltail = input.read_u32::<LittleEndian>()?;
    let zllen = input.read_u16::<LittleEndian>()?;

    Ok((zlbytes, zltail, zllen))
}

pub fn read_ziplist_entry_string<R: Read>(input: &mut R) -> RdbResult<Vec<u8>> {
    let entry = read_ziplist_entry(input)?;
    match entry {
        ZiplistEntry::String(val) => Ok(val),
        ZiplistEntry::Number(val) => Ok(val.to_string().into_bytes()),
    }
}

fn read_ziplist_entry<R: Read>(input: &mut R) -> RdbResult<ZiplistEntry> {
    // 1. 1 or 5 bytes length of previous entry
    let byte = input.read_u8()?;
    if byte == 254 {
        let mut bytes = [0; 4];
        match input.read(&mut bytes) {
            Ok(4) => (),
            Ok(_) => {
                return Err(RdbError::MissingValue(
                    "4 bytes to skip after ziplist length",
                ))
            }
            Err(e) => return Err(RdbError::Io(e)),
        };
    }

    let length: u64;
    let number_value: i64;

    // 2. Read flag or number value
    let flag = input.read_u8()?;

    match (flag & 0xC0) >> 6 {
        0 => length = (flag & 0x3F) as u64,
        1 => {
            let next_byte = input.read_u8()?;
            length = (((flag & 0x3F) as u64) << 8) | next_byte as u64;
        }
        2 => {
            length = input.read_u32::<BigEndian>()? as u64;
        }
        _ => {
            match (flag & 0xF0) >> 4 {
                0xC => number_value = input.read_i16::<LittleEndian>()? as i64,
                0xD => number_value = input.read_i32::<LittleEndian>()? as i64,
                0xE => number_value = input.read_i64::<LittleEndian>()? as i64,
                0xF => match flag & 0xF {
                    0 => {
                        let mut bytes = [0; 3];
                        match input.read(&mut bytes) {
                            Ok(3) => (),
                            Ok(_) => {
                                return Err(RdbError::MissingValue(
                                    "24bit number",
                                ))
                            }
                            Err(e) => return Err(RdbError::Io(e)),
                        };

                        let number: i32 = (((bytes[2] as i32) << 24)
                            ^ ((bytes[1] as i32) << 16)
                            ^ ((bytes[0] as i32) << 8)
                            ^ 48)
                            >> 8;

                        number_value = number as i64;
                    }
                    0xE => {
                        number_value = input.read_i8()? as i64;
                    }
                    _ => {
                        number_value = (flag & 0xF) as i64 - 1;
                    }
                },
                _ => {
                    return Err(RdbError::UnknownEncoding(flag));
                }
            }

            return Ok(ZiplistEntry::Number(number_value));
        }
    }

    // 3. Read value
    let rawval = read_exact(input, length as usize)?;
    Ok(ZiplistEntry::String(rawval))
}
