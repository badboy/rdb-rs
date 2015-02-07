use std::{str,f64};
use std::old_io::{MemReader,IoError,IoErrorKind};
use lzf;

use helper;
use formatter::Formatter;
use filter::Filter;

#[doc(hidden)]
use constants::{
    version,
    constant,
    op_code,
    encoding_type,
    encoding
};

#[doc(hidden)]
pub use types::{
    ZiplistEntry,
    Type,

    /* error and result types */
    RdbError,
    RdbResult,
    RdbOk,

    EncodingType
};

pub struct RdbParser<R: Reader, F: Formatter, L: Filter> {
    input: R,
    formatter: F,
    filter: L,
    last_expiretime: Option<u64>
}

pub fn read_length_with_encoding<R: Reader>(input: &mut R) -> RdbResult<(u32, bool)> {
    let mut length;
    let mut is_encoded = false;

    let enc_type = try!(input.read_byte());

    match (enc_type & 0xC0) >> 6 {
        constant::RDB_ENCVAL => {
            is_encoded = true;
            length = (enc_type & 0x3F) as u32;
        },
        constant::RDB_6BITLEN => {
            length = (enc_type & 0x3F) as u32;
        },
        constant::RDB_14BITLEN => {
            let next_byte = try!(input.read_byte());
            length = (((enc_type & 0x3F) as u32) <<8) | next_byte as u32;
        },
        _ => {
            length = try!(input.read_be_u32());
        }
    }

    Ok((length, is_encoded))
}

pub fn read_length<R: Reader>(input: &mut R) -> RdbResult<u32> {
    let (length, _) = try!(read_length_with_encoding(input));
    Ok(length)
}

pub fn verify_magic<R: Reader>(input: &mut R) -> RdbOk {
    let magic = try!(input.read_exact(5));

    // Meeeeeh.
    let is_ok = magic[0] == constant::RDB_MAGIC.as_bytes()[0] &&
        magic[1] == constant::RDB_MAGIC.as_bytes()[1] &&
        magic[2] == constant::RDB_MAGIC.as_bytes()[2] &&
        magic[3] == constant::RDB_MAGIC.as_bytes()[3] &&
        magic[4] == constant::RDB_MAGIC.as_bytes()[4];

    if is_ok {
        Ok(())
    } else {
        Err(IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "Invalid magic string",
            detail: None
        })
    }
}

pub fn verify_version<R: Reader>(input: &mut R) -> RdbOk {
    let version = try!(input.read_exact(4));

    let version = (version[0]-48) as u32 * 1000 +
        (version[1]-48) as u32 * 100 +
        (version[2]-48) as u32 * 10 +
        (version[3]-48) as u32;

    let is_ok = version >= version::SUPPORTED_MINIMUM &&
        version <= version::SUPPORTED_MAXIMUM;

    if is_ok {
        Ok(())
    } else {
        Err(IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "Version not supported",
            detail: None
        })
    }
}

pub fn read_blob<R: Reader>(input: &mut R) -> RdbResult<Vec<u8>> {
    let (length, is_encoded) = try!(read_length_with_encoding(input));

    if is_encoded {
        let result = match length {
            encoding::INT8 => { helper::int_to_vec(try!(input.read_i8()) as i32) },
            encoding::INT16 => { helper::int_to_vec(try!(input.read_le_i16()) as i32) },
            encoding::INT32 => { helper::int_to_vec(try!(input.read_le_i32()) as i32) },
            encoding::LZF => {
                let compressed_length = try!(read_length(input));
                let real_length = try!(read_length(input));
                let data = try!(input.read_exact(compressed_length as usize));
                lzf::decompress(data.as_slice(), real_length as usize).unwrap()
            },
            _ => { panic!("Unknown encoding: {}", length) }
        };

        Ok(result)
    } else {
        input.read_exact(length as usize)
    }
}

fn read_ziplist_metadata<T: Reader>(input: &mut T) -> RdbResult<(u32, u32, u16)> {
    let zlbytes = try!(input.read_le_u32());
    let zltail = try!(input.read_le_u32());
    let zllen = try!(input.read_le_u16());

    Ok((zlbytes, zltail, zllen))
}

impl<R: Reader, F: Formatter, L: Filter> RdbParser<R, F, L> {
    pub fn new(input: R, formatter: F, filter: L) -> RdbParser<R, F, L> {
        RdbParser{
            input: input,
            formatter: formatter,
            filter: filter,
            last_expiretime: None
        }
    }

    pub fn parse(&mut self) -> RdbOk {
        try!(verify_magic(&mut self.input));
        try!(verify_version(&mut self.input));

        self.formatter.start_rdb();

        let mut last_database : u32 = 0;
        loop {
            let next_op = try!(self.input.read_byte());

            match next_op {
                op_code::SELECTDB => {
                    last_database = unwrap_or_panic!(read_length(&mut self.input));
                    if self.filter.matches_db(last_database) {
                        self.formatter.start_database(last_database);
                    }
                },
                op_code::EOF => {
                    self.formatter.end_database(last_database);
                    self.formatter.end_rdb();

                    let checksum = try!(self.input.read_to_end());
                    if checksum.len() > 0 {
                        self.formatter.checksum(checksum.as_slice());
                    }
                    break;
                },
                op_code::EXPIRETIME_MS => {
                    let expiretime_ms = try!(self.input.read_le_u64());
                    self.last_expiretime = Some(expiretime_ms);
                },
                op_code::EXPIRETIME => {
                    let expiretime = try!(self.input.read_be_u32());
                    self.last_expiretime = Some(expiretime as u64 * 1000);
                },
                op_code::RESIZEDB => {
                    let db_size = try!(read_length(&mut self.input));
                    let expires_size = try!(read_length(&mut self.input));

                    self.formatter.resizedb(db_size, expires_size);
                },
                op_code::AUX => {
                    let auxkey = try!(read_blob(&mut self.input));
                    let auxval = try!(read_blob(&mut self.input));

                    self.formatter.aux_field(
                        auxkey.as_slice(),
                        auxval.as_slice());
                },
                _ => {
                    if self.filter.matches_db(last_database) {
                        let key = try!(read_blob(&mut self.input));

                        if self.filter.matches_type(next_op) && self.filter.matches_key(key.as_slice()) {
                            try!(self.read_type(key.as_slice(), next_op));
                        } else {
                            try!(self.skip_object(next_op));
                        }
                    } else {
                        try!(self.skip_key_and_object(next_op));
                    }

                    self.last_expiretime = None;
                }
            }
        }

        Ok(())
    }

    fn read_set_linked_list(&mut self, key: &[u8]) -> RdbOk {
        let mut len = try!(read_length(&mut self.input));

        self.formatter.start_set(key, len, self.last_expiretime, EncodingType::LinkedList);

        while len > 0 {
            let blob = try!(read_blob(&mut self.input));
            self.formatter.list_element(key, blob.as_slice());
            len -= 1;
        }

        self.formatter.end_set(key);

        Ok(())
    }

    fn read_linked_list(&mut self, key: &[u8], typ: Type) -> RdbOk {
        let mut len = try!(read_length(&mut self.input));

        match typ {
            Type::List => {
                self.formatter.start_list(key, len, self.last_expiretime, EncodingType::LinkedList);
            },
            Type::Set => {
                self.formatter.start_set(key, len, self.last_expiretime, EncodingType::LinkedList);
            },
            _ => { panic!("Unknown encoding type for linked list") }
        }

        while len > 0 {
            let blob = try!(read_blob(&mut self.input));
            self.formatter.list_element(key, blob.as_slice());
            len -= 1;
        }

        self.formatter.end_list(key);
        match typ {
            Type::List => self.formatter.end_list(key),
            Type::Set => self.formatter.end_set(key),
            _ => { panic!("Unknown encoding type for linked list") }
        }

        Ok(())
    }

    fn read_sorted_set(&mut self, key: &[u8]) -> RdbOk {
        let mut set_items = unwrap_or_panic!(read_length(&mut self.input));

        self.formatter.start_sorted_set(key, set_items, self.last_expiretime, EncodingType::Hashtable);

        while set_items > 0 {
            let val = try!(read_blob(&mut self.input));
            let score_length = try!(self.input.read_byte());
            let score = match score_length {
                253 => { f64::NAN },
                254 => { f64::INFINITY },
                255 => { f64::NEG_INFINITY },
                _ => {
                    let tmp = try!(self.input.read_exact(score_length as usize));
                    unsafe{str::from_utf8_unchecked(tmp.as_slice())}.
                        parse::<f64>().unwrap()
                }
            };

            self.formatter.sorted_set_element(key, score, val.as_slice());

            set_items -= 1;
        }

        self.formatter.end_sorted_set(key);

        Ok(())
    }

    fn read_hash(&mut self, key: &[u8]) -> RdbOk {
        let mut hash_items = try!(read_length(&mut self.input));

        self.formatter.start_hash(key, hash_items, self.last_expiretime, EncodingType::Hashtable);

        while hash_items > 0 {
            let field = try!(read_blob(&mut self.input));
            let val = try!(read_blob(&mut self.input));

            self.formatter.hash_element(key, field.as_slice(), val.as_slice());

            hash_items -= 1;
        }

        self.formatter.end_hash(key);

        Ok(())
    }

    fn read_ziplist_entry<T: Reader>(&mut self, ziplist: &mut T) -> RdbResult<ZiplistEntry> {
        // 1. 1 or 5 bytes length of previous entry
        let byte = try!(ziplist.read_byte());
        if byte == 254 {
            try!(ziplist.read_exact(4));
        }

        let mut length : u64;
        let mut number_value : i64;

        // 2. Read flag or number value
        let flag = try!(ziplist.read_byte());

        match (flag & 0xC0) >> 6 {
            0 => { length = (flag & 0x3F) as u64 },
            1 => {
                let next_byte = try!(ziplist.read_byte());
                length = (((flag & 0x3F) as u64) << 8) | next_byte as u64;
            },
            2 => {
                length = try!(ziplist.read_be_u32()) as u64;
            },
            _ => {
                match (flag & 0xF0) >> 4 {
                    0xC => { number_value = try!(ziplist.read_le_i16()) as i64 },
                    0xD => { number_value = try!(ziplist.read_le_i32()) as i64 },
                    0xE => { number_value = try!(ziplist.read_le_i64()) as i64 },
                    0xF => {
                        match flag & 0xF {
                            0 => {
                                let bytes = try!(ziplist.read_exact(3));
                                number_value = ((bytes[0] as i64) << 16) ^
                                    ((bytes[1] as i64) << 8) ^
                                    (bytes[2] as i64);
                            },
                            0xE => {
                                number_value = try!(ziplist.read_byte()) as i64 },
                                _ => { number_value = (flag & 0xF) as i64 - 1; }
                        }
                    },
                    _ => {
                        panic!("Flag not handled: {}", flag);
                    }

                }

                return Ok(ZiplistEntry::Number(number_value));
            }
        }

        // 3. Read value
        let rawval = try!(ziplist.read_exact(length as usize));
        Ok(ZiplistEntry::String(rawval))
    }

    fn read_ziplist_entry_string<T: Reader>(& mut self, reader: &mut T) -> RdbResult<Vec<u8>> {
        let entry = try!(self.read_ziplist_entry(reader));
        match entry {
            ZiplistEntry::String(val) => Ok(val),
            ZiplistEntry::Number(val) => Ok(val.to_string().into_bytes())
        }
    }

    fn read_list_ziplist(&mut self, key: &[u8], typ: Type) -> RdbOk {
        let ziplist = try!(read_blob(&mut self.input));
        let raw_length = ziplist.len() as u64;

        let mut reader = MemReader::new(ziplist);
        let (_zlbytes, _zltail, zllen) = try!(read_ziplist_metadata(&mut reader));

        match typ {
            Type::List => {
                self.formatter.start_list(key, zllen as u32,
                                          self.last_expiretime,
                                          EncodingType::Ziplist(raw_length));
            },
            Type::Set => {
                self.formatter.start_set(key, zllen as u32,
                                          self.last_expiretime,
                                          EncodingType::Ziplist(raw_length));
            },
            Type::SortedSet => {
                self.formatter.start_sorted_set(key, zllen as u32,
                                          self.last_expiretime,
                                          EncodingType::Ziplist(raw_length));
            },
            _ => panic!("Unknown encoding type in ziplist: {:?}", typ)
        }

        for _ in (0..zllen) {
            let entry = try!(self.read_ziplist_entry_string(&mut reader));
            self.formatter.list_element(key, entry.as_slice());
        }

        let last_byte = try!(reader.read_byte());
        if last_byte != 0xFF {
            return Err(IoError {
                kind: IoErrorKind::OtherIoError,
                desc: "Invalid end byte of ziplist",
                detail: None
            })
        }

        self.formatter.end_list(key);
        match typ {
            Type::List => self.formatter.end_list(key),
            Type::Set => self.formatter.end_set(key),
            Type::SortedSet => self.formatter.end_sorted_set(key),
            _ => panic!("Unknown encoding type in ziplist")
        }

        Ok(())
    }

    fn read_sortedset_ziplist(&mut self, key: &[u8]) -> RdbOk {
        let ziplist = try!(read_blob(&mut self.input));
        let raw_length = ziplist.len() as u64;

        let mut reader = MemReader::new(ziplist);
        let (_zlbytes, _zltail, zllen) = try!(read_ziplist_metadata(&mut reader));

        self.formatter.start_sorted_set(key, zllen as u32,
                                        self.last_expiretime,
                                        EncodingType::Ziplist(raw_length));

        assert!(zllen%2 == 0);
        let zllen = zllen / 2;

        for _ in (0..zllen) {
            let entry = try!(self.read_ziplist_entry_string(&mut reader));
            let score = try!(self.read_ziplist_entry_string(&mut reader));
            let score = str::from_utf8(score.as_slice())
                .unwrap()
                .parse::<f64>().unwrap();
            self.formatter.sorted_set_element(key,
                                              score,
                                              entry.as_slice());
        }

        let last_byte = try!(reader.read_byte());
        if last_byte != 0xFF {
            return Err(IoError {
                kind: IoErrorKind::OtherIoError,
                desc: "Invalid end byte of ziplist",
                detail: None
            })
        }

        self.formatter.end_sorted_set(key);

        Ok(())
    }

    fn read_quicklist_ziplist(&mut self, key: &[u8]) -> RdbOk {
        let ziplist = try!(read_blob(&mut self.input));

        let mut reader = MemReader::new(ziplist);
        let (_zlbytes, _zltail, zllen) = try!(read_ziplist_metadata(&mut reader));

        for _ in (0..zllen) {
            let entry = try!(self.read_ziplist_entry_string(&mut reader));
            self.formatter.list_element(key, entry.as_slice());
        }

        let last_byte = try!(reader.read_byte());
        if last_byte != 0xFF {
            return Err(IoError {
                kind: IoErrorKind::OtherIoError,
                desc: "Invalid end byte of ziplist (quicklist)",
                detail: None
            })
        }

        Ok(())
    }

    fn read_zipmap_entry<T: Reader>(&mut self, next_byte: u8, zipmap: &mut T) -> RdbResult<Vec<u8>> {
        let mut elem_len;
        match next_byte {
            253 => { elem_len = zipmap.read_le_u32().unwrap() },
            254 | 255 => {
                panic!("Invalid length value in zipmap: {}", next_byte)
            },
            _ => { elem_len = next_byte as u32 }
        }

        zipmap.read_exact(elem_len as usize)
    }

    fn read_hash_zipmap(&mut self, key: &[u8]) -> RdbOk {
        let zipmap = try!(read_blob(&mut self.input));
        let raw_length = zipmap.len() as u64;

        let mut reader = MemReader::new(zipmap);

        let zmlen = try!(reader.read_byte());

        let mut length;
        let mut size;
        if zmlen <= 254 {
            length = zmlen;
            size = zmlen
        } else {
            length = -1;
            size = 0;
        }

        self.formatter.start_hash(key, size as u32, self.last_expiretime,
                                  EncodingType::Zipmap(raw_length));

        loop {
            let next_byte = try!(reader.read_byte());

            if next_byte == 0xFF {
                break; // End of list.
            }

            let field = try!(self.read_zipmap_entry(next_byte, &mut reader));

            let next_byte = try!(reader.read_byte());
            let _free = try!(reader.read_byte());
            let value = try!(self.read_zipmap_entry(next_byte, &mut reader));

            self.formatter.hash_element(key, field.as_slice(), value.as_slice());

            if length > 0 {
                length -= 1;
            }

            if length == 0 {
                let last_byte = try!(reader.read_byte());

                if last_byte != 0xFF {
                    return Err(IoError {
                        kind: IoErrorKind::OtherIoError,
                        desc: "Invalid end byte of zipmap",
                        detail: None
                    })
                }
                break;
            }
        }

        self.formatter.end_hash(key);

        Ok(())
    }

    fn read_set_intset(&mut self, key: &[u8]) -> RdbOk {
        let intset = try!(read_blob(&mut self.input));
        let raw_length = intset.len() as u64;

        let mut reader = MemReader::new(intset);
        let byte_size = try!(reader.read_le_u32());
        let intset_length = try!(reader.read_le_u32());

        self.formatter.start_set(key, intset_length, self.last_expiretime,
                                 EncodingType::Intset(raw_length));

        for _ in (0..intset_length) {
            let val = match byte_size {
                2 => try!(reader.read_le_i16()) as i64,
                4 => try!(reader.read_le_i32()) as i64,
                8 => try!(reader.read_le_i64()),
                _ => panic!("unhandled byte size in intset: {}", byte_size)
            };

            self.formatter.set_element(key, val.to_string().as_bytes());
        }

        self.formatter.end_set(key);

        Ok(())
    }

    fn read_quicklist(&mut self, key: &[u8]) -> RdbOk {
        let len = try!(read_length(&mut self.input));

        self.formatter.start_set(key, 0, self.last_expiretime, EncodingType::Quicklist);
        for _ in (0..len) {
            try!(self.read_quicklist_ziplist(key));
        }
        self.formatter.end_set(key);

        Ok(())
    }

    fn read_type(&mut self, key: &[u8], value_type: u8) -> RdbOk {
        match value_type {
            encoding_type::STRING => {
                let val = try!(read_blob(&mut self.input));
                self.formatter.set(key, val.as_slice(), self.last_expiretime);
            },
            encoding_type::LIST => {
                try!(self.read_linked_list(key, Type::List))
            },
            encoding_type::SET => {
                try!(self.read_linked_list(key, Type::Set))
            },
            encoding_type::ZSET => {
                try!(self.read_sorted_set(key))
            },
            encoding_type::HASH => {
                try!(self.read_hash(key))
            },
            encoding_type::HASH_ZIPMAP => {
                try!(self.read_hash_zipmap(key))
            },
            encoding_type::LIST_ZIPLIST => {
                try!(self.read_list_ziplist(key, Type::List))
            },
            encoding_type::SET_INTSET => {
                try!(self.read_set_intset(key))
            },
            encoding_type::ZSET_ZIPLIST => {
                try!(self.read_sortedset_ziplist(key))
            },
            encoding_type::HASH_ZIPLIST => {
                try!(self.read_list_ziplist(key, Type::Hash))
            },
            encoding_type::LIST_QUICKLIST => {
                try!(self.read_quicklist(key))
            },
            _ => { panic!("Value Type not implemented: {}", value_type) }
        };

        Ok(())
    }

    fn skip(&mut self, skip_bytes: usize) -> RdbResult<()> {
        match self.input.read_exact(skip_bytes) {
            Ok(_) => Ok(()),
            Err(e) => Err(e)
        }
    }

    fn skip_blob(&mut self) -> RdbResult<()> {
        let (len, is_encoded) = unwrap_or_panic!(read_length_with_encoding(&mut self.input));
        let mut skip_bytes;

        if is_encoded {
            skip_bytes = match len {
                encoding::INT8 => 1,
                encoding::INT16 => 2,
                encoding::INT32 => 4,
                encoding::LZF => {
                    let compressed_length = unwrap_or_panic!(read_length(&mut self.input));
                    let _real_length = unwrap_or_panic!(read_length(&mut self.input));
                    compressed_length
                },
                _ => { panic!("Unknown encoding: {}", len) }
            }
        } else {
            skip_bytes = len;
        }

        self.skip(skip_bytes as usize)
    }

    fn skip_object(&mut self, enc_type: u8) -> RdbResult<()> {
        let blobs_to_skip = match enc_type {
            encoding_type::STRING |
                encoding_type::HASH_ZIPMAP |
                encoding_type::LIST_ZIPLIST |
                encoding_type::SET_INTSET |
                encoding_type::ZSET_ZIPLIST |
                encoding_type::HASH_ZIPLIST => 1,
            encoding_type::LIST | encoding_type::SET => unwrap_or_panic!(read_length(&mut self.input)),
            encoding_type::ZSET | encoding_type::HASH => unwrap_or_panic!(read_length(&mut self.input)) * 2,
            _ => { panic!("Unknown encoding type: {}", enc_type) }
        };

        for _ in (0..blobs_to_skip) {
            try!(self.skip_blob())
        }

        Ok(())
    }

    fn skip_key_and_object(&mut self, enc_type: u8) -> RdbResult<()> {
        try!(self.skip_blob());
        try!(self.skip_object(enc_type));
        Ok(())
    }
}
