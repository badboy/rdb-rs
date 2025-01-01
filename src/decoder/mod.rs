mod common;
mod hash;
mod list;
mod rdb;
mod set;
mod sorted_set;

use std::io::Read;

use self::rdb::DecoderState;
use crate::filter::Filter;
use crate::types::{RdbResult, RdbValue};

pub struct RdbDecoder<R: Read, F: Filter> {
    reader: R,
    filter: F,
    state: DecoderState,
}

impl<R: Read, F: Filter> RdbDecoder<R, F> {
    pub(crate) fn new(mut reader: R, filter: F) -> RdbResult<Self> {
        rdb::verify_header(&mut reader)?;
        Ok(Self {
            reader,
            filter,
            state: DecoderState::default(),
        })
    }
}

impl<R: Read, F: Filter> Iterator for RdbDecoder<R, F> {
    type Item = RdbResult<RdbValue>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state.reached_eof {
            return None;
        }
        Some(rdb::process_next_operation(
            &mut self.reader,
            &self.filter,
            &mut self.state,
        ))
    }
}
