use std::io::Read;
use std::io::Result as IoResult;

pub fn int_to_vec(number: i32) -> Vec<u8> {
  let number = number.to_string();
  let mut result = Vec::with_capacity(number.len());
  for &c in number.as_bytes().iter() {
    result.push(c);
  }
  result
}

pub fn read_exact<T: Read>(reader: &mut T, len: usize) -> IoResult<Vec<u8>> {
    let mut buf = vec![0; len];
    try!(reader.read_exact(&mut buf));

    Ok(buf)
}
