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

fn try_read<T: Read>(reader: &mut T, buf : &mut [u8], min_bytes : usize) -> IoResult<usize> {
    let mut pos = 0;
    let buf_len = buf.len();
    while pos < min_bytes {
        let buf1 = &mut buf[pos .. buf_len];
        let n = try!(reader.read(buf1));
        pos += n;
        if n == 0 { return Ok(pos);  }

    }
    return Ok(pos);

}

fn read<T: Read>(reader: &mut T, buf : &mut [u8], min_bytes : usize) -> IoResult<usize> {
    let n = try!(try_read(reader, buf, min_bytes));
    if n < min_bytes {
        Err(::std::io::Error::new(::std::io::ErrorKind::Other,
                                  "Could not read enough bytes from Reader",
                                  None))
    } else {
        Ok(n)
    }
}

pub fn read_exact<T: Read>(reader: &mut T, len: usize) -> IoResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(len);
    unsafe { buf.set_len(len); }

    try!(read(reader, &mut buf, len));

    Ok(buf)
}
