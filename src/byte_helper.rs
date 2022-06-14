use crate::sha1;

#[derive(Debug)]
pub enum ByteHelperError {
    InvalidHexChar,
    SizingError,
}

pub fn bytes_to_u32(bytes: [u8; 4]) -> u32  {
    (((bytes[0] as u32) << 24) & 0xff000000) |
    (((bytes[1] as u32) << 16) & 0x00ff0000) |
    (((bytes[2] as u32) <<  8) & 0x0000ff00) |
    ((bytes[3] as u32)        & 0x000000ff)
}

pub fn u32_to_bytes(word: u32) -> [u8; 4] {
    // (0..4).rev()
        // .map(|i| ((word >> 8*i) & 0xff) as u8)
        // .collect()
    [
        ((word >> 8*3) & 0xff) as u8,
        ((word >> 8*2) & 0xff) as u8,
        ((word >> 8*1) & 0xff) as u8,
        ((word >> 8*0) & 0xff) as u8,
    ]
}

pub fn u64_to_bytes(word: u64) -> [u8; 8] {
    // (0..8).rev()
        // .map(|i| ((word >> 8*i) & 0xff) as u8)
        // .collect()
    [
        ((word >> 8*7) & 0xff) as u8,
        ((word >> 8*6) & 0xff) as u8,
        ((word >> 8*5) & 0xff) as u8,
        ((word >> 8*4) & 0xff) as u8,
        ((word >> 8*3) & 0xff) as u8,
        ((word >> 8*2) & 0xff) as u8,
        ((word >> 8*1) & 0xff) as u8,
        ((word >> 8*0) & 0xff) as u8,
    ]
}

pub fn hexstring_to_digest(hexstring: &str) -> Result<[u8; sha1::DIGEST_SIZE], ByteHelperError> {
    // Return an error if there are an odd number of hex digits 
    if hexstring.len() % 2 != 0 {
        return Err(ByteHelperError::SizingError);
    }

    // Convert hexstring into a byte-array
    let mut digest = [0_u8; sha1::DIGEST_SIZE];
    for i in (0..hexstring.len()).step_by(2) {
        match u8::from_str_radix(&hexstring[i..i+2], 16) {
            Ok(byte) => digest[i/2] = byte,
            Err(_) => return Err(ByteHelperError::InvalidHexChar),
        }
    }

    Ok(digest)
}

pub fn otp6_to_chars(num: u32) -> [char; 6] {
    let mut out_bytes = [' '; 6];

    for i in 0..6 {
        let digit = num / 10_u32.pow(6-i-1) % 10;
        out_bytes[i as usize] = ((0x30 + digit) as u8) as char;
    }

    out_bytes
}