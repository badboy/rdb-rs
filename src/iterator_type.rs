#[derive(Debug,PartialEq,Clone)]
pub enum RdbIteratorType<'a> {
    Value,
    Skipped,
    EOF,

    StartDatabase(u32),
    EndDatabase(u32),
    Ended,

    Key(&'a[u8], u64), // (name, expiry)

    // Blobs. Can sometimes be a 64bit int
    Blob(&'a[u8]),
    Int(u64),

    // Lists
    ListStart(u64), // includes (expected?) size
    ListEnd(u64), // includes real size (?)
    ListElement(&'a[u8]), // Always byte array?

    // Sets
    SetStart(u64),
    SetEnd,
    SetElement(&'a[u8]),

    // Sorted Sets
    SortedSetStart(u64),
    SortedSetEnd,
    SortedSetElement(u64, &'a[u8]), // (score, member)

    // Hashes
    StartHash(u32), // length
    EndHash(u32),
    HashElement(&'a[u8], &'a[u8]), // (field, value)
}
