use formatter::RdbParseFormatter;

#[derive(Copy)]
pub struct NilFormatter;

impl NilFormatter {
    pub fn new() -> NilFormatter {
        NilFormatter
    }
}

impl RdbParseFormatter for NilFormatter {}
