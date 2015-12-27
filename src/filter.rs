//! Filter trait and implementations to skip items in the parser

use std::str;
use regex::Regex;
use types::Type;

/// A trait to decied to skip databases, types or keys
pub trait Filter {
    fn matches_db(&self, _db: u32) -> bool { true }
    fn matches_type(&self, _enc_type: u8) -> bool { true }
    fn matches_key(&self, _key: &[u8]) -> bool { true }
}

/// A filter to match by database, type or a regular expression against key names
pub struct Simple {
    databases: Vec<u32>,
    types: Vec<Type>,
    keys: Option<Regex>,
}

impl Simple {
    pub fn new() -> Simple {
        Simple {
            databases: vec![],
            types: vec![],
            keys: None,
        }
    }

    pub fn add_database(&mut self, db: u32) {
        self.databases.push(db);
    }

    pub fn add_type(&mut self, typ: Type) {
        self.types.push(typ);
    }

    pub fn add_keys(&mut self, re: Regex) {
        self.keys = Some(re);
    }
}

impl Filter for Simple {
    fn matches_db(&self, db: u32) -> bool {
        if self.databases.is_empty() {
            true
        } else {
            self.databases.iter().any(|&x| x == db)
        }
    }

    fn matches_type(&self, enc_type: u8) -> bool {
        if self.types.is_empty() {
            return true
        }

        let typ = Type::from_encoding(enc_type);
        self.types.iter().any(|x| *x == typ)
    }

    fn matches_key(&self, key: &[u8]) -> bool {
        match self.keys.clone() {
            None => true,
            Some(re) => {
                let key = unsafe { str::from_utf8_unchecked(key) };
                re.is_match(key)
            }
        }
    }
}
