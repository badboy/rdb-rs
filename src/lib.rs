#![feature(slicing_syntax)]
#![allow(unstable)]

extern crate lzf;
use std::str;
use lzf::decompress;
use std::io::MemReader;
use std::io;
use formatter::RdbParseFormatter;

pub use nil_formatter::NilFormatter;
pub use plain_formatter::PlainFormatter;
pub use json_formatter::JSONFormatter;

mod helper;

pub mod formatter;
pub mod nil_formatter;
pub mod plain_formatter;
pub mod json_formatter;


mod version {
    pub const SUPPORTED_MINIMUM : u32 = 1;
    pub const SUPPORTED_MAXIMUM : u32 = 7;
}

mod constants {
    pub const RDB_6BITLEN : u8 = 0;
    pub const RDB_14BITLEN : u8 = 1;
    pub const RDB_ENCVAL : u8 = 3;
    pub const RDB_MAGIC : &'static str = "REDIS";
}

mod op_codes {
    pub const AUX : u8 = 250;
    pub const RESIZEDB : u8 = 251;
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
    pub const LIST_QUICKLIST : u8 = 14;
}

mod encoding {
    pub const INT8 : u32 = 0;
    pub const INT16 : u32 = 1;
    pub const INT32 : u32 = 2;
    pub const LZF : u32 = 3;
}

#[derive(Show,Clone)]
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

pub struct RdbParser<R: Reader, F: RdbParseFormatter> {
    input: R,
    formatter: F
}


pub fn parse<R: Reader, F: RdbParseFormatter>(input: R, formatter: F) {
    let mut parser = RdbParser::new(input, formatter);
    parser.parse()
}

impl<R: Reader, F: RdbParseFormatter> RdbParser<R, F> {
    pub fn new(input: R, formatter: F) -> RdbParser<R, F> {
        RdbParser{input: input, formatter: formatter}
    }

    pub fn parse(&mut self) {
        assert!(self.verify_magic());
        assert!(self.verify_version());

        self.formatter.start_rdb();

        let mut out = io::stdout();
        let mut last_database : u32 = 0;
        loop {
            let next_op = self.input.read_byte().unwrap();

            match next_op {
                op_codes::SELECTDB => {
                    last_database = self.read_length();
                    self.formatter.start_database(last_database);
                },
                op_codes::EOF => {
                    self.formatter.end_database(last_database);
                    self.formatter.end_rdb();

                    let checksum = self.input.read_to_end().unwrap();
                    self.formatter.checksum(checksum);
                    break ;
                },
                op_codes::EXPIRETIME_MS => {
                    let expiretime_ms = self.input.read_le_u64().unwrap();
                    println!("EXPIRETIME_MS: {}", expiretime_ms);
                },
                op_codes::EXPIRETIME => {
                    let expiretime = self.input.read_be_u32().unwrap();
                    println!("EXPIRETIME: {}", expiretime);
                },
                op_codes::RESIZEDB => {
                    let db_size = self.read_length();
                    let expires_size = self.read_length();
                    println!("DB Size: {}, Expires Size: {}",
                             db_size, expires_size);
                },
                op_codes::AUX => {
                    let auxkey = self.read_blob();
                    let auxval = self.read_blob();

                    self.formatter.aux_field(auxkey, auxval);
                },
                _ => {
                    let key = self.read_blob();

                    match self.read_type(next_op) {
                        DataType::String(t) => {
                            self.formatter.set(key, t, None);
                        },
                        DataType::Number(t) => { println!("{}", t) },
                        DataType::ListOfTypes(_t) => { println!("ListOfTypes follows") },
                        DataType::Intset(_t) => { println!("Intset follows") },
                        DataType::Hash(t) => {
                            for val in t.iter() {
                                let _ = out.write(val.as_slice());
                                let _ = out.write_str(", ");
                            }
                            let _ = out.write_str("\n");
                        },
                        DataType::HashOfTypes(_t) => { println!("Hash follows") },
                        _ => {}
                    }
                }
            }

        }
    }

    pub fn verify_magic(&mut self) -> bool {
        let magic = self.input.read_exact(5).unwrap();

        // Meeeeeh.
        magic[0] == constants::RDB_MAGIC.as_bytes()[0] &&
            magic[1] == constants::RDB_MAGIC.as_bytes()[1] &&
            magic[2] == constants::RDB_MAGIC.as_bytes()[2] &&
            magic[3] == constants::RDB_MAGIC.as_bytes()[3] &&
            magic[4] == constants::RDB_MAGIC.as_bytes()[4]
    }

    pub fn verify_version(&mut self) -> bool {
        let version = self.input.read_exact(4).unwrap();

        let version = (version[0]-48) as u32 * 1000 +
            (version[1]-48) as u32 * 100 +
            (version[2]-48) as u32 * 10 +
            (version[3]-48) as u32;

        version >= version::SUPPORTED_MINIMUM &&
            version <= version::SUPPORTED_MAXIMUM
    }


    fn read_linked_list(&mut self) -> Vec<Vec<u8>> {
        let mut len = self.read_length();
        let mut list = vec![];

        while len > 0 {
            let blob = self.read_blob();
            list.push(blob);
            len -= 1;
        }

        list
    }

    fn read_sorted_set(&mut self) -> Vec<(f64,Vec<u8>)> {
        let mut set = vec![];
        let mut set_items = self.read_length();

        while set_items > 0 {
            let val = self.read_blob();
            let score_length = self.input.read_byte().unwrap();
            let score = match score_length {
                253 => { std::f64::NAN },
                254 => { std::f64::INFINITY },
                255 => { std::f64::NEG_INFINITY },
                _ => {
                    let tmp = self.input.read_exact(score_length as usize).unwrap();
                    unsafe{str::from_utf8_unchecked(tmp.as_slice())}.
                        parse::<f64>().unwrap()
                }
            };

            set.push((score, val));

            set_items -= 1;
        }

        set
    }

    fn read_hash(&mut self) -> Vec<Vec<u8>> {
        let mut hash = vec![];
        let mut hash_items = self.read_length();
        hash_items = 2*hash_items;

        while hash_items > 0 {
            let val = self.read_blob();
            hash.push(val);

            hash_items -= 1;
        }

        hash
    }

    fn read_ziplist_entry<T: Reader>(&mut self, ziplist: &mut T) -> DataType {
        // 1. 1 or 5 bytes length of previous entry
        match ziplist.read_byte().unwrap() {
            254 => {
                let _ = ziplist.read_exact(4).unwrap();
            },
            _ => {}
        }

        let mut length : u64;
        let mut number_value : i64;

        // 2. Read flag or number value
        let flag = ziplist.read_byte().unwrap();

        match (flag & 0xC0) >> 6 {
            0 => { length = (flag & 0x3F) as u64 },
            1 => {
                let next_byte = ziplist.read_byte().unwrap();
                length = (((flag & 0x3F) as u64) << 8) | next_byte as u64;
            },
            2 => {
                length = ziplist.read_be_u32().unwrap() as u64;
            },
            _ => {
                match (flag & 0xF0) >> 4 {
                    0xC => { number_value = ziplist.read_le_i16().unwrap() as i64 },
                    0xD => { number_value = ziplist.read_le_i32().unwrap() as i64 },
                    0xE => { number_value = ziplist.read_le_i64().unwrap() as i64 },
                    0xF => {
                        match flag & 0xF {
                            0 => {
                                let bytes = ziplist.read_exact(3).unwrap();
                                number_value = ((bytes[0] as i64) << 16) ^
                                    ((bytes[1] as i64) << 8) ^
                                    (bytes[2] as i64);
                            },
                            0xE => {
                                number_value = ziplist.read_byte().unwrap() as i64 },
                                _ => { number_value = (flag & 0xF) as i64 - 1; }
                        }
                    },
                    _ => {
                        println!("Flag not handled: {}", flag);
                        return DataType::Number(-42);
                    }

                }

                return DataType::Number(number_value)
            }
        }

        // 3. Read value
        let rawval = ziplist.read_exact(length as usize).unwrap();
        DataType::String(rawval)
    }

    fn read_list_ziplist(&mut self) -> Vec<DataType> {
        let ziplist = self.read_blob();

        let mut reader = MemReader::new(ziplist);

        let _zlbytes = reader.read_le_u32().unwrap();
        let _zltail = reader.read_le_u32().unwrap();
        let zllen = reader.read_le_u16().unwrap();
        let mut list = Vec::with_capacity(zllen as usize);

        for _ in range(0, zllen) {
            list.push(self.read_ziplist_entry(&mut reader));
        }

        assert!(reader.read_byte().unwrap() == 0xFF);

        list
    }

    fn read_zipmap_entry<T: Reader>(&mut self, next_byte: u8, zipmap: &mut T) -> Vec<u8> {
        let mut elem_len;
        match next_byte {
            253 => { elem_len = zipmap.read_le_u32().unwrap() },
            254 | 255 => {
                panic!("Invalid length value in zipmap: {}", next_byte)
            },
            _ => { elem_len = next_byte as u32 }
        }

        zipmap.read_exact(elem_len as usize).unwrap()
    }

    fn read_hash_zipmap(&mut self) -> Vec<Vec<u8>> {
        let zipmap = self.read_blob();

        let mut reader = MemReader::new(zipmap);

        let zmlen = reader.read_byte().unwrap();

        let mut length;
        let mut hash;
        if zmlen <= 254 {
            length = zmlen as usize;
            hash = Vec::with_capacity(length);
        } else {
            length = -1;
            hash = Vec::with_capacity(255);
        }

        loop {
            let next_byte = reader.read_byte().unwrap();

            if next_byte == 0xFF {
                break; // End of list.
            }

            let key = self.read_zipmap_entry(next_byte, &mut reader);
            hash.push(key);

            let next_byte = reader.read_byte().unwrap();
            let _free = reader.read_byte().unwrap();
            let value = self.read_zipmap_entry(next_byte, &mut reader);
            hash.push(value);

            if length > 0 {
                length -= 1;
            }

            if length == 0 {
                assert!(reader.read_byte().unwrap() == 0xFF);
                break;
            }
        }

        hash
    }

    fn read_set_intset(&mut self) -> Vec<i64> {
        let mut set = vec![];

        let intset = self.read_blob();

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

    fn read_quicklist(&mut self) -> Vec<DataType> {
        let len = self.read_length();

        let mut list = vec![];
        for _ in range(0, len) {
            let zl = self.read_list_ziplist();
            list.push_all(zl.as_slice());
        }
        list
    }

    fn read_type(&mut self, value_type: u8) -> DataType {
        match value_type {
            types::STRING => {
                DataType::String(self.read_blob())
            },
            types::LIST => {
                DataType::List(self.read_linked_list())
            },
            types::SET => {
                DataType::Set(self.read_linked_list())
            },
            types::ZSET => {
                DataType::SortedSet(self.read_sorted_set())
            },
            types::HASH => {
                DataType::Hash(self.read_hash())
            },
            types::HASH_ZIPMAP => {
                DataType::Hash(self.read_hash_zipmap())
            },
            types::LIST_ZIPLIST => {
                DataType::ListOfTypes(self.read_list_ziplist())
            },
            types::SET_INTSET => {
                DataType::Intset(self.read_set_intset())
            },
            types::ZSET_ZIPLIST => {
                DataType::SortedSetOfTypes(self.read_list_ziplist())
            },
            types::HASH_ZIPLIST => {
                DataType::ListOfTypes(self.read_list_ziplist())
            },
            types::LIST_QUICKLIST => {
                DataType::ListOfTypes(self.read_quicklist())
            },
            _ => { panic!("Value Type not implemented: {}", value_type) }
        }
    }

    pub fn read_length_with_encoding(&mut self) -> (u32, bool) {
        let mut length;
        let mut is_encoded = false;

        let enc_type = self.input.read_byte().unwrap();

        match (enc_type & 0xC0) >> 6 {
            constants::RDB_ENCVAL => {
                is_encoded = true;
                length = (enc_type & 0x3F) as u32;
            },
            constants::RDB_6BITLEN => {
                length = (enc_type & 0x3F) as u32;
            },
            constants::RDB_14BITLEN => {
                let next_byte = self.input.read_byte().unwrap();
                length = (((enc_type & 0x3F) as u32) <<8) | next_byte as u32;
            },
            _ => {
                length = self.input.read_be_u32().unwrap();
            }
        }

        (length, is_encoded)
    }

    pub fn read_length(&mut self) -> u32 {
        let (length, _) = self.read_length_with_encoding();
        length
    }

    pub fn read_blob(&mut self) -> Vec<u8> {
        let (length, is_encoded) = self.read_length_with_encoding();

        if is_encoded {
            match length {
                encoding::INT8 => { helper::int_to_vec(self.input.read_i8().unwrap() as i32) },
                encoding::INT16 => { helper::int_to_vec(self.input.read_le_i16().unwrap() as i32) },
                encoding::INT32 => { helper::int_to_vec(self.input.read_le_i32().unwrap() as i32) },
                encoding::LZF => {
                    let compressed_length = self.read_length();
                    let real_length = self.read_length();
                    let data = self.input.read_exact(compressed_length as usize).unwrap();
                    lzf::decompress(data.as_slice(), real_length as usize).unwrap()
                },
                _ => { panic!("Unknown encoding: {}", length) }
            }
        } else {
            self.input.read_exact(length as usize).unwrap()
        }
    }
}
