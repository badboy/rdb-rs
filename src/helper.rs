use std::io::Read;
use std::io::Result as IoResult;
use std::io::{Error,ErrorKind};

pub fn int_to_vec(number: i32) -> Vec<u8> {
    let number = number.to_string();
    let mut result = Vec::with_capacity(number.len());
    for &c in number.as_bytes().iter() {
        result.push(c);
    }
    result
}

// Taken from Rust:
// https://github.com/rust-lang/rust/blob/6f880eee792e974b18cbb129fd16928939589e7c/src/libstd/io/mod.rs#L596-L611
pub fn read_exact<T: Read>(reader: &mut T, len: usize) -> IoResult<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    unsafe {
        out.set_len(len);
    }

    {
        let mut buf = &mut *out;
        while !buf.is_empty() {
            match reader.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            return Err(Error::new(ErrorKind::Interrupted,
                                  "failed to fill whole buffer"))
        }
    }

    Ok(out)
}
