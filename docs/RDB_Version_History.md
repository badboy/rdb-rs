% RDB Version History
## RDB Version History

This document tracks the changes made to the dump file format over time.

An RDB file is forwards compatible. An older dump file format will always work with a newer version of Redis.

## Version 7

Introduced 2014-01-08, integrated into Redis 2.9.x.

* New opcode: `RESIZEDB` (251). This encodes hash tables sizes to allow for faster loading.  
  Followed by two length-encoded integers indicating:
    * Database hash table size
    * Expiry hash table size
* New opcode: `AUX` (250). This allows for arbitrary key-value settings. Unknown keys are ignored.  
  Followed by two length-prefixed strings representing the key and value of the setting. Currently implemented fields:
    * `redis-ver`: The Redis Version that wrote the RDB
    * `redis-bits`: Bit architecture of the system that wrote the RDB, either 32 or 64
    * `ctime`: Creation time of the RDB
    * `used-mem`: Used memory of the instance that wrote the RDB
* New encoding type `LIST_QUICKLIST` (14)

Relevant links:

* opcode `AUX`: [redis#206cd219](https://github.com/antirez/redis/commit/206cd219b63c2255c0238cb9c602b65f05e98120), [redis#4c0e8923](https://github.com/antirez/redis/commit/4c0e8923a6cb376c7b2a53fa76ae95f74610285c)
* opcode `RESIZEDB`: [redis#e8614a1a](https://github.com/antirez/redis/commit/e8614a1a77d2989f7be3cb7b24cd88b01f14f17e)
* new type `LIST_QUICKLIST`: [redis#101b3a6e](https://github.com/antirez/redis/commit/101b3a6e42e84e5cb423ef413225d8b8d8cc0bbc), [Redis Quicklist: Do you even list?](https://matt.sh/redis-quicklist-visions)

**Caution**: This breaks backwards-compatibility. Redis 2.8 cannot load a RDB version 7 file.

## Version 6

In previous versions, ziplists used a variable length encoding scheme for integers.
Integers were stored in 16, 32 or 64 bits. In this version, this variable length
encoding system has been extended.

The following additions have been made :

* Integers 0 through 12, both inclusive, are now encoded as part of the entry header
* Numbers between -128 and 127, both inclusive, are stored in 1 byte
* Numbers between -2^23 and 2^23 -1, both inclusive, are stored in 3 bytes

Issue ID: [redis#469](https://github.com/antirez/redis/issues/469)

To migrate to version 5:

* In redis.conf, set `list-max-ziplist-entries` to 0
* Restart Redis Server, and issue the `SAVE` command
* Edit the dump.rdb file, and change the rdb version in the header to `REDIS0005`


## Version 5

This version introduced an 8 byte checksum (CRC64) at the end of the file. If checksum is disabled in redis.conf,
the last 8 bytes will be zeroes.

Issue ID: [redis#366](https://github.com/antirez/redis/issues/366)

To migrate to version 4:

* Delete the last 8 bytes of the file (i.e. after the byte `0xFF`)
* Change the rdb version in the header to `REDIS0004`


## Version 4

This version introduced a new encoding for hashmaps - "Hashmaps encoded as Zip Lists". This version also deprecates
the Zipmap encoding that was used in previous versions.

"Hashmaps encoded as ziplists" has encoding type = 13. The value is parsed like a ziplist, and adjacent entries
in the list are considered key=value pairs in the hashmap.

Issue ID: [redis#285](https://github.com/antirez/redis/pull/285)

To migrate to version 3:

* In redis.conf, set `hash-max-ziplist-entries` to 0
* Restart Redis Server, and issue the `SAVE` command
* Edit the dump.rdb file, and change the rdb version in the header to `REDIS0003`

## Version 3

This version introduced key expiry with millisecond precision.

Earlier versions stored key expiry in the format `0xFD <4 byte timestamp>`. In version 3, key expiry is stored as
`0xFC <8 byte timestamp>`. Here, 0xFD and 0xFC are the opcodes to indicate key expiry in seconds and milliseconds respectively.

Issue ID: [redis#169](https://github.com/antirez/redis/issues/169)

To migrate to version 2:

* If you don't use key expiry, simply change the version in the header to `REDIS0002`
* If you use key expiry, you can still migrate, but there will be some loss in expiry precision. Also, the migration is a bit involved.
* For each key=value pair in the dump file, you will have to convert `0xFC <8 byte timestamp>` to `0xFD <4 byte timestamp>`.
* After converting the timestamps, change the version in the header to `REDIS0002`

## Version 2

This version introduced special encoding for small hashmaps, lists and sets.

Specifically, it introduced the following encoding types:

    REDIS_RDB_TYPE_HASH_ZIPMAP = 9
    REDIS_RDB_TYPE_LIST_ZIPLIST = 10
    REDIS_RDB_TYPE_SET_INTSET = 11
    REDIS_RDB_TYPE_ZSET_ZIPLIST = 12

Commit: [redis#6b52ad87](https://github.com/antirez/redis/commit/6b52ad87c05ca2162a2d21f1f5b5329bf52a7678)

To migrate to version 1:

* In redis.conf, set the following properties to 0 `hash-max-zipmap-entries, list-max-ziplist-entries, set-max-intset-entries, zset-max-ziplist-entries`
* Restart Redis, and issue the SAVE command
* Edit the dump.rdb file, and change the rdb version in the header to `REDIS0001`

