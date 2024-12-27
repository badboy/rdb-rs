use super::common::utils::{
    read_blob, read_length, read_length_with_encoding, verify_magic, verify_version,
};
use super::{hash, list, set};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Read;

use crate::constants::{encoding, encoding_type, op_code};
use crate::filter::Filter;
use crate::types::{RdbError, RdbValue};


pub type RdbResult<T> = Result<T, RdbError>;

pub struct RdbParser<R: Read, L: Filter> {
    input: R,
    filter: L,
    last_expiretime: Option<u64>,
    current_database: u32,
    reached_eof: bool,
}

impl<R: Read, L: Filter> RdbParser<R, L> {
    pub fn new(input: R, filter: L) -> RdbResult<RdbParser<R, L>> {
        let mut parser = RdbParser {
            input,
            filter,
            last_expiretime: None,
            current_database: 0,
            reached_eof: false,
        };
        
        parser.verify_header()?;
        Ok(parser)
    }

    /// Verifies the RDB file header (magic number and version).
    fn verify_header(&mut self) -> RdbResult<()> {
        verify_magic(&mut self.input)?;
        verify_version(&mut self.input)
    }

    fn read_type(&mut self, key: &[u8], value_type: u8) -> RdbResult<RdbValue> {
        let result = match value_type {
            encoding_type::STRING => {
                let value = read_blob(&mut self.input)?;
                RdbValue::String {
                    key: key.to_vec(),
                    value,
                    expiry: self.last_expiretime,
                }
            }
            encoding_type::LIST => list::read_linked_list(&mut self.input, key, self.last_expiretime)?,
            encoding_type::SET => set::read_set(&mut self.input, key, self.last_expiretime)?,
            encoding_type::ZSET => set::read_sorted_set(&mut self.input, key, self.last_expiretime)?,
            encoding_type::HASH => hash::read_hash(&mut self.input, key, self.last_expiretime)?,
            encoding_type::HASH_ZIPMAP => {
                hash::read_hash_zipmap(&mut self.input, key, self.last_expiretime)?
            }
            encoding_type::LIST_ZIPLIST => {
                list::read_list_ziplist(&mut self.input, key, self.last_expiretime)?
            }
            encoding_type::SET_INTSET => {
                set::read_set_intset(&mut self.input, key, self.last_expiretime)?
            }
            encoding_type::ZSET_ZIPLIST => {
                set::read_sortedset_ziplist(&mut self.input, key, self.last_expiretime)?
            }
            encoding_type::HASH_ZIPLIST => {
                hash::read_hash_ziplist(&mut self.input, key, self.last_expiretime)?
            }
            encoding_type::LIST_QUICKLIST => {
                list::read_quicklist(&mut self.input, key, self.last_expiretime)?
            }
            encoding_type::HASH_LIST_PACK => {
                hash::read_hash_list_pack(&mut self.input, key, self.last_expiretime)?
            }
            encoding_type::ZSET_2 => {
                todo!("read_zset_2 not implemented");
            }
            encoding_type::LIST_QUICKLIST_2 => {
                todo!("read_quicklist_2 not implemented");
            }
            encoding_type::STREAM_LIST_PACKS => {
                todo!("read_stream_list_packs v1 not implemented");
            }
            encoding_type::STREAM_LIST_PACKS_2 => {
                todo!("read_stream_list_packs v2 not implemented");
            }
            encoding_type::STREAM_LIST_PACKS_3 => {
                todo!("read_stream_list_packs v3 not implemented");
            }
            encoding_type::ZSET_LIST_PACK => {
                todo!("read_zset_list_pack not implemented");
            }
            encoding_type::SET_LIST_PACK => {
                todo!("read_set_list_pack not implemented");
            }
            unknown_type => {
                self.skip_object(unknown_type)?;
                return self.next().and_then(Result::ok).ok_or(RdbError::MissingValue("skip"));
            }
        };
        Ok(result)
    }

    fn skip(&mut self, skip_bytes: usize) -> RdbResult<()> {
        let mut buf = vec![0; skip_bytes];
        self.input.read_exact(&mut buf).map_err(|e| RdbError::Io(e))?;
        Ok(())
    }

    fn skip_blob(&mut self) -> RdbResult<()> {
        let (len, is_encoded) = read_length_with_encoding(&mut self.input)?;
        let skip_bytes;

        if is_encoded {
            skip_bytes = match len {
                encoding::INT8 => 1,
                encoding::INT16 => 2,
                encoding::INT32 => 4,
                encoding::LZF => {
                    let compressed_length = read_length(&mut self.input)?;
                    let _real_length = read_length(&mut self.input)?;
                    compressed_length
                }
                _ => {
                    return Err(RdbError::UnknownEncodingValue(len as u64));
                }
            }
        } else {
            skip_bytes = len;
        }

        self.skip(skip_bytes as usize)
    }

    fn skip_object(&mut self, enc_type: u8) -> RdbResult<()> {
        let blobs_count = match enc_type {
            encoding_type::STRING | encoding_type::HASH_ZIPMAP | encoding_type::LIST_ZIPLIST 
            | encoding_type::SET_INTSET | encoding_type::ZSET_ZIPLIST 
            | encoding_type::HASH_ZIPLIST | encoding_type::HASH_LIST_PACK => 1,
            encoding_type::LIST | encoding_type::SET | encoding_type::LIST_QUICKLIST => {
                read_length(&mut self.input)?
            }
            encoding_type::ZSET | encoding_type::HASH => read_length(&mut self.input)? * 2,
            _ => return Err(RdbError::UnknownEncoding(enc_type))
        };

        for _ in 0..blobs_count {
            self.skip_blob()?;
        }
        Ok(())
    }

    fn skip_key_and_object(&mut self, enc_type: u8) -> RdbResult<()> {
        self.skip_blob()?;
        self.skip_object(enc_type)?;
        Ok(())
    }
}

impl<R: Read, L: Filter> Iterator for RdbParser<R, L> {
    type Item = RdbResult<RdbValue>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reached_eof {
            return None;
        }

        Some(self.process_next_operation())
    }
}

impl<R: Read, L: Filter> RdbParser<R, L> {
    fn process_next_operation(&mut self) -> RdbResult<RdbValue> {
        let next_op = match self.input.read_u8() {
            Ok(op) => op,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(RdbValue::Checksum(vec![]))
            }
            Err(e) => return Err(e.into()),
        };

        match next_op {
            op_code::SELECTDB => {
                self.current_database = read_length(&mut self.input)?;
                Ok(RdbValue::SelectDb(self.current_database))
            }
            op_code::EOF => {
                let mut checksum = Vec::new();
                self.input.read_to_end(&mut checksum)?;
                self.reached_eof = true;
                Ok(RdbValue::Checksum(checksum))
            }
            op_code::EXPIRETIME_MS => {
                self.last_expiretime = Some(self.input.read_u64::<LittleEndian>()?);
                self.next().ok_or_else(|| RdbError::MissingValue("expiry"))?
            }
            op_code::EXPIRETIME => {
                self.last_expiretime = Some(self.input.read_u32::<BigEndian>()? as u64 * 1000);
                self.next().ok_or_else(|| RdbError::MissingValue("expiry"))?
            }
            op_code::RESIZEDB => {
                let db_size = read_length(&mut self.input)?;
                let expires_size = read_length(&mut self.input)?;
                Ok(RdbValue::ResizeDb {
                    db_size,
                    expires_size,
                })
            }
            op_code::AUX => {
                let key = read_blob(&mut self.input)?;
                let value = read_blob(&mut self.input)?;
                Ok(RdbValue::AuxField { key, value })
            }
            op_code::MODULE_AUX => {
                self.skip_blob()?;
                self.next().ok_or_else(|| RdbError::MissingValue("module aux"))?
            }
            op_code::IDLE => {
                let _idle_time = read_length(&mut self.input)?;
                self.next().ok_or_else(|| RdbError::MissingValue("idle"))?
            }
            op_code::FREQ => {
                let _freq = self.input.read_u8()?;
                self.next().ok_or_else(|| RdbError::MissingValue("freq"))?
            }
            value_type => self.handle_value_type(value_type),
        }
    }

    fn handle_value_type(&mut self, value_type: u8) -> RdbResult<RdbValue> {
        if !self.filter.matches_db(self.current_database) {
            self.skip_key_and_object(value_type)?;
            return Ok(RdbValue::SelectDb(self.current_database));
        }

        let key = read_blob(&mut self.input)?;
        if !self.filter.matches_type(value_type) || !self.filter.matches_key(&key) {
            self.skip_object(value_type)?;
            return Ok(RdbValue::SelectDb(self.current_database));
        }

        self.read_type(&key, value_type)
    }
}
