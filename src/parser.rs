use std::{str, f64};
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::io::{Read, Cursor};
use std::iter;
use std::mem;
use byteorder::{LittleEndian, BigEndian, ReadBytesExt};
use lzf;

use helper;
use helper::read_exact;
use iterator_type::RdbIteratorType;
use iterator_type::RdbIteratorType::*;

use filter::Filter;

#[derive(Debug,Clone)]
enum RdbParserState {
    Empty,
    Start,
    Version,
    OpCode,
    RdbEnd,
    Finished,
    Value(u8),
    List(u32),
    Set(u32),
    SortedSet(u32),
    Hash(u32),
    Zipmap(Cursor<Vec<u8>>, i32),
    SetIntset(Cursor<Vec<u8>>, u32, u32),

    ListZiplist(Cursor<Vec<u8>>, u32),
    SortedSetZiplist(Cursor<Vec<u8>>, u32),
    HashZiplist(Cursor<Vec<u8>>, u32),

    Quicklist(u32, Option<(Cursor<Vec<u8>>, u16)>),
}

pub type RdbIteratorResult = RdbResult<RdbIteratorType>;

#[doc(hidden)]
use constants::{version, constant, op_code, encoding_type, encoding};

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

pub struct RdbParser<R: Read, L: Filter> {
    input: R,
    filter: L,
    last_expiretime: Option<u64>,
    last_database: u32,
    state: RdbParserState,
}

// Yo, I heart you like options, so I put some Result in your Option
// and some Enums in your Result and some data into your Enums.
impl<R: Read, F: Filter> iter::Iterator for RdbParser<R,F> {
    type Item = RdbIteratorResult;

    fn next(&mut self) -> Option<Self::Item> {
        match self.advance() {
            Ok(EOF) => None,
            val @ _ => Some(val),
        }
    }
}
#[inline]
fn other_error(desc: &'static str) -> IoError {
    IoError::new(IoErrorKind::Other, desc)
}

pub fn read_length_with_encoding<R: Read>(input: &mut R) -> RdbResult<(u32, bool)> {
    let length;
    let mut is_encoded = false;

    let enc_type = try!(input.read_u8());

    match (enc_type & 0xC0) >> 6 {
        constant::RDB_ENCVAL => {
            is_encoded = true;
            length = (enc_type & 0x3F) as u32;
        }
        constant::RDB_6BITLEN => {
            length = (enc_type & 0x3F) as u32;
        }
        constant::RDB_14BITLEN => {
            let next_byte = try!(input.read_u8());
            length = (((enc_type & 0x3F) as u32) << 8) | next_byte as u32;
        }
        _ => {
            length = try!(input.read_u32::<BigEndian>());
        }
    }

    Ok((length, is_encoded))
}

pub fn read_length<R: Read>(input: &mut R) -> RdbResult<u32> {
    let (length, _) = try!(read_length_with_encoding(input));
    Ok(length)
}

pub fn verify_magic<R: Read>(input: &mut R) -> RdbOk {
    let mut magic = [0; 5];
    match input.read(&mut magic) {
        Ok(5) => (),
        Ok(_) => return Err(other_error("Could not read enough bytes for the magic")),
        Err(e) => return Err(e),
    };

    if magic == constant::RDB_MAGIC.as_bytes() {
        Ok(())
    } else {
        Err(other_error("Invalid magic string"))
    }
}

pub fn verify_version<R: Read>(input: &mut R) -> RdbOk {
    let mut version = [0; 4];
    match input.read(&mut version) {
        Ok(4) => (),
        Ok(_) => return Err(other_error("Could not read enough bytes for the version")),
        Err(e) => return Err(e),
    };

    let version = (version[0]-48) as u32 * 1000 + (version[1]-48) as u32 * 100 +
                  (version[2]-48) as u32 * 10 + (version[3]-48) as u32;

    let is_ok = version >= version::SUPPORTED_MINIMUM && version <= version::SUPPORTED_MAXIMUM;

    if is_ok {
        Ok(())
    } else {
        Err(other_error("Version not supported"))
    }
}

pub fn read_blob<R: Read>(input: &mut R) -> RdbResult<Vec<u8>> {
    let (length, is_encoded) = try!(read_length_with_encoding(input));

    if is_encoded {
        let result = match length {
            encoding::INT8 => {
                helper::int_to_vec(try!(input.read_i8()) as i32)
            }
            encoding::INT16 => {
                helper::int_to_vec(try!(input.read_i16::<LittleEndian>()) as i32)
            }
            encoding::INT32 => {
                helper::int_to_vec(try!(input.read_i32::<LittleEndian>()) as i32)
            }
            encoding::LZF => {
                let compressed_length = try!(read_length(input));
                let real_length = try!(read_length(input));
                let data = try!(read_exact(input, compressed_length as usize));
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

fn read_ziplist_metadata<T: Read>(input: &mut T) -> RdbResult<(u32, u32, u16)> {
    let zlbytes = try!(input.read_u32::<LittleEndian>());
    let zltail = try!(input.read_u32::<LittleEndian>());
    let zllen = try!(input.read_u16::<LittleEndian>());

    Ok((zlbytes, zltail, zllen))
}

impl<R: Read, F: Filter> RdbParser<R, F> {
    pub fn new(input: R, filter: F) -> RdbParser<R, F> {
        RdbParser {
            input: input,
            filter: filter,
            last_expiretime: None,
            last_database: 0,
            state: RdbParserState::Start,
        }
    }

    fn advance(&mut self) -> RdbIteratorResult {
        loop {
            match self._next() {
                Ok(Skipped) => continue,
                // read_opcode already set the new state
                result @ _ => return result,
            }
        }
    }

    fn _next(&mut self) -> RdbIteratorResult {
        let state = mem::replace(&mut self.state, RdbParserState::Empty);

        match state {
            RdbParserState::Start => {
                try!(verify_magic(&mut self.input));
                self.state = RdbParserState::Version;
            }
            RdbParserState::Version => {
                try!(verify_version(&mut self.input));
                self.state = RdbParserState::OpCode;
            }
            RdbParserState::OpCode => {
                return self.read_opcode();
            }
            RdbParserState::RdbEnd => {
                let mut checksum = Vec::new();
                let len = try!(self.input.read_to_end(&mut checksum));

                self.state = RdbParserState::Finished;

                if len > 0 {
                    return Ok(Checksum(checksum))
                }
            }
            RdbParserState::Finished => {
                self.state = RdbParserState::Finished;
                return Ok(EOF);
            }

            RdbParserState::Value(op) => {
                return self.read_type(op);
            }

            RdbParserState::List(0) => {
                self.state = RdbParserState::OpCode;
                return Ok(ListEnd);
            }
            RdbParserState::List(len) => {
                return self.read_linked_list_element(Type::List, len);
            }

            RdbParserState::Set(0) => {
                self.state = RdbParserState::OpCode;
                return Ok(SetEnd);
            }
            RdbParserState::Set(len) => {
                return self.read_linked_list_element(Type::Set, len);
            }

            RdbParserState::Hash(0) => {
                self.state = RdbParserState::OpCode;
                return Ok(HashEnd);
            }
            RdbParserState::Hash(len) => {
                return self.read_hash_element(len);
            }

            RdbParserState::SortedSet(0) => {
                self.state = RdbParserState::OpCode;
                return Ok(SortedSetEnd);
            }
            RdbParserState::SortedSet(len) => {
                return self.read_sorted_set_element(len);
            }

            RdbParserState::Zipmap(reader, len) => {
                return self.read_hash_zipmap_element(reader, len);
            }

            RdbParserState::ListZiplist(reader, len) => {
                return self.read_list_ziplist_element(reader, len);
            }

            RdbParserState::SortedSetZiplist(reader, len) => {
                return self.read_zset_ziplist_element(reader, len);
            }

            RdbParserState::HashZiplist(reader, len) => {
                return self.read_hash_ziplist_element(reader, len);
            }

            RdbParserState::SetIntset(_, 0, _) => {
                self.state = RdbParserState::OpCode;
                return Ok(SetEnd);
            }
            RdbParserState::SetIntset(reader, len, byte_size) => {
                return self.read_set_intset_element(reader, len, byte_size);
            }

            RdbParserState::Quicklist(len, opt) => {
                return self.read_quicklist_element(len, opt);
            }

            _ => {
                panic!("Unimplemented state encountered: {:?}", self.state);
            }

        };

        Ok(Skipped)
    }

    fn read_opcode(&mut self) -> RdbIteratorResult {
        let next_op = try!(self.input.read_u8());

        match next_op {
            op_code::SELECTDB => {
                self.last_database = try!(read_length(&mut self.input));
                self.state = RdbParserState::OpCode;

                if self.filter.matches_db(self.last_database) {
                    return Ok(StartDatabase(self.last_database));
                } else {
                    return Ok(Skipped);
                }
            }
            op_code::EOF => {
                self.state = RdbParserState::RdbEnd;
                return Ok(RdbEnd)
            }
            op_code::EXPIRETIME_MS => {
                let expiretime_ms = try!(self.input.read_u64::<LittleEndian>());
                self.last_expiretime = Some(expiretime_ms);

                self.state = RdbParserState::OpCode;
                return Ok(Skipped);
            }
            op_code::EXPIRETIME => {
                let expiretime = try!(self.input.read_u32::<BigEndian>());
                self.last_expiretime = Some(expiretime as u64 * 1000);

                self.state = RdbParserState::OpCode;
                return Ok(Skipped);
            }
            op_code::RESIZEDB => {
                let db_size = try!(read_length(&mut self.input));
                let expires_size = try!(read_length(&mut self.input));

                self.state = RdbParserState::OpCode;
                return Ok(ResizeDB(db_size, expires_size));
            }
            op_code::AUX => {
                let auxkey = try!(read_blob(&mut self.input));
                let auxval = try!(read_blob(&mut self.input));

                self.state = RdbParserState::OpCode;
                return Ok(AuxiliaryKey(auxkey, auxval))
            }
            _ => {
                if !self.filter.matches_db(self.last_database) {
                    try!(self.skip_key_and_object(next_op));
                    return Ok(Skipped);
                }

                let key = try!(read_blob(&mut self.input));
                if self.filter.matches_type(next_op) && self.filter.matches_key(&key) {
                    self.state = RdbParserState::Value(next_op);
                    return Ok(Key(key, self.last_expiretime.take()));
                } else {
                    try!(self.skip_object(next_op));
                    return Ok(Skipped);
                }
            }
        };
    }

    fn read_ziplist_entry<T: Read>(&mut self, ziplist: &mut T) -> RdbResult<ZiplistEntry> {
        // 1. 1 or 5 bytes length of previous entry
        let byte = try!(ziplist.read_u8());
        if byte == 254 {
            let mut bytes = [0; 4];
            match ziplist.read(&mut bytes) {
                Ok(4) => (),
                Ok(_) => return Err(other_error("Could not read 4 bytes to skip after ziplist \
                                                 length")),
                Err(e) => return Err(e),
            };
        }

        let length: u64;
        let number_value: i64;

        // 2. Read flag or number value
        let flag = try!(ziplist.read_u8());

        match (flag & 0xC0) >> 6 {
            0 => {
                length = (flag & 0x3F) as u64
            }
            1 => {
                let next_byte = try!(ziplist.read_u8());
                length = (((flag & 0x3F) as u64) << 8) | next_byte as u64;
            }
            2 => {
                length = try!(ziplist.read_u32::<BigEndian>()) as u64;
            }
            _ => {
                match (flag & 0xF0) >> 4 {
                    0xC => {
                        number_value = try!(ziplist.read_i16::<LittleEndian>()) as i64
                    }
                    0xD => {
                        number_value = try!(ziplist.read_i32::<LittleEndian>()) as i64
                    }
                    0xE => {
                        number_value = try!(ziplist.read_i64::<LittleEndian>()) as i64
                    }
                    0xF => {
                        match flag & 0xF {
                            0 => {
                                let mut bytes = [0; 3];
                                match ziplist.read(&mut bytes) {
                                    Ok(3) => (),
                                    Ok(_) => return Err(other_error("Could not read enough \
                                                                     bytes for 24bit number")),
                                    Err(e) => return Err(e),
                                };

                                let number: i32 = (((bytes[2] as i32) << 24) ^
                                                   ((bytes[1] as i32) << 16) ^
                                                   ((bytes[0] as i32) << 8) ^
                                                   48) >>
                                                  8;

                                number_value = number as i64;
                            }
                            0xE => {
                                number_value = try!(ziplist.read_i8()) as i64;
                            }
                            _ => {
                                number_value = (flag & 0xF) as i64 - 1;
                            }
                        }
                    }
                    _ => {
                        panic!("Flag not handled: {}", flag);
                    }

                }

                return Ok(ZiplistEntry::Number(number_value));
            }
        }

        // 3. Read value
        let rawval = try!(read_exact(ziplist, length as usize));
        Ok(ZiplistEntry::String(rawval))
    }

    fn read_ziplist_entry_string<T: Read>(&mut self, reader: &mut T) -> RdbResult<Vec<u8>> {
        let entry = try!(self.read_ziplist_entry(reader));
        match entry {
            ZiplistEntry::String(val) => Ok(val),
            ZiplistEntry::Number(val) => Ok(val.to_string().into_bytes()),
        }
    }

    fn read_zipmap_entry<T: Read>(&mut self, next_byte: u8, zipmap: &mut T) -> RdbResult<Vec<u8>> {
        let elem_len;
        match next_byte {
            253 => {
                elem_len = zipmap.read_u32::<LittleEndian>().unwrap()
            }
            254 | 255 => {
                panic!("Invalid length value in zipmap: {}", next_byte)
            }
            _ => {
                elem_len = next_byte as u32
            }
        }

        read_exact(zipmap, elem_len as usize)
    }

    fn read_quicklist_header(&mut self) -> RdbIteratorResult {
        let len = try!(read_length(&mut self.input));

        self.state = RdbParserState::Quicklist(len, None);
        Ok(ListStart(0))
    }

    fn read_quicklist_element(&mut self,
                              len: u32,
                              opt: Option<(Cursor<Vec<u8>>, u16)>)
                              -> RdbIteratorResult {
        if let Some((cursor, zlen)) = opt {
            return self.read_quicklist_ziplist_element(len, cursor, zlen);
        }

        if len > 0 {
            return self.read_quicklist_ziplist(len);
        }

        self.state = RdbParserState::OpCode;
        return Ok(ListEnd);
    }

    fn read_quicklist_ziplist_element(&mut self,
                                      len: u32,
                                      mut reader: Cursor<Vec<u8>>,
                                      zlen: u16)
                                      -> RdbIteratorResult {
        let entry = try!(self.read_ziplist_entry_string(&mut reader));
        if zlen > 1 {
            self.state = RdbParserState::Quicklist(len, Some((reader, zlen - 1)));
        } else {
            let last_byte = try!(reader.read_u8());
            if last_byte != 0xFF {
                return Err(other_error("Invalid end byte of ziplist (quicklist)"))
            }

            self.state = RdbParserState::Quicklist(len - 1, None);
        }
        Ok(ListElement(entry))
    }

    fn read_quicklist_ziplist(&mut self, len: u32) -> RdbIteratorResult {
        let ziplist = try!(read_blob(&mut self.input));

        let mut reader = Cursor::new(ziplist);
        let (_zlbytes, _zltail, zllen) = try!(read_ziplist_metadata(&mut reader));

        self.read_quicklist_ziplist_element(len, reader, zllen)
    }

    fn read_type(&mut self, value_type: u8) -> RdbIteratorResult {
        match value_type {
            encoding_type::STRING => {
                let val = try!(read_blob(&mut self.input));
                self.state = RdbParserState::OpCode;
                return Ok(Blob(val));
            }
            encoding_type::LIST => {
                return self.read_linked_list_header(Type::List);
            }
            encoding_type::SET => {
                return self.read_linked_list_header(Type::Set);
            }
            encoding_type::ZSET => {
                return self.read_sorted_set_header();
            }
            encoding_type::HASH => {
                return self.read_hash_header();
            }
            encoding_type::HASH_ZIPMAP => {
                return self.read_hash_zipmap_header();
            }
            encoding_type::LIST_ZIPLIST => {
                return self.read_ziplist_header(Type::List);
            }
            encoding_type::SET_INTSET => {
                return self.read_set_intset_header();
            }
            encoding_type::ZSET_ZIPLIST => {
                return self.read_ziplist_header(Type::SortedSet);
            }
            encoding_type::HASH_ZIPLIST => {
                return self.read_ziplist_header(Type::Hash);
            }
            encoding_type::LIST_QUICKLIST => {
                return self.read_quicklist_header();
            }
            _ => {
                panic!("Value Type not implemented: {}", value_type)
            }
        };
    }

    fn read_linked_list_header(&mut self, typ: Type) -> RdbIteratorResult {
        let len = try!(read_length(&mut self.input));

        match typ {
            Type::List => {
                self.state = RdbParserState::List(len);
                return Ok(ListStart(len));
            }
            Type::Set => {
                self.state = RdbParserState::Set(len);
                return Ok(SetStart(len));
            }
            _ => {
                panic!("Unknown encoding type for linked list")
            }
        }
    }

    fn read_linked_list_element(&mut self, typ: Type, len: u32) -> RdbIteratorResult {
        debug_assert!(len > 0);

        let blob = try!(read_blob(&mut self.input));

        match typ {
            Type::List => {
                self.state = RdbParserState::List(len - 1);
                Ok(ListElement(blob))
            }
            Type::Set => {
                self.state = RdbParserState::Set(len - 1);
                Ok(SetElement(blob))
            }
            _ => {
                panic!("Unknown encoding type for linked list")
            }
        }
    }

    fn read_sorted_set_header(&mut self) -> RdbIteratorResult {
        let set_items = unwrap_or_panic!(read_length(&mut self.input));

        self.state = RdbParserState::SortedSet(set_items);
        Ok(SortedSetStart(set_items))
    }

    fn read_sorted_set_element(&mut self, set_items: u32) -> RdbIteratorResult {
        debug_assert!(set_items > 0);

        let val = try!(read_blob(&mut self.input));
        let score_length = try!(self.input.read_u8());
        let score = match score_length {
            253 => {
                f64::NAN
            }
            254 => {
                f64::INFINITY
            }
            255 => {
                f64::NEG_INFINITY
            }
            _ => {
                let tmp = try!(read_exact(&mut self.input, score_length as usize));
                unsafe { str::from_utf8_unchecked(&tmp) }
                    .parse::<f64>()
                    .unwrap()
            }
        };

        self.state = RdbParserState::SortedSet(set_items - 1);
        Ok(SortedSetElement(score, val))
    }

    fn read_hash_header(&mut self) -> RdbIteratorResult {
        let hash_items = try!(read_length(&mut self.input));

        self.state = RdbParserState::Hash(hash_items);
        Ok(HashStart(hash_items))
    }

    fn read_hash_element(&mut self, hash_items: u32) -> RdbIteratorResult {
        debug_assert!(hash_items > 0);

        let field = try!(read_blob(&mut self.input));
        let val = try!(read_blob(&mut self.input));

        self.state = RdbParserState::Hash(hash_items - 1);
        Ok(HashElement(field, val))
    }

    fn read_hash_zipmap_header(&mut self) -> RdbIteratorResult {
        let zipmap = try!(read_blob(&mut self.input));
        let _raw_length = zipmap.len() as u64;

        let mut reader = Cursor::new(zipmap);

        let zmlen = try!(reader.read_u8());

        let length: i32;
        let size;
        if zmlen <= 254 {
            length = zmlen as i32;
            size = zmlen
        } else {
            length = -1;
            size = 0;
        }

        self.state = RdbParserState::Zipmap(reader, length);
        Ok(HashStart(size as u32))
    }

    fn read_hash_zipmap_element(&mut self,
                                mut reader: Cursor<Vec<u8>>,
                                mut length: i32)
                                -> RdbIteratorResult {
        if length == 0 {
            let last_byte = try!(reader.read_u8());

            if last_byte != 0xFF {
                return Err(other_error("Invalid end byte of zipmap"))
            }

            self.state = RdbParserState::OpCode;
            return Ok(HashEnd);
        }

        let next_byte = try!(reader.read_u8());

        if next_byte == 0xFF {
            self.state = RdbParserState::OpCode;
            return Ok(HashEnd);
        }

        let field = try!(self.read_zipmap_entry(next_byte, &mut reader));

        let next_byte = try!(reader.read_u8());
        let _free = try!(reader.read_u8());
        let value = try!(self.read_zipmap_entry(next_byte, &mut reader));

        if length > 0 {
            length -= 1;
            self.state = RdbParserState::Zipmap(reader, length);
        } else {
            self.state = RdbParserState::Zipmap(reader, -1);
        }

        Ok(HashElement(field, value))
    }

    fn read_ziplist_header(&mut self, typ: Type) -> RdbIteratorResult {
        let ziplist = try!(read_blob(&mut self.input));

        let mut reader = Cursor::new(ziplist);
        let (_zlbytes, _zltail, zllen) = try!(read_ziplist_metadata(&mut reader));

        let zllen = zllen as u32;
        match typ {
            Type::List => {
                self.state = RdbParserState::ListZiplist(reader, zllen);
                Ok(ListStart(zllen as u32))
            }
            Type::SortedSet => {
                assert!(zllen % 2 == 0);

                self.state = RdbParserState::SortedSetZiplist(reader, zllen);
                Ok(SortedSetStart(zllen / 2 as u32))
            }
            Type::Hash => {
                assert!(zllen % 2 == 0);

                self.state = RdbParserState::HashZiplist(reader, zllen);
                Ok(HashStart(zllen / 2 as u32))
            }
            _ => {
                panic!("Unknown encoding type for ziplist")
            }
        }
    }

    fn read_list_ziplist_element(&mut self,
                                 mut reader: Cursor<Vec<u8>>,
                                 len: u32)
                                 -> RdbIteratorResult {

        if len > 0 {
            let entry = try!(self.read_ziplist_entry_string(&mut reader));

            self.state = RdbParserState::ListZiplist(reader, len - 1);
            return Ok(ListElement(entry));
        }

        let last_byte = try!(reader.read_u8());
        if last_byte != 0xFF {
            return Err(other_error("Invalid end byte of ziplist"))
        }

        self.state = RdbParserState::OpCode;
        Ok(ListEnd)
    }

    fn read_zset_ziplist_element(&mut self,
                                 mut reader: Cursor<Vec<u8>>,
                                 len: u32)
                                 -> RdbIteratorResult {

        if len > 0 {
            let entry = try!(self.read_ziplist_entry_string(&mut reader));
            let score = try!(self.read_ziplist_entry_string(&mut reader));
            let score = str::from_utf8(&score)
                            .unwrap()
                            .parse::<f64>()
                            .unwrap();

            self.state = RdbParserState::SortedSetZiplist(reader, len - 2);
            return Ok(SortedSetElement(score, entry));
        }

        let last_byte = try!(reader.read_u8());
        if last_byte != 0xFF {
            return Err(other_error("Invalid end byte of ziplist"))
        }

        self.state = RdbParserState::OpCode;
        Ok(SortedSetEnd)
    }

    fn read_hash_ziplist_element(&mut self,
                                 mut reader: Cursor<Vec<u8>>,
                                 len: u32)
                                 -> RdbIteratorResult {

        if len > 0 {
            let field = try!(self.read_ziplist_entry_string(&mut reader));
            let value = try!(self.read_ziplist_entry_string(&mut reader));

            self.state = RdbParserState::HashZiplist(reader, len - 2);
            return Ok(HashElement(field, value));
        }

        let last_byte = try!(reader.read_u8());
        if last_byte != 0xFF {
            return Err(other_error("Invalid end byte of ziplist"))
        }

        self.state = RdbParserState::OpCode;
        Ok(HashEnd)
    }

    fn read_set_intset_header(&mut self) -> RdbIteratorResult {
        let intset = try!(read_blob(&mut self.input));

        let mut reader = Cursor::new(intset);
        let byte_size = try!(reader.read_u32::<LittleEndian>());
        let intset_length = try!(reader.read_u32::<LittleEndian>());

        self.state = RdbParserState::SetIntset(reader, intset_length as u32, byte_size);
        Ok(SetStart(intset_length))
    }

    fn read_set_intset_element(&mut self,
                               mut reader: Cursor<Vec<u8>>,
                               len: u32,
                               byte_size: u32)
                               -> RdbIteratorResult {
        debug_assert!(len > 0);

        let val = match byte_size {
            2 => try!(reader.read_i16::<LittleEndian>()) as i64,
            4 => try!(reader.read_i32::<LittleEndian>()) as i64,
            8 => try!(reader.read_i64::<LittleEndian>()),
            _ => panic!("unhandled byte size in intset: {}", byte_size),
        };

        self.state = RdbParserState::SetIntset(reader, len - 1, byte_size);
        Ok(SetElement(val.to_string().into_bytes()))
    }

    fn skip(&mut self, skip_bytes: usize) -> RdbResult<()> {
        let mut buf = Vec::with_capacity(skip_bytes);
        match self.input.read(&mut buf) {
            Ok(n) if n == skip_bytes => Ok(()),
            Ok(_) => Err(other_error("Can't skip number of requested bytes")),
            Err(e) => Err(e),
        }
    }

    fn skip_blob(&mut self) -> RdbResult<()> {
        let (len, is_encoded) = unwrap_or_panic!(read_length_with_encoding(&mut self.input));
        let skip_bytes;

        if is_encoded {
            skip_bytes = match len {
                encoding::INT8 => 1,
                encoding::INT16 => 2,
                encoding::INT32 => 4,
                encoding::LZF => {
                    let compressed_length = unwrap_or_panic!(read_length(&mut self.input));
                    let _real_length = unwrap_or_panic!(read_length(&mut self.input));
                    compressed_length
                }
                _ => {
                    panic!("Unknown encoding: {}", len)
                }
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
            encoding_type::LIST | encoding_type::SET =>
                unwrap_or_panic!(read_length(&mut self.input)),
            encoding_type::ZSET | encoding_type::HASH =>
                unwrap_or_panic!(read_length(&mut self.input)) * 2,
            _ => {
                panic!("Unknown encoding type: {}", enc_type)
            }
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
