mod version {
    pub const SUPPORTED_MINIMUM : u32 = 0;
    pub const SUPPORTED_MAXIMUM : u32 = 6;
}

mod constants {
    pub const RDB_6BITLEN : u8 = 0;
    pub const RDB_14BITLEN : u8 = 1;
    pub const RDB_32BITLEN : u8 = 2;
    pub const RDB_ENCVAL : u8 = 3;
    // REDIS
    pub const RDB_MAGIC : &'static str = "REDIS";
}

mod op_codes {
    const EXPIRETIME_MS : u8 = 252;
    const EXPIRETIME : u8 = 253;
    const SELECTDB   : u8 = 254;
    const EOF : u8 = 255;
}

mod types {
    const STRING : u8 = 0;
    const LIST : u8 = 1;
    const SET : u8 = 2;
    const ZSET : u8 = 3;
    const HASH : u8 = 4;
    const HASH_ZIPMAP : u8 = 9;
    const LIST_ZIPLIST : u8 = 10;
    const SET_INTSET : u8 = 11;
    const ZSET_ZIPLIST : u8 = 12;
    const HASH_ZIPLIST : u8 = 13;
}

mod encoding {
    pub const INT8 : u32 = 0;
    pub const INT16 : u32 = 1;
    pub const INT32 : u32 = 2;
    pub const LZF : u32 = 3;
}

#[deriving(Copy)]
pub enum DataType {
    String,
    List,
    Set,
    SortedSet,
    Hash,
    Unknown
}

pub fn type_mapping(in_type: u8) -> DataType {
    match in_type {
        0 => DataType::String,
        1 | 10 => DataType::List,
        2 | 11 => DataType::Set,
        3 | 12 => DataType::SortedSet,
        4 | 9 | 13 => DataType::Hash,
        _ => DataType::Unknown
    }
}

fn verify_magic<T: Reader>(input: &mut T) -> bool {
    let magic = input.read_exact(5).unwrap();

    // Meeeeeh.
    magic[0] == constants::RDB_MAGIC.as_bytes()[0] &&
        magic[1] == constants::RDB_MAGIC.as_bytes()[1] &&
        magic[2] == constants::RDB_MAGIC.as_bytes()[2] &&
        magic[3] == constants::RDB_MAGIC.as_bytes()[3] &&
        magic[4] == constants::RDB_MAGIC.as_bytes()[4]
}

fn verify_version<T: Reader>(input: &mut T) -> bool {
    let version = input.read_exact(4).unwrap();

    let version = (version[0]-48) as u32 * 1000 +
        (version[1]-48) as u32 * 100 +
        (version[2]-48) as u32 * 10 +
        (version[3]-48) as u32;

    version > version::SUPPORTED_MINIMUM &&
        version < version::SUPPORTED_MAXIMUM
}

#[test]
fn test_verify_magic() {
    use std::io::MemReader;

    assert_eq!(
        true,
        verify_magic(&mut MemReader::new(vec!(0x52, 0x45, 0x44, 0x49, 0x53)))
    );

    assert_eq!(
        false,
        verify_magic(&mut MemReader::new(vec!(0x52, 0x0, 0x0, 0x0, 0x0)))
    );
}

#[test]
fn test_verify_version() {
    use std::io::MemReader;

    assert_eq!(
        true,
        verify_version(&mut MemReader::new(vec!(0x30, 0x30, 0x30, 0x33)))
    );

    assert_eq!(
        false,
        verify_version(&mut MemReader::new(vec!(0x30, 0x30, 0x30, 0x3a)))
    );
}

#[deriving(Show,PartialEq)]
enum LengthEncoded {
    LE(u32, bool)
    //Encoded(u32),
    //Plain(u32)
}

fn read_length_with_encoding<T: Reader>(input: &mut T) -> LengthEncoded {
    let mut length;
    let mut is_encoded = false;

    let enc_type = input.read_byte().unwrap();

    match (enc_type & 0xC0) >> 6 {
        constants::RDB_ENCVAL => {
            is_encoded = true;
            length = (enc_type & 0x3F) as u32;
        },
        constants::RDB_6BITLEN => {
            length = (enc_type & 0x3F) as u32;
        },
        constants::RDB_14BITLEN => {
            let next_byte = input.read_byte().unwrap();
            length = ((enc_type & 0x3F) as u32 <<8) | next_byte as u32;
        },
        _ => {
            println!("foo");
            length = input.read_le_u32().unwrap();
        }
    }

    LengthEncoded::LE(length, is_encoded)
}

fn read_blob<T: Reader>(input: &mut T) -> Vec<u8> {
    let LengthEncoded::LE(length, is_encoded) = read_length_with_encoding(input);

    let mut val : Vec<u8>;
    if is_encoded {
        val = match length {
            encoding::INT8 => { input.read_exact(1).unwrap() },
            encoding::INT16 => { input.read_exact(2).unwrap() },
            encoding::INT32 => { input.read_exact(4).unwrap() },
            encoding::LZF => { panic!("Encoding not implemented. LZF") },
            _ => { panic!("Unknown encoding: {}", is_encoded) }
        }
    } else {
        val = input.read_exact(length as uint).unwrap();
    }

    val
}

#[test]
fn test_read_length() {
    use std::io::MemReader;

    assert_eq!(
        LengthEncoded::LE(0, false),
        read_length_with_encoding(&mut MemReader::new(vec!(0x0)))
    );

    assert_eq!(
        LengthEncoded::LE(16383, false),
        read_length_with_encoding(&mut MemReader::new(vec!(0x7f, 0xff)))
    );

    assert_eq!(
        LengthEncoded::LE(4294967295, false),
        read_length_with_encoding(&mut MemReader::new(
                vec!(0x80, 0xff, 0xff, 0xff, 0xff)))
    );

    assert_eq!(
        LengthEncoded::LE(0, true),
        read_length_with_encoding(&mut MemReader::new(
                vec!(0xC0)))
    );
}

#[test]
fn test_read_blob() {
    use std::io::MemReader;

    assert_eq!(
        vec!(0x61, 0x62, 0x63, 0x64),
        read_blob(&mut MemReader::new(vec!(
                    4, 0x61, 0x62, 0x63, 0x64))));
}
