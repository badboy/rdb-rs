#![macro_use]

macro_rules! ensure {
    ($expr:expr, $err_result:expr) => (
        if !($expr) { return $err_result; }
    )
}

macro_rules! fail {
    ($expr:expr) => (
        return Err(::std::error::FromError::from_error($expr));
    )
}

macro_rules! unwrap_or {
    ($expr:expr, $or:expr) => (
        match $expr {
            Some(x) => x,
            None => { $or; }
        }
    )
}

macro_rules! unwrap_or_panic {
    ($expr:expr) => (match $expr {
        Ok(val) => val,
        Err(err) => panic!("Error: {:?}", err)
    })
}


macro_rules! try_or_ok {
    ($expr:expr) => (match $expr {
        Ok(_) => Ok(()),
        e => e
    })
}

macro_rules! unpack_ziplist_entry {
    ($data:expr) => ({
        let entry = try!($data);
        match entry {
            ZiplistEntry::String(ref val) => val.as_slice(),
            ZiplistEntry::Number(val) => val.to_string().as_bytes()
        }
    })
}
