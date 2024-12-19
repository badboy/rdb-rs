use super::common::utils::{
    read_blob, read_length, read_length_with_encoding, verify_magic, verify_version,
};
use super::{hash, list, set};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Read;

use crate::filter::Filter;
use crate::formatter::Formatter;

#[doc(hidden)]
use crate::constants::{encoding, encoding_type, op_code};

#[doc(hidden)]
pub use crate::types::{RdbOk, RdbResult, Type};

pub struct RdbParser<R: Read, F: Formatter, L: Filter> {
    input: R,
    formatter: F,
    filter: L,
    last_expiretime: Option<u64>,
}

impl<R: Read, F: Formatter, L: Filter> RdbParser<R, F, L> {
    pub fn new(input: R, formatter: F, filter: L) -> RdbParser<R, F, L> {
        RdbParser {
            input: input,
            formatter: formatter,
            filter: filter,
            last_expiretime: None,
        }
    }

    pub fn parse(&mut self) -> RdbOk {
        verify_magic(&mut self.input)?;
        verify_version(&mut self.input)?;

        self.formatter.start_rdb();

        let mut last_database: u32 = 0;
        loop {
            let next_op = self.input.read_u8()?;

            match next_op {
                op_code::SELECTDB => {
                    last_database = read_length(&mut self.input)?;
                    if self.filter.matches_db(last_database) {
                        self.formatter.start_database(last_database);
                    }
                }
                op_code::EOF => {
                    self.formatter.end_database(last_database);
                    self.formatter.end_rdb();

                    let mut checksum = Vec::new();
                    let len = self.input.read_to_end(&mut checksum)?;
                    if len > 0 {
                        self.formatter.checksum(&checksum);
                    }
                    break;
                }
                op_code::EXPIRETIME_MS => {
                    let expiretime_ms = self.input.read_u64::<LittleEndian>()?;
                    self.last_expiretime = Some(expiretime_ms);
                }
                op_code::EXPIRETIME => {
                    let expiretime = self.input.read_u32::<BigEndian>()?;
                    self.last_expiretime = Some(expiretime as u64 * 1000);
                }
                op_code::RESIZEDB => {
                    let db_size = read_length(&mut self.input)?;
                    let expires_size = read_length(&mut self.input)?;

                    self.formatter.resizedb(db_size, expires_size);
                }
                op_code::AUX => {
                    let auxkey = read_blob(&mut self.input)?;
                    let auxval = read_blob(&mut self.input)?;

                    self.formatter.aux_field(&auxkey, &auxval);
                }
                op_code::MODULE_AUX => {
                    // TODO: Implement module auxiliary data parsing
                    // Parse the module-specific data if a handler is registered.
                    // For now, skip the data.
                    self.skip_blob()?; // Skip module auxiliary data blob
                }
                op_code::IDLE => {
                    let _idle_time = read_length(&mut self.input)?;
                }
                op_code::FREQ => {
                    let _freq = self.input.read_u8()?;
                }
                _ => {
                    if self.filter.matches_db(last_database) {
                        let key = read_blob(&mut self.input)?;

                        if self.filter.matches_type(next_op) && self.filter.matches_key(&key) {
                            self.read_type(&key, next_op)?;
                        } else {
                            self.skip_object(next_op)?;
                        }
                    } else {
                        self.skip_key_and_object(next_op)?;
                    }

                    self.last_expiretime = None;
                }
            }
        }

        Ok(())
    }

    fn read_type(&mut self, key: &[u8], value_type: u8) -> RdbOk {
        match value_type {
            encoding_type::STRING => {
                let val = read_blob(&mut self.input)?;
                self.formatter.set(key, &val, self.last_expiretime);
            }
            encoding_type::LIST => list::read_linked_list(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
                Type::List,
            )?,
            encoding_type::SET => list::read_linked_list(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
                Type::Set,
            )?,
            encoding_type::ZSET => set::read_sorted_set(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::HASH => hash::read_hash(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::HASH_ZIPMAP => hash::read_hash_zipmap(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::LIST_ZIPLIST => list::read_list_ziplist(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::SET_INTSET => set::read_set_intset(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::ZSET_ZIPLIST => set::read_sortedset_ziplist(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::HASH_ZIPLIST => hash::read_hash_ziplist(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::LIST_QUICKLIST => list::read_quicklist(
                &mut self.input,
                &mut self.formatter,
                key,
                self.last_expiretime,
            )?,
            encoding_type::ZSET_2 => {
                todo!(); //self.read_zset_2(key)?;
            }
            encoding_type::LIST_QUICKLIST_2 => {
                todo!(); //self.read_quicklist_2(key)?;
            }
            encoding_type::STREAM_LIST_PACKS => {
                todo!(); //self.read_stream_list_packs(key, 1)?;
            }
            encoding_type::STREAM_LIST_PACKS_2 => {
                todo!(); //self.read_stream_list_packs(key, 2)?;
            }
            encoding_type::STREAM_LIST_PACKS_3 => {
                todo!(); //self.read_stream_list_packs(key, 3)?;
            }
            encoding_type::HASH_LIST_PACK => {
                todo!(); //self.read_hash_list_pack(key)?;
            }
            encoding_type::ZSET_LIST_PACK => {
                todo!(); //self.read_zset_list_pack(key)?;
            }
            encoding_type::SET_LIST_PACK => {
                todo!(); //self.read_set_list_pack(key)?;
            }
            _ => {
                panic!("Value Type not implemented: {}", value_type)
            }
        };

        Ok(())
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
