use formatter::Formatter;

/// Do not output anything
pub struct Nil;

impl Nil {
    pub fn new() -> Nil {
        Nil
    }
}

impl Formatter for Nil {}
