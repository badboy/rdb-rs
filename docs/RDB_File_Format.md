# Redis RDB File Format

Redis' RDB file is a binary representation of the in-memory store. This binary file is sufficient to completely restore Redis' state.

The RDB file format is optimized for fast read and writes. Where possible LZF compression is used to reduce the file size. In general, objects are prefixed with their lengths, so before reading the object you know exactly how much memory to allocate.

Optimizing for fast read/writes means the on-disk format should be as close as possible to the in-memory representation. This is the approach taken by the RDB file. As a consequence, you cannot parse the RDB file without some understanding of Redis' in-memory representation of data structures.

## High Level Algorithm to parse RDB

At a high level, the RDB file has the following structure

```
----------------------------#
52 45 44 49 53              # Magic String "REDIS"
30 30 30 33                 # RDB Version Number as ASCII string. "0003" = 3
----------------------------
FA                          # Auxiliary field
$string-encoded-key         # May contain arbitrary metadata
$string-encoded-value       # such as Redis version, creation time, used memory, ...
----------------------------
FE 00                       # Indicates database selector. db number = 00
FB                          # Indicates a resizedb field
$length-encoded-int         # Size of the corresponding hash table
$length-encoded-int         # Size of the corresponding expire hash table
----------------------------# Key-Value pair starts
FD $unsigned-int            # "expiry time in seconds", followed by 4 byte unsigned int
$value-type                 # 1 byte flag indicating the type of value
$string-encoded-key         # The key, encoded as a redis string
$encoded-value              # The value, encoding depends on $value-type
----------------------------
FC $unsigned long           # "expiry time in ms", followed by 8 byte unsigned long
$value-type                 # 1 byte flag indicating the type of value
$string-encoded-key         # The key, encoded as a redis string
$encoded-value              # The value, encoding depends on $value-type
----------------------------
$value-type                 # key-value pair without expiry
$string-encoded-key
$encoded-value
----------------------------
FE $length-encoding         # Previous db ends, next db starts.
----------------------------
...                         # Additional key-value pairs, databases, ...

FF                          ## End of RDB file indicator
8-byte-checksum             ## CRC64 checksum of the entire file.
```


## Magic Number

The file starts off with the magic string "REDIS". This is a quick sanity check to know we are dealing with a redis rdb file.

```
52 45 44 49 53  # "REDIS"
```

## RDB Version Number

The next 4 bytes store the version number of the rdb format. The 4 bytes are interpreted as ASCII characters and then converted to an integer using string to integer conversion.

```
30 30 30 33 # "0003" => Version 3
```

## Op Codes

Each part after the initial header is introduced by a special op code.
The available op codes are:

| Byte | Name         | Description |
| ---- | ------------ | ----------- |
| 0xFF | EOF          | End of the RDB file |
| 0xFE | SELECTDB     | [Database Selector](#database-selector) |
| 0xFD | EXPIRETIME   | Expire time in seconds, see [Key Expiry Timestamp](#key-expiry-timestamp) |
| 0xFC | EXPIRETIMEMS | Expire time in milliseconds, see [Key Expiry Timestamp](#key-expiry-timestamp) |
| 0xFB | RESIZEDB     | Hash table sizes for the main keyspace and expires, see [Resizedb information](#resizedb) |
| 0xFA | AUX          | Auxiliary fields. Arbitrary key-value settings, see [Auxiliary fields](#aux-fields) |

## Database Selector

A Redis instance can have multiple databases.

A single byte `0xFE` flags the start of the database selector. After this byte, a variable length field indicates the database number. See the section [Length Encoding](#length-encoding) to understand how to read this database number.

## Resizedb information

This op code was introduced in RDB version 7.

It encodes two values to speed up RDB loading by avoiding additional resizes and rehashing.
The op code is followed by two [length-encoded](#length-encoded) integers indicating:

* Database hash table size
* Expiry hash table size

## Auxiliary fields

This op code was introduced in RDB version 7.

The op code is followed by two [Redis Strings](#string-encoding), representing the key and value of a setting.
Unknown fields should be ignored by a parser.

Currently the following settings are implemented:

* `redis-ver`: The Redis Version that wrote the RDB
* `redis-bits`: Bit architecture of the system that wrote the RDB, either 32 or 64
* `ctime`: Creation time of the RDB
* `used-mem`: Used memory of the instance that wrote the RDB

## Key Value Pairs

After the database selector, the file contains a sequence of key value pairs.

Each key value pair has 4 parts:

* Key Expiry Timestamp. This is optional.
* 1 byte flag indicating the value type.
* The key, encoded as a Redis String. See [String Encoding](#string-encoding).
* The value, encoded according to the value type. See [Value Encoding](#value-encoding).

### Key Expiry Timestamp

This section starts with a one byte flag.
This flag is either:

* `0xFD`: The following expire value is specified in seconds. The following 4 bytes represent the Unix timestamp as an unsigned integer.
* `0xFC`: The following expire value is specified in milliseconds. The following 8 bytes represent the Unix timestamp as an unsigned long.

During the import process, keys that have expired must be discarded.

### Value Type

A one byte flag indicates encoding used to save the Value.

* ` 0` =  [String Encoding](#string-encoding)
* ` 1` =  [List Encoding](#list-encoding)
* ` 2` =  [Set Encoding](#set-encoding)
* ` 3` =  [Sorted Set Encoding](#sorted-set-encoding)
* ` 4` =  [Hash Encoding](#hash-encoding)
* ` 9` =  [Zipmap Encoding](#zipmap-encoding)
* `10` = [Ziplist Encoding](#ziplist-encoding)
* `11` = [Intset Encoding](#intset-encoding)
* `12` = [Sorted Set in Ziplist Encoding](#sorted-set-in-ziplist-encoding)
* `13` = [Hashmap in Ziplist Encoding](#hashmap-in-ziplist-encoding) (Introduced in RDB version 4)
* `14` = [List in Quicklist encoding](#quicklist-encoding) (Introduced in RDB version 7)

### Key

The key is simply encoded as a Redis string. See the section [String Encoding](#string-encoding) to learn how the key is encoded.

### Value

The value is parsed according to the previously read [Value Type](#value-type)

## Encodings

### Length Encoding

Length encoding is used to store the length of the next object in the stream. Length encoding is a variable byte encoding designed to use as few bytes as possible.

This is how length encoding works :
Read one byte from the stream, compare the two most significant bits:

| Bits | How to parse |
| ---- | ------------ |
| `00` | The next 6 bits represent the length |
| `01` | Read one additional byte. The combined 14 bits represent the length |
| `10` | Discard the remaining 6 bits. The next 4 bytes from the stream represent the length |
| `11` | The next object is encoded in a special format. The remaining 6 bits indicate the format. May be used to store numbers or Strings, see [String Encoding](#string-encoding) |

As a result of this encoding:

* Numbers up to and including 63 can be stored in 1 byte
* Numbers up to and including 16383 can be stored in 2 bytes
* Numbers up to 2^32 -1 can be stored in 4 bytes

### String Encoding

Redis Strings are binary safe - which means you can store anything in them. They do not have any special end-of-string token. It is best to think of Redis Strings as a byte array.

There are three types of Strings in Redis:

* Length prefixed strings
* An 8, 16 or 32 bit integer
* A LZF compressed string

#### Length Prefixed String

Length prefixed strings are quite simple. The length of the string in bytes is first encoded using [Length Encoding](#length-encoding). After this, the raw bytes of the string are stored.

#### Integers as String

First read the section [Length Encoding](#length-encoding), specifically the part when the first two bits are `11`. In this case, the remaining 6 bits are read.

If the value of those 6 bits is:

* `0` indicates that an 8 bit integer follows
* `1` indicates that a 16 bit integer follows
* `2` indicates that a 32 bit integer follows

#### Compressed Strings

First read the section [Length Encoding](#length-encoding), specifically the part when the first two bits are `11`. In this case, the remaining 6 bits are read.
If the value of those 6 bits is 4, it indicates that a compressed string follows.

The compressed string is read as follows:

* The compressed length `clen` is read from the stream using [Length Encoding](#length-encoding)
* The uncompressed length is read from the stream using [Length Encoding](#length-encoding)
* The next `clen` bytes are read from the stream
* Finally, these bytes are decompressed using LZF algorithm

### List Encoding

A Redis list is represented as a sequence of strings.

* First, the size of the list `size` is read from the stream using [Length Encoding](#length-encoding)
* Next, `size` strings are read from the stream using [String Encoding](#string-encoding)
* The list is then re-constructed using these Strings

### Set Encoding

Sets are encoded exactly like lists.

### Sorted Set Encoding

* First, the size of the sorted set `size` is read from the stream using [Length Encoding](#length-encoding)
* **TODO**

### Hash Encoding

* First, the `size` of the hash is read from the stream using [Length Encoding](#length-encoding)
* Next ` 2 * size ` strings are read from the stream using [String Encoding](#string-encoding) (alternate strings are key and values)

**Example:**

```
2 us washington india delhi
```

represents the map

```
{"us" => "washington", "india" => "delhi"}
```

### Zipmap Encoding

A Zipmap is a hashmap that has been serialized to a string. In essence, the key value pairs are stored sequentially. Looking up a key in this structure is O(N). This structure is used instead of a dictionary when the number of key value pairs are small.

To parse a zipmap, first a string is read from the stream using [String Encoding](#string-encoding).
The contents of this string represent the zipmap.

The structure of a zipmap within this string is as follows:

```
<zmlen><len>"foo"<len><free>"bar"<len>"hello"<len><free>"world"<zmend>
```

* `zmlen`: 1 byte that holds the size of the zip map. If it is greater than or equal to 254, value is not used. You will have to iterate the entire zip map to find the length.
* `len`: the length of the following string, which can be either a key or a value. This length is stored in either 1 byte or 5 bytes (yes, it differs from [Length Encoding](#length-encoding) described above). If the first byte is between 0 and 252, that is the length of the zipmap. If the first byte is 253, then the next 4 bytes read as an unsigned integer represent the length of the zipmap. 254 and 255 are invalid values for this field.
* `free` : This is always 1 byte, and indicates the number of free bytes _after_ the value. For example, if the value of a key is "America" and its get updated to "USA", 4 free bytes will be available.
* `zmend` : Always `255`. Indicates the end of the zipmap.

**Example:**

`18 02 06 4D 4B 44 31 47 36 01 00 32 05 59 4E 4E 58 4b 04 00 46 37 54 49 FF ..`

* Start by decoding this using [String Encoding](#string-encoding). You will notice that `0x18` (`24` in decimal) is the length of the string. Accordingly, we will read the next 24 bytes i.e. up to `FF`
* Now, we are parsing the string starting at `02 06... ` using the [Zipmap Encoding](#zipmap-encoding)
* `02` is the number of entries in the hashmap.
* `06` is the length of the next string. Since this is less than 254, we don't have to read any additional bytes
* We read the next 6 bytes i.e. `4d 4b 44 31 47 36` to get the key "MKD1G6"
* `01` is the length of the next string, which would be the value
* `00` is the number of free bytes
* We read the next 1 byte, which is `0x32`. Thus, we get our value `"2"`
* In this case, the free bytes is `0`, so we don't skip anything
* `05` is the length of the next string, in this case a key.
* We read the next 5 bytes `59 4e 4e 58 4b`, to get the key `"YNNXK"`
* `04` is the length of the next string, which is a value
* `00` is the number of free bytes after the value
* We read the next 4 bytes i.e. `46 37 54 49` to get the value `"F7TI"`
* Finally, we encounter `FF`, which indicates the end of this zip map
* Thus, this zip map represents the hash `{"MKD1G6" => "2", "YNNXK" => "F7TI"}`

### Ziplist Encoding

A Ziplist is a list that has been serialized to a string. In essence, the elements of the list are stored sequentially along with flags and offsets to allow efficient traversal of the list in both directions.

To parse a ziplist, first a string is read from the stream using [String Encoding](#string-encoding).
The contents of this string represent the ziplist.

The structure of a ziplist within this string is as follows:

```
<zlbytes><zltail><zllen><entry><entry><zlend>
```

* `zlbytes`: a 4 byte unsigned integer representing the total size in bytes of the ziplist. The 4 bytes are in little endian format - the least significant bit comes first.
* `zltail`: a 4 byte unsigned integer in little endian format. It represents the offset to the tail (i.e. last) entry in the ziplist
* `zllen`: This is a 2 byte unsigned integer in little endian format. It represents the number of entries in this ziplist
* `entry`: An entry represents an element in the ziplist. Details below
* `zlend`: Always `255`. It represents the end of the ziplist.

Each entry in the ziplist has the following format :

```
<length-prev-entry><special-flag><raw-bytes-of-entry>
```

`length-prev-entry`: stores the length of the previous entry, or 0 if this is the first entry. This allows easy traversal of the list in the reverse direction. This length is stored in either 1 byte or in 5 bytes. If the first byte is less than or equal to 253, it is considered as the length. If the first byte is 254, then the next 4 bytes are used to store the length. The 4 bytes are read as an unsigned integer.

`special-flag`: This flag indicates whether the entry is a string or an integer. It also indicates the length of the string, or the size of the integer.
The various encodings of this flag are shown below:

| Bytes | Length | Meaning |
| ----- | ------ | ------- |
| `00pppppp` | 1 byte | String value with length less than or equal to 63 bytes (6 bits) |
| `01pppppp|qqqqqqqq` | 2 bytes | String value with length less than or equal to 16383 bytes (14 bits) |
| `10______|<4 byte>` | 5 bytes | Next 4 byte contain an unsigned int. String value with length greater than or equal to 16384 bytes |
| `1100____` | 1 byte | Integer encoded as 16 bit Integer (2 bytes) |
| `1101____` | 1 byte | Integer encoded as 32 bit Integer (4 bytes) |
| `1110____` | 1 byte | Integer encoded as 64 bit Integer (8 bytes) |

`raw-byte`: After the special flag, the raw bytes of entry follow. The number of bytes was previously determined as part of the special flag.

**Example:**

```
23 23 00 00 00 1E 00 00 00 04 00 00 E0 FF FF FF FF FF
FF FF 7F 0A D0 FF FF 00 00 06 C0 FC 3F 04 C0 3F 00 FF ...
```

* Start by decoding this using [String Encoding](#string-encoding). `23` is the length of the string (35 in decimal), therefore we will read the next 35 bytes till `ff`
* Now, we are parsing the string starting at `23 00 00 ...` using [Ziplist encoding](#ziplist-encoding)
* The first 4 bytes `23 00 00 00` represent the total length in bytes of this ziplist. Notice that this is in little endian format
* The next 4 bytes `1e 00 00 00` represent the offset to the tail entry. `1E` = 30 (in decimal), and this is a 0 based offset. 0th position = `23`, 1st position = `00` and so on. It follows that the last entry starts at `04 c0 3f 00 ..`
* The next 2 bytes `04 00` represent the number of entries in this list as a big endian 16 bit integer. `04 00 = 4` in decimal.
* From now on, we start reading the entries
* `00` represents the length of previous entry. `0` indicates this is the first entry.
* `E0` is the special flag. Since it starts with the bit pattern `1110____`, we read the next 8 bytes as an integer. This is the first entry of the list.
* We now start the second entry
* `0A` is the length of the previous entry. `0A` = `10` in decimal. 10 bytes = 1 byte for prev. length + 1 byte for special flag + 8 bytes for integer.
* `D0` is the special flag. Since it starts with the bit pattern `1101____`, we read the next 4 bytes as an integer. This is the second entry of the list
* We now start the third entry
* `06` is the length of previous entry. 6 bytes = 1 byte for prev. length + 1 byte for special flag + 4 bytes for integer
* `C0` is the special flag. Since it starts with the bit pattern `1100____`, we read the next 2 bytes as an integer. This is the third entry of the list
* We now start the last entry
* `04` is length of previous entry
* `C0` indicates a 2 byte number
* We read the next 2 bytes, which gives us our fourth entry
* Finally, we encounter `FF`, which tells us we have consumed all elements in this ziplist.
* Thus, this ziplist stores the values `[0x7fffffffffffffff, 65535, 16380, 63]`

### Intset Encoding

An Intset is a binary search tree of integers. The binary tree is implemented in an array of integers. An intset is used when all the elements of the set are integers. An Intset has support for up to 64 bit integers. As an optimization, if the integers can be represented in fewer bytes, the array of integers will be constructed from 16 bit or 32 bit integers. When a new element is inserted, the implementation takes care to upgrade if necessary.

Since an Intset is a binary search tree, the numbers in this set will always be sorted.

An Intset has an external interface of a Set.

To parse an Intset, first a string is read from thee stream using [String Encoding](#string-encoding).
The contents of this string represent the Intset.

Within this string, the Intset has a very simple layout :

```
<encoding><length-of-contents><contents>
```

* `encoding`: is a 32 bit unsigned integer. It has 3 possible values - 2, 4 or 8. It indicates the size in bytes of each integer stored in contents. And yes, this is wasteful - we could have stored the same information in 2 bits.
* `length-of-contents`: is a 32 bit unsigned integer, and indicates the length of the contents array
* `contents`: is an array of $length-of-contents bytes. It contains the binary tree of integers

**Example**

`14 04 00 00 00 03 00 00 00 FC FF 00 00 FD FF 00 00 FE FF 00 00 ...`

* Start by decoding this using "String Encoding". `14` is the length of the string, therefore we will read the next 20 bytes till `00`
* Now, we start interpreting the string starting at `04 00 00 ... `
* The first 4 bytes `04 00 00 00` is the encoding. Since this evaluates to 4, we know we are dealing with 32 bit integers
* The next 4 bytes `03 00 00 00` is the length of contents. So, we know we are dealing with 3 integers, each 4 byte long
* From now on, we read in groups of 4 bytes, and convert it into a unsigned integer
* Thus, our intset looks like `[0x0000FFFC, 0x0000FFFD, 0x0000FFFE]`. Notice that the integers are in little endian format i.e. least significant bit came first.

### Sorted Set in Ziplist Encoding

A sorted set in ziplist encoding is stored just like the [Ziplist](#ziplist-encoding) described above. Each element in the sorted set is followed by its score in the ziplist.

**Example**

`['Manchester City', 1, 'Manchester United', 2, 'Tottenham', 3]`

As you see, the scores follow each element.

### Hashmap in Ziplist Encoding

In this, key=value pairs of a hashmap are stored as successive entries in a ziplist.

Note: This was introduced in RDB version 4. This deprecates the [zipmap encoding](#zipmap-encoding) that was used in earlier versions.

**Example**

`{"us" => "washington", "india" => "delhi"}`

is stored in a ziplist as :

`["us", "washington", "india", "delhi"]`

### Quicklist Encoding

RDB Version 7 introduced a new variant of list encoding: Quicklist.

Quicklist is a linked list of ziplists. Quicklist combines the memory efficiency of small ziplists with the extensibility of a linked list allowing us to create space-efficient lists of any length.

To parse a quicklist, first a string is read from the stream using [String Encoding](#string-encoding).
The contents of this string represent the ziplist.

The structure of a quicklist within this string is as follows:

```
<len><ziplist><ziplist>...
```

* `len`: This is the number of nodes of the linked list, [length-encoded](#length-encoding)
* `ziplist`: A string that wraps a ziplist, parse it with [Ziplist encoding](#ziplist-encoding)

A complete list needs to be constructed from all elements of all ziplists.

**Example**:

```
01 00 0e 09 71 75 ...
```

*TODO: proper example*


## CRC64 Checksum

Starting with RDB Version 5, an 8 byte CRC64 checksum is added to the end of the file. It is possible to disable this checksum via a parameter in redis.conf.
When the checksum is disabled, this field will have zeroes.
