#[derive(Debug,PartialEq,Clone)]
pub enum RdbIteratorType {
    /// A value was skipped
    Skipped,
    /// Failed to parse the next item
    Failed,
    /// End of the file reached
    EOF,
    /// RDB end marker
    RdbEnd,
    /// Checksum in the file
    Checksum(Vec<u8>),

    /// ResizeDB tag (size of hash table, size of expire hash table)
    ResizeDB(u32, u32),
    /// Auxiliary tag (arbitrary key-value pair)
    AuxiliaryKey(Vec<u8>, Vec<u8>),

    /// Start of a database
    StartDatabase(u32),

    /// A key (name and optional expire time)
    Key(Vec<u8>, Option<u64>), // (name, expiry)

    /// Binary blobs (Can sometimes be a 64-bit int)
    Blob(Vec<u8>),
    /// A 64-bit unsigned integer
    Int(u64),

    /// Start of a list (with expected size)
    ListStart(u32),
    /// End of a list
    ListEnd, // includes real size (?)
    /// A list element
    ListElement(Vec<u8>), // Always byte array?

    /// Start of a set (with expected size)
    SetStart(u32),
    /// End of a set
    SetEnd,
    /// A set element
    SetElement(Vec<u8>),

    /// Start of a sorted set (with expected size)
    SortedSetStart(u32),
    /// End of a sorted set
    SortedSetEnd,
    /// A sorted set element
    SortedSetElement(f64, Vec<u8>), // (score, member)

    /// Start of a hash (with number of items)
    HashStart(u32), // length
    /// End of a hash
    HashEnd,
    /// A hash element (a field-value pair)
    HashElement(Vec<u8>, Vec<u8>), // (field, value)
}
