#[derive(Debug,PartialEq,Clone)]
pub enum RdbIteratorType {
    Value,
    Skipped,
    Failed,
    EOF,
    RdbEnd,
    Checksum(Vec<u8>),

    ResizeDB(u32, u32),
    AuxiliaryKey(Vec<u8>, Vec<u8>),

    StartDatabase(u32),
    EndDatabase(u32),
    Ended,

    Key(Vec<u8>, Option<u64>), // (name, expiry)

    // Blobs. Can sometimes be a 64bit int
    Blob(Vec<u8>),
    Int(u64),

    // Lists
    ListStart(u32), // includes (expected?) size
    ListEnd, // includes real size (?)
    ListElement(Vec<u8>), // Always byte array?

    // Sets
    SetStart(u32),
    SetEnd,
    SetElement(Vec<u8>),

    // Sorted Sets
    SortedSetStart(u32),
    SortedSetEnd,
    SortedSetElement(f64, Vec<u8>), // (score, member)

    // Hashes
    HashStart(u32), // length
    HashEnd,
    HashElement(Vec<u8>, Vec<u8>), // (field, value)
}
