use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use lzf;
use std::io::Read;
use crate::types::RdbError;

#[doc(hidden)]
use crate::constants::{constant, encoding, version};

#[doc(hidden)]
pub use crate::types::{RdbOk, RdbResult};

pub fn read_length_with_encoding<R: Read>(input: &mut R) -> RdbResult<(u32, bool)> {
    let length;
    let mut is_encoded = false;

    let enc_type = input.read_u8()?;

    match (enc_type & 0xC0) >> 6 {
        constant::RDB_ENCVAL => {
            is_encoded = true;
            length = (enc_type & 0x3F) as u32;
        }
        constant::RDB_6BITLEN => {
            length = (enc_type & 0x3F) as u32;
        }
        constant::RDB_14BITLEN => {
            let next_byte = input.read_u8()?;
            length = (((enc_type & 0x3F) as u32) << 8) | next_byte as u32;
        }
        _ => {
            length = input.read_u32::<BigEndian>()?;
        }
    }

    Ok((length, is_encoded))
}

pub fn read_length<R: Read>(input: &mut R) -> RdbResult<u32> {
    let (length, _) = read_length_with_encoding(input)?;
    Ok(length)
}

pub fn verify_magic<R: Read>(input: &mut R) -> RdbOk {
    let mut magic = [0; 5];
    match input.read(&mut magic) {
        Ok(5) => (),
        Ok(_) => return Err(RdbError::MissingValue("magic bytes")),
        Err(e) => return Err(RdbError::Io(e)),
    };

    if magic == constant::RDB_MAGIC.as_bytes() {
        Ok(())
    } else {
        Err(RdbError::MissingValue("invalid magic string"))
    }
}

pub fn verify_version<R: Read>(input: &mut R) -> RdbOk {
    let mut buf = [0u8; 4];
    input.read_exact(&mut buf)?;

    // Check if all characters are ASCII digits
    for &byte in &buf {
        if !byte.is_ascii_digit() {
            return Err(RdbError::MissingValue("invalid version number"));
        }
    }

    // Convert ASCII string to number (e.g., "0003" -> 3)
    let version_str = std::str::from_utf8(&buf).unwrap();
    let version = version_str.parse::<u32>().unwrap();

    // Check if version is in supported range
    let is_ok = version >= version::SUPPORTED_MINIMUM && version <= version::SUPPORTED_MAXIMUM;

    if !is_ok {
        return Err(RdbError::MissingValue("unsupported version"));
    }

    Ok(())
}

pub fn read_blob<R: Read>(input: &mut R) -> RdbResult<Vec<u8>> {
    let (length, is_encoded) = read_length_with_encoding(input)?;

    if is_encoded {
        let result = match length {
            encoding::INT8 => int_to_vec(i32::from(input.read_i8()?)),
            encoding::INT16 => int_to_vec(i32::from(input.read_i16::<LittleEndian>()?)),
            encoding::INT32 => int_to_vec(input.read_i32::<LittleEndian>()?),
            encoding::LZF => {
                let compressed_length = read_length(input)?;
                let real_length = read_length(input)?;
                let data = read_exact(input, compressed_length as usize)?;
                lzf::decompress(&data, real_length as usize).unwrap()
            }
            _ => {
                panic!("Unknown encoding: {}", length)
            }
        };

        Ok(result)
    } else {
        read_exact(input, length as usize)
    }
}

pub fn int_to_vec(number: i32) -> Vec<u8> {
    let number = number.to_string();
    let mut result = Vec::with_capacity(number.len());
    for &c in number.as_bytes().iter() {
        result.push(c);
    }
    result
}

pub fn read_exact<T: Read>(reader: &mut T, len: usize) -> RdbResult<Vec<u8>> {
    let mut buf = vec![0; len];
    reader.read_exact(&mut buf)?;

    Ok(buf)
}

pub fn read_sequence<R: Read, T, F>(input: &mut R, mut transform: F) -> RdbResult<Vec<T>>
where
    F: FnMut(&mut R) -> RdbResult<T>,
{
    let mut len = read_length(input)?;
    let mut values = Vec::with_capacity(len as usize);

    while len > 0 {
        values.push(transform(input)?);
        len -= 1;
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::io::Cursor;

    #[rstest]
    #[case(&[0x0], (0, false), 1)]
    #[case(&[0x7f, 0xff], (16383, false), 2)]
    #[case(&[0x80, 0xff, 0xff, 0xff, 0xff], (4294967295, false), 5)]
    #[case(&[0xC0], (0, true), 1)]
    fn test_read_length(
        #[case] input: &[u8],
        #[case] expected: (u32, bool),
        #[case] expected_position: u64,
    ) {
        let mut cursor = Cursor::new(Vec::from(input));
        assert_eq!(expected, read_length_with_encoding(&mut cursor).unwrap());
        assert_eq!(expected_position, cursor.position());
    }

    #[test]
    fn test_read_blob() {
        assert_eq!(
            vec![0x61, 0x62, 0x63, 0x64],
            read_blob(&mut Cursor::new(vec![4, 0x61, 0x62, 0x63, 0x64])).unwrap()
        );
    }

    #[test]
    fn test_verify_version() {
        // Valid version "0003" should succeed
        assert_eq!(
            (),
            verify_version(&mut Cursor::new(vec![0x30, 0x30, 0x30, 0x33])).unwrap()
        );

        // Invalid version "000:" should fail
        let result = verify_version(&mut Cursor::new(vec![0x30, 0x30, 0x30, 0x3a]));
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_magic() {
        assert_eq!(
            (),
            verify_magic(&mut Cursor::new(vec![0x52, 0x45, 0x44, 0x49, 0x53])).unwrap()
        );

        match verify_magic(&mut Cursor::new(vec![0x51, 0x0, 0x0, 0x0, 0x0])) {
            Ok(_) => assert!(false),
            Err(_) => assert!(true),
        }
    }
}
