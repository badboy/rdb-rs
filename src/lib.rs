#![feature(slicing_syntax)]
extern crate lzf;
use std::str;
use lzf::decompress;
use std::io::MemReader;
use std::io;

mod version {
    pub const SUPPORTED_MINIMUM : u32 = 1;
    pub const SUPPORTED_MAXIMUM : u32 = 6;
}

mod constants {
    pub const RDB_6BITLEN : u8 = 0;
    pub const RDB_14BITLEN : u8 = 1;
    pub const RDB_32BITLEN : u8 = 2;
    pub const RDB_ENCVAL : u8 = 3;
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

#[derive(Show)]
pub enum DataType {
    String(Vec<u8>),
    Number(i64),
    ListOfTypes(Vec<DataType>),
    HashOfTypes(Vec<DataType>),
    SortedSetOfTypes(Vec<DataType>),
    Intset(Vec<i64>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    SortedSet(Vec<(f64,Vec<u8>)>),
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

    let mut out = io::stdout();
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
                let _ = out.write(key[]);
                let _ = out.write_str(": ");

                match read_type(next_op, input) {
                    DataType::String(t) => {
                        let _ = out.write(t[]);
                    },
                    _ => {  }
                }
                let _ = out.write_str("\n");
            }
        }

    }

    return;

}

fn read_linked_list<T: Reader>(input: &mut T) -> Vec<Vec<u8>> {
    let mut len = read_length(input);
    let mut list = vec![];

    while len > 0 {
        let blob = read_blob(input);
        list.push(blob);
        len -= 1;
    }

    list
}

fn read_sorted_set<T: Reader>(input: &mut T) -> Vec<(f64,Vec<u8>)> {
    let mut set = vec![];
    let mut set_items = read_length(input);

    while set_items > 0 {
        let val = read_blob(input);
        let score_length = input.read_byte().unwrap();
        let score = input.read_exact(score_length as uint).unwrap();
        let score = unsafe{str::from_utf8_unchecked(score[])}.
            parse::<f64>().unwrap();

        set.push((score, val));

        set_items -= 1;
    }

    set
}

fn read_hash<T: Reader>(input: &mut T) -> Vec<Vec<u8>> {
    let mut hash = vec![];
    let mut hash_items = read_length(input);
    hash_items = 2*hash_items;

    while hash_items > 0 {
        let val = read_blob(input);
        hash.push(val);

        hash_items -= 1;
    }

    hash
}

fn read_list_ziplist<T: Reader>(input: &mut T) -> Vec<DataType> {
    let mut list = vec![];

    let ziplist = read_blob(input);

    let mut reader = MemReader::new(ziplist);

    let _zlbytes = reader.read_le_u32().unwrap();
    let _zltail = reader.read_le_u32().unwrap();
    let zllen = reader.read_le_u16().unwrap();

    for _ in range(0, zllen) {
        // 1. 1 or 5 bytes length of previous entry
        match reader.read_byte().unwrap() {
            254 => {
                let _ = reader.read_exact(4).unwrap();
            },
            _ => {}
        }

        let mut length : u64;
        let mut number_value : i64;

        // 2. Read flag or number value
        let flag = reader.read_byte().unwrap();

        match (flag & 0xC0) >> 6 {
            0 => { length = (flag & 0x3F) as u64 },
            1 => {
                let next_byte = reader.read_byte().unwrap();
                length = (((flag & 0x3F) as u64) << 8) | next_byte as u64;
            },
            2 => {
                length = input.read_le_u32().unwrap() as u64;
            },
            _ => {
                match (flag & 0xF0) >> 4 {
                    0xC => {
                        number_value = reader.read_le_i16().unwrap() as i64 },
                    0xD => {
                        number_value = reader.read_le_i32().unwrap() as i64 },
                    0xE => {
                        number_value = reader.read_le_i64().unwrap() as i64 },
                    0xF => {
                        match flag & 0xF {
                            0 => {
                                let bytes = reader.read_exact(3).unwrap();
                                number_value = ((bytes[0] as i64) << 16) ^
                                    ((bytes[1] as i64) << 8) ^
                                    (bytes[2] as i64);
                            },
                            0xE => {
                                number_value = reader.read_byte().unwrap() as i64 },
                            _ => { number_value = (flag & 0xF) as i64 - 1; }
                        }
                    },
                    _ => {
                        println!("Flag not handled: {}", flag);
                        continue;
                    }

                }

                list.push(DataType::Number(number_value));
                continue;
            }
        }

        // 3. Read value
        let rawval = reader.read_exact(length as uint).unwrap();
        list.push(DataType::String(rawval));
    }

    list
}
fn read_set_intset<T: Reader>(input: &mut T) -> Vec<i64> {
    let mut set = vec![];

    let intset = read_blob(input);

    let mut reader = MemReader::new(intset);
    let byte_size = reader.read_le_u32().unwrap();
    let intset_length = reader.read_le_u32().unwrap();

    for _ in range(0, intset_length) {
        let val = match byte_size {
            2 => reader.read_le_i16().unwrap() as i64,
            4 => reader.read_le_i32().unwrap() as i64,
            8 => reader.read_le_i64().unwrap(),
            _ => panic!("unhandled byte size in intset: {}", byte_size)
        };

        set.push(val);
    }

    set
}

fn read_type<T: Reader>(value_type: u8, input: &mut T) -> DataType {
    match value_type {
        types::STRING => {
            DataType::String(read_blob(input))
        },
        types::LIST => {
            DataType::List(read_linked_list(input))
        },
        types::SET => {
            DataType::Set(read_linked_list(input))
        },
        types::ZSET => {
            DataType::SortedSet(read_sorted_set(input))
        },
        types::HASH => {
            DataType::Hash(read_hash(input))
        },
        types::HASH_ZIPMAP => {
            println!("Value Type not implemented: {}", "HASH_ZIPMAP");
            //DataType::Hash(read_hash_zipmap(input))
            DataType::Hash(vec![])
        },
        types::LIST_ZIPLIST => {
            DataType::ListOfTypes(read_list_ziplist(input))
        },
        types::SET_INTSET => {
            DataType::Intset(read_set_intset(input))
        },
        types::ZSET_ZIPLIST => {
            DataType::SortedSetOfTypes(read_list_ziplist(input))
        },
        types::HASH_ZIPLIST => {
            DataType::ListOfTypes(read_list_ziplist(input))
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
    assert_eq!(
        true,
        verify_version(&mut MemReader::new(vec!(0x30, 0x30, 0x30, 0x33)))
    );

    assert_eq!(
        false,
        verify_version(&mut MemReader::new(vec!(0x30, 0x30, 0x30, 0x3a)))
    );
}

#[derive(Show,PartialEq)]
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
            length = (((enc_type & 0x3F) as u32) <<8) | next_byte as u32;
        },
        _ => {
            length = input.read_le_u32().unwrap();
        }
    }

    LengthEncoded::LE(length, is_encoded)
}

fn read_length<T: Reader>(input: &mut T) -> u32 {
    let LengthEncoded::LE(length, _) = read_length_with_encoding(input);
    length
}

fn int_to_vec(number: i32) -> Vec<u8> {
    let number = number.to_string();
    let mut result = Vec::with_capacity(number.len());
    for &c in number.as_bytes().iter() {
        result.push(c);
    }
    result
}

fn read_blob<T: Reader>(input: &mut T) -> Vec<u8> {
    let LengthEncoded::LE(length, is_encoded) = read_length_with_encoding(input);

    if is_encoded {
        match length {
            encoding::INT8 => { int_to_vec(input.read_i8().unwrap() as i32) },
            encoding::INT16 => { int_to_vec(input.read_le_i16().unwrap() as i32) },
            encoding::INT32 => { int_to_vec(input.read_le_i32().unwrap() as i32) },
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
    assert_eq!(
        vec!(0x61, 0x62, 0x63, 0x64),
        read_blob(&mut MemReader::new(vec!(
                    4, 0x61, 0x62, 0x63, 0x64))));
}
