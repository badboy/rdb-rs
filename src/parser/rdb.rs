use super::common::utils::{
    read_blob, read_length, read_length_with_encoding, verify_magic, verify_version,
};
use super::{hash, list, set};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Read;

use super::value::RdbValue;
use crate::constants::{encoding, encoding_type, op_code};
use crate::filter::Filter;
use crate::types::RdbResult;

pub struct RdbParser<R: Read, L: Filter> {
    input: R,
    filter: L,
    last_expiretime: Option<u64>,
    current_database: u32,
}

impl<R: Read, L: Filter> RdbParser<R, L> {
    pub fn new(input: R, filter: L) -> RdbParser<R, L> {
        RdbParser {
            input,
            filter,
            last_expiretime: None,
            current_database: 0,
        }
    }

    /// Verifies the RDB file header (magic number and version).
    /// This should be called before starting iteration.
    pub fn verify_header(&mut self) -> RdbResult<()> {
        verify_magic(&mut self.input)?;
        verify_version(&mut self.input)
    }

    fn read_type(&mut self, key: &[u8], value_type: u8) -> RdbResult<RdbValue> {
        match value_type {
            encoding_type::STRING => {
                let val = read_blob(&mut self.input)?;
                Ok(RdbValue::String {
                    key: key.to_vec(),
                    value: val,
                    expiry: self.last_expiretime,
                })
            }
            encoding_type::LIST => {
                list::read_linked_list(&mut self.input, key, self.last_expiretime)
            }
            encoding_type::SET => set::read_set(&mut self.input, key, self.last_expiretime),
            encoding_type::ZSET => set::read_sorted_set(&mut self.input, key, self.last_expiretime),
            encoding_type::HASH => hash::read_hash(&mut self.input, key, self.last_expiretime),
            encoding_type::HASH_ZIPMAP => {
                hash::read_hash_zipmap(&mut self.input, key, self.last_expiretime)
            }
            encoding_type::LIST_ZIPLIST => {
                list::read_list_ziplist(&mut self.input, key, self.last_expiretime)
            }
            encoding_type::SET_INTSET => {
                set::read_set_intset(&mut self.input, key, self.last_expiretime)
            }
            encoding_type::ZSET_ZIPLIST => {
                set::read_sortedset_ziplist(&mut self.input, key, self.last_expiretime)
            }
            encoding_type::HASH_ZIPLIST => {
                hash::read_hash_ziplist(&mut self.input, key, self.last_expiretime)
            }
            encoding_type::LIST_QUICKLIST => {
                list::read_quicklist(&mut self.input, key, self.last_expiretime)
            }
            encoding_type::HASH_LIST_PACK => {
                hash::read_hash_list_pack(&mut self.input, key, self.last_expiretime)
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
            _ => {
                panic!("Value Type not implemented: {}", value_type)
            }
        }
    }

    fn skip(&mut self, skip_bytes: usize) -> RdbResult<()> {
        let mut buf = vec![0; skip_bytes];
        self.input.read_exact(&mut buf)
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
            encoding_type::STRING
            | encoding_type::HASH_ZIPMAP
            | encoding_type::LIST_ZIPLIST
            | encoding_type::SET_INTSET
            | encoding_type::ZSET_ZIPLIST
            | encoding_type::HASH_ZIPLIST => 1,
            encoding_type::LIST | encoding_type::SET | encoding_type::LIST_QUICKLIST => {
                read_length(&mut self.input)?
            }
            encoding_type::ZSET | encoding_type::HASH => read_length(&mut self.input)? * 2,
            _ => {
                panic!("Unknown encoding type: {}", enc_type)
            }
        };

        for _ in 0..blobs_to_skip {
            self.skip_blob()?
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
        let next_op = match self.input.read_u8() {
            Ok(op) => op,
            Err(_) => return None,
        };

        let result = match next_op {
            op_code::SELECTDB => match read_length(&mut self.input) {
                Ok(db_index) => {
                    self.current_database = db_index;
                    Some(Ok(RdbValue::SelectDb(db_index)))
                }
                Err(e) => Some(Err(e)),
            },
            op_code::EOF => {
                let mut checksum = Vec::new();
                match self.input.read_to_end(&mut checksum) {
                    Ok(_) => {
                        if !checksum.is_empty() {
                            Some(Ok(RdbValue::Checksum(checksum)))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            }
            op_code::EXPIRETIME_MS => match self.input.read_u64::<LittleEndian>() {
                Ok(expiretime_ms) => {
                    self.last_expiretime = Some(expiretime_ms);
                    self.next()
                }
                Err(e) => Some(Err(e)),
            },
            op_code::EXPIRETIME => match self.input.read_u32::<BigEndian>() {
                Ok(expiretime) => {
                    self.last_expiretime = Some(expiretime as u64 * 1000);
                    self.next()
                }
                Err(e) => Some(Err(e)),
            },
            op_code::RESIZEDB => {
                match (read_length(&mut self.input), read_length(&mut self.input)) {
                    (Ok(db_size), Ok(expires_size)) => Some(Ok(RdbValue::ResizeDb {
                        db_size,
                        expires_size,
                    })),
                    (Err(e), _) | (_, Err(e)) => Some(Err(e)),
                }
            }
            op_code::AUX => match (read_blob(&mut self.input), read_blob(&mut self.input)) {
                (Ok(auxkey), Ok(auxval)) => Some(Ok(RdbValue::AuxField {
                    key: auxkey,
                    value: auxval,
                })),
                (Err(e), _) | (_, Err(e)) => Some(Err(e)),
            },
            op_code::MODULE_AUX => match self.skip_blob() {
                Ok(_) => self.next(),
                Err(e) => Some(Err(e)),
            },
            op_code::IDLE => match read_length(&mut self.input) {
                Ok(_idle_time) => self.next(),
                Err(e) => Some(Err(e)),
            },
            op_code::FREQ => match self.input.read_u8() {
                Ok(_freq) => self.next(),
                Err(e) => Some(Err(e)),
            },
            _ => {
                if self.filter.matches_db(self.current_database) {
                    match read_blob(&mut self.input) {
                        Ok(key) => {
                            if self.filter.matches_type(next_op) && self.filter.matches_key(&key) {
                                match self.read_type(&key, next_op) {
                                    Ok(value) => Some(Ok(value)),
                                    Err(e) => Some(Err(e)),
                                }
                            } else {
                                match self.skip_object(next_op) {
                                    Ok(_) => self.next(),
                                    Err(e) => Some(Err(e)),
                                }
                            }
                        }
                        Err(e) => Some(Err(e)),
                    }
                } else {
                    match self.skip_key_and_object(next_op) {
                        Ok(_) => self.next(),
                        Err(e) => Some(Err(e)),
                    }
                }
            }
        };

        result
    }
}
