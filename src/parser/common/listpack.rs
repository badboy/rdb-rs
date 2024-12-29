use crate::types::{RdbError, RdbResult};
use byteorder::ReadBytesExt;
use std::io::Read;

/// Skip the backlen field in a listpack entry
/// The backlen field is used to traverse the listpack backwards
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

/// Read a single entry from a listpack as a string
/// Format (first 2 bits):
/// 00/01: 7-bit integer
/// 10: string with 6-bit length
/// 11: complex encoding (integers or strings)
pub fn read_list_pack_entry_as_string<R: Read>(reader: &mut R) -> RdbResult<Vec<u8>> {
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
                15 => Err(RdbError::MissingValue("listpack entry")),
                _ => Err(RdbError::ParsingError {
                    context: "read_list_pack_entry_as_string",
                    message: format!("Unknown encoding value: {}", header),
                }),
            },
        },
        _ => unreachable!(),
    }
}
