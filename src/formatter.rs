pub trait RdbParseFormatter {
    fn start_rdb(&mut self);
    fn end_rdb(&mut self);
    fn checksum(&mut self, Vec<u8>);

    fn start_database(&mut self, u32);
    fn end_database(&mut self, u32) {}

    fn set(&mut self, Vec<u8>, Vec<u8>);

    fn aux_field(&mut self, _key: Vec<u8>, _value: Vec<u8>) {}
}
