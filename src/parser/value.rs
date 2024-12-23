use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum RdbValue {
    SelectDb(u32),
    ResizeDb {
        db_size: u32,
        expires_size: u32,
    },
    AuxField {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Checksum(Vec<u8>),
    String {
        key: Vec<u8>,
        value: Vec<u8>,
        expiry: Option<u64>,
    },
    Hash {
        key: Vec<u8>,
        values: HashMap<Vec<u8>, Vec<u8>>,
        expiry: Option<u64>,
    },
    Set {
        key: Vec<u8>,
        members: HashSet<Vec<u8>>,
        expiry: Option<u64>,
    },
    List {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        expiry: Option<u64>,
    },
    SortedSet {
        key: Vec<u8>,
        values: Vec<(f64, Vec<u8>)>, // (score, member)
        expiry: Option<u64>,
    },
}
