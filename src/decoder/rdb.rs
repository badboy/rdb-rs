use super::common::utils::{
    read_blob, read_length, read_length_with_encoding, verify_magic, verify_version,
};
use super::{hash, list, set, sorted_set};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Read;

use crate::constants::{encoding, encoding_type, op_code};
use crate::filter::Filter;
use crate::types::{RdbError, RdbResult, RdbValue};

#[derive(Default)]
pub(crate) struct DecoderState {
    pub last_expiretime: Option<u64>,
    pub current_database: u32,
    pub reached_eof: bool,
}

pub(crate) fn verify_header<R: Read>(input: &mut R) -> RdbResult<()> {
    verify_magic(input)?;
    verify_version(input)
}

pub(crate) fn read_type<R: Read>(
    input: &mut R,
    key: &[u8],
    value_type: u8,
    expiry: Option<u64>,
) -> RdbResult<RdbValue> {
    let result = match value_type {
        encoding_type::STRING => {
            let value = read_blob(input)?;
            RdbValue::String {
                key: key.to_vec(),
                value,
                expiry,
            }
        }
        encoding_type::LIST => list::read_linked_list(input, key, expiry)?,
        encoding_type::SET => set::read_set(input, key, expiry)?,
        encoding_type::ZSET => sorted_set::read_sorted_set(input, key, expiry, false)?,
        encoding_type::HASH => hash::read_hash(input, key, expiry)?,
        encoding_type::HASH_ZIPMAP => hash::read_hash_zipmap(input, key, expiry)?,
        encoding_type::LIST_ZIPLIST => list::read_list_ziplist(input, key, expiry)?,
        encoding_type::SET_INTSET => set::read_set_intset(input, key, expiry)?,
        encoding_type::ZSET_ZIPLIST => sorted_set::read_sorted_set_ziplist(input, key, expiry)?,
        encoding_type::HASH_ZIPLIST => hash::read_hash_ziplist(input, key, expiry)?,
        encoding_type::LIST_QUICKLIST => list::read_quicklist(input, key, expiry)?,
        encoding_type::HASH_LIST_PACK => hash::read_hash_list_pack(input, key, expiry)?,
        encoding_type::ZSET_2 => sorted_set::read_sorted_set(input, key, expiry, true)?,
        encoding_type::LIST_QUICKLIST_2 => list::read_quicklist_2(input, key, expiry)?,
        encoding_type::STREAM_LIST_PACKS => {
            todo!("read_stream_list_packs v1 not implemented");
        }
        encoding_type::STREAM_LIST_PACKS_2 => {
            todo!("read_stream_list_packs v2 not implemented");
        }
        encoding_type::STREAM_LIST_PACKS_3 => {
            todo!("read_stream_list_packs v3 not implemented");
        }
        encoding_type::ZSET_LIST_PACK => sorted_set::read_sorted_set_listpack(input, key, expiry)?,
        encoding_type::SET_LIST_PACK => set::read_set_list_pack(input, key, expiry)?,
        unknown_type => {
            log::debug!("Skipping unknown encoding type: {}", unknown_type);
            skip_object(input, unknown_type)?;
            return Err(RdbError::MissingValue("skip"));
        }
    };
    Ok(result)
}

pub(crate) fn skip<R: Read>(input: &mut R, skip_bytes: usize) -> RdbResult<()> {
    let mut buf = vec![0; skip_bytes];
    input.read_exact(&mut buf).map_err(|e| RdbError::Io(e))?;
    Ok(())
}

pub(crate) fn skip_blob<R: Read>(input: &mut R) -> RdbResult<()> {
    let (len, is_encoded) = read_length_with_encoding(input)?;
    let skip_bytes;

    if is_encoded {
        skip_bytes = match len {
            encoding::INT8 => 1,
            encoding::INT16 => 2,
            encoding::INT32 => 4,
            encoding::LZF => {
                let compressed_length = read_length(input)?;
                let _real_length = read_length(input)?;
                compressed_length
            }
            _ => {
                return Err(RdbError::ParsingError {
                    context: "skip_blob",
                    message: format!("Unknown encoding value: {}", len),
                });
            }
        }
    } else {
        skip_bytes = len;
    }

    skip(input, skip_bytes as usize)
}

pub(crate) fn skip_object<R: Read>(input: &mut R, enc_type: u8) -> RdbResult<()> {
    let blobs_count = match enc_type {
        encoding_type::STRING
        | encoding_type::HASH_ZIPMAP
        | encoding_type::LIST_ZIPLIST
        | encoding_type::SET_INTSET
        | encoding_type::ZSET_ZIPLIST
        | encoding_type::HASH_ZIPLIST
        | encoding_type::HASH_LIST_PACK => 1,
        encoding_type::LIST | encoding_type::SET | encoding_type::LIST_QUICKLIST => {
            read_length(input)?
        }
        encoding_type::ZSET | encoding_type::HASH => read_length(input)? * 2,
        _ => return Err(RdbError::UnknownEncoding(enc_type)),
    };

    for _ in 0..blobs_count {
        skip_blob(input)?;
    }
    Ok(())
}

pub(crate) fn process_next_operation<R: Read, F: Filter>(
    input: &mut R,
    filter: &F,
    state: &mut DecoderState,
) -> RdbResult<RdbValue> {
    let next_op = match input.read_u8() {
        Ok(op) => op,
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Ok(RdbValue::Checksum(vec![]))
        }
        Err(e) => return Err(e.into()),
    };

    match next_op {
        op_code::SELECTDB => {
            state.current_database = read_length(input)?;
            Ok(RdbValue::SelectDb(state.current_database))
        }
        op_code::EOF => {
            let mut checksum = Vec::new();
            input.read_to_end(&mut checksum)?;
            state.reached_eof = true;
            Ok(RdbValue::Checksum(checksum))
        }
        op_code::EXPIRETIME_MS => {
            state.last_expiretime = Some(input.read_u64::<LittleEndian>()?);
            process_next_operation(input, filter, state)
        }
        op_code::EXPIRETIME => {
            state.last_expiretime = Some(input.read_u32::<BigEndian>()? as u64 * 1000);
            process_next_operation(input, filter, state)
        }
        op_code::RESIZEDB => {
            let db_size = read_length(input)?;
            let expires_size = read_length(input)?;
            Ok(RdbValue::ResizeDb {
                db_size,
                expires_size,
            })
        }
        op_code::AUX => {
            let key = read_blob(input)?;
            let value = read_blob(input)?;
            Ok(RdbValue::AuxField { key, value })
        }
        op_code::MODULE_AUX => {
            skip_blob(input)?;
            process_next_operation(input, filter, state)
        }
        op_code::IDLE => {
            let _idle_time = read_length(input)?;
            process_next_operation(input, filter, state)
        }
        op_code::FREQ => {
            let _freq = input.read_u8()?;
            process_next_operation(input, filter, state)
        }
        value_type => {
            if !filter.matches_db(state.current_database) {
                skip_object(input, value_type)?;
                return Ok(RdbValue::SelectDb(state.current_database));
            }

            let key = read_blob(input)?;
            if !filter.matches_type(value_type) || !filter.matches_key(&key) {
                skip_object(input, value_type)?;
                return Ok(RdbValue::SelectDb(state.current_database));
            }

            read_type(input, &key, value_type, state.last_expiretime)
        }
    }
}
