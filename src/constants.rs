pub mod version {
    pub const SUPPORTED_MINIMUM: u32 = 1;
    pub const SUPPORTED_MAXIMUM: u32 = 12;
}

pub mod constant {
    pub const RDB_6BITLEN: u8 = 0;
    pub const RDB_14BITLEN: u8 = 1;
    pub const RDB_ENCVAL: u8 = 3;
    pub const RDB_MAGIC: &'static str = "REDIS";
}

pub mod op_code {
    pub const MODULE_AUX: u8 = 247;
    pub const IDLE: u8 = 248;
    pub const FREQ: u8 = 249;
    pub const AUX: u8 = 250;
    pub const RESIZEDB: u8 = 251;
    pub const EXPIRETIME_MS: u8 = 252;
    pub const EXPIRETIME: u8 = 253;
    pub const SELECTDB: u8 = 254;
    pub const EOF: u8 = 255;
}

pub mod encoding_type {
    pub const STRING: u8 = 0;
    pub const LIST: u8 = 1;
    pub const SET: u8 = 2;
    pub const ZSET: u8 = 3;
    pub const HASH: u8 = 4;
    pub const ZSET_2: u8 = 5;
    pub const MODULE: u8 = 6;
    pub const MODULE_2: u8 = 7;
    pub const HASH_ZIPMAP: u8 = 9;
    pub const LIST_ZIPLIST: u8 = 10;
    pub const SET_INTSET: u8 = 11;
    pub const ZSET_ZIPLIST: u8 = 12;
    pub const HASH_ZIPLIST: u8 = 13;
    pub const LIST_QUICKLIST: u8 = 14;
    pub const STREAM_LIST_PACKS: u8 = 15;
    pub const HASH_LIST_PACK: u8 = 16;
    pub const ZSET_LIST_PACK: u8 = 17;
    pub const LIST_QUICKLIST_2: u8 = 18;
    pub const STREAM_LIST_PACKS_2: u8 = 19;
    pub const SET_LIST_PACK: u8 = 20;
    pub const STREAM_LIST_PACKS_3: u8 = 21;
}

pub mod encoding {
    pub const INT8: u32 = 0;
    pub const INT16: u32 = 1;
    pub const INT32: u32 = 2;
    pub const LZF: u32 = 3;
}
