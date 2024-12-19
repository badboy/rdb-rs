use crate::formatter::Formatter;

pub struct Nil;

impl Nil {
    pub fn new() -> Nil {
        Nil
    }
}

impl Formatter for Nil {}
