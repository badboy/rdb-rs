extern crate lzf;
use std::str;
use lzf::decompress;

mod version {
    pub const SUPPORTED_MINIMUM : u32 = 1;
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
    pub const EXPIRETIME_MS : u8 = 252;
    pub const EXPIRETIME : u8 = 253;
    pub const SELECTDB   : u8 = 254;
    pub const EOF : u8 = 255;
}

mod types {
    pub const STRING : u8 = 0;
    pub const LIST : u8 = 1;
    pub const SET : u8 = 2;
    pub const ZSET : u8 = 3;
    pub const HASH : u8 = 4;
    pub const HASH_ZIPMAP : u8 = 9;
    pub const LIST_ZIPLIST : u8 = 10;
    pub const SET_INTSET : u8 = 11;
    pub const ZSET_ZIPLIST : u8 = 12;
    pub const HASH_ZIPLIST : u8 = 13;
}

mod encoding {
    pub const INT8 : u32 = 0;
    pub const INT16 : u32 = 1;
    pub const INT32 : u32 = 2;
    pub const LZF : u32 = 3;
}

#[deriving(Show)]
pub enum DataType {
    String(Vec<u8>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    SortedSet(Vec<(f32,Vec<u8>)>),
    Hash(Vec<Vec<u8>>),
    Unknown
}

//pub fn type_mapping(in_type: u8) -> DataType {
    //match in_type {
        //0 => DataType::String,
        //1 | 10 => DataType::List,
        //2 | 11 => DataType::Set,
        //3 | 12 => DataType::SortedSet,
        //4 | 9 | 13 => DataType::Hash,
        //_ => DataType::Unknown
    //}
//}

pub fn parse<T: Reader>(input: &mut T) {

    assert!(verify_magic(input));
    assert!(verify_version(input));

    loop {
        let next_op = input.read_byte().unwrap();
        //println!("next_op: {:x}", next_op);

        match next_op {
            op_codes::SELECTDB => {
                let db = input.read_byte().unwrap();
                println!("SELECTDB: {}", db);
            },
            op_codes::EOF => {
                println!("EOF");
                break ;
            },
            op_codes::EXPIRETIME_MS => {
                let expiretime_ms = input.read_le_u64().unwrap();
                println!("EXPIRETIME_MS: {}", expiretime_ms);
            },
            op_codes::EXPIRETIME => {
                let expiretime = input.read_be_u32().unwrap();
                println!("EXPIRETIME: {}", expiretime);
            }
            _ => {
                let key = read_blob(input);
                println!("Key: {}",
                         str::from_utf8(key.as_slice()).unwrap());
                println!("Val: {}", read_type(next_op, input));
                break;
            }
        }

    }

    return;

}

fn read_type<T: Reader>(value_type: u8, input: &mut T) -> DataType {
    match value_type {
        types::STRING => {
            DataType::String(read_blob(input))
        },
        types::LIST => {
            //DataType::List(read_linked_list(input))

            let mut len = read_length(input);
            let mut list = vec![];

            while len > 0 {
                let blob = read_blob(input);
                list.push(blob);
                len -= 1;
            }

            DataType::List(list)
        },
        types::SET => {
            println!("Value Type not implemented: {}", "SET");

            //DataType::Set(read_set(input))
            DataType::Set(vec![])
        },
        types::ZSET => {
            println!("Value Type not implemented: {}", "ZSET");
            //DataType::SortedSet(read_sorted_set(input))
            DataType::SortedSet(vec![])
        },
        types::HASH => {
            println!("Value Type not implemented: {}", "HASH");
            //DataType::Hash(read_hash(input))
            DataType::Hash(vec![])
        },
        types::HASH_ZIPMAP => {
            println!("Value Type not implemented: {}", "HASH_ZIPMAP");
            //DataType::Hash(read_hash_zipmap(input))
            DataType::Hash(vec![])
        },
        types::LIST_ZIPLIST => {
            println!("Value Type not implemented: {}", "LIST_ZIPLIST");
            //DataType::List(read_list_ziplist(input))
            DataType::List(vec![])
        },
        types::SET_INTSET => {
            println!("Value Type not implemented: {}", "SET_INTSET");
            //DataType::Set(read_set_intset(input))
            DataType::Set(vec![])
        },
        types::ZSET_ZIPLIST => {
            println!("Value Type not implemented: {}", "ZSET_ZIPLIST");
            //DataType::SortedSet(read_zset_ziplist(input))
            DataType::SortedSet(vec![])
        },
        types::HASH_ZIPLIST => {
            println!("Value Type not implemented: {}", "HASH_ZIPLIST");
            //DataType::Hash(read_hash_ziplist(input))
            DataType::Hash(vec![])
        },
        _ => { panic!("Value Type not implemented: {}", value_type) }
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

    version >= version::SUPPORTED_MINIMUM &&
        version <= version::SUPPORTED_MAXIMUM
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

fn read_length<T: Reader>(input: &mut T) -> u32 {
    let LengthEncoded::LE(length, _) = read_length_with_encoding(input);
    length
}

fn read_blob<T: Reader>(input: &mut T) -> Vec<u8> {
    let LengthEncoded::LE(length, is_encoded) = read_length_with_encoding(input);

    if is_encoded {
        match length {
            encoding::INT8 => { input.read_exact(1).unwrap() },
            encoding::INT16 => { input.read_exact(2).unwrap() },
            encoding::INT32 => { input.read_exact(4).unwrap() },
            encoding::LZF => {
                let compressed_length = read_length(input);
                let real_length = read_length(input);
                let data = input.read_exact(compressed_length as uint).unwrap();
                lzf::decompress(data.as_slice(), real_length as uint).unwrap()
            },
            _ => { panic!("Unknown encoding: {}", length) }
        }
    } else {
        input.read_exact(length as uint).unwrap()
    }
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
