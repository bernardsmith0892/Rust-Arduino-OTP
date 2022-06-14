use crate::byte_helper;

const BLOCK_SIZE: usize = 64;
const PAD_TARGET: usize = 56;
const MAX_KEY_SIZE: usize = 56;
pub const DIGEST_SIZE: usize = 20;

#[derive(Debug)]
pub enum OtpError {
    InputSizeError,
    ProcessingError,
}

pub fn gen_sha1_hotp(key: &[u8], counter: u64, digits: u32) -> Result<u32, OtpError> {
    if key.len() > MAX_KEY_SIZE {
        return Err(OtpError::InputSizeError);
    }

    // Convert counter value into its byte form
    let counter_bytes = byte_helper::u64_to_bytes(counter);

    // Calculate HMAC(key, counter)
    let hmac = gen_sha1_hmac(key, &counter_bytes)?;

    // Dynamic Truncation
    // 1. Determine offset based on lower 4-bits of the last byte
    let offset = match hmac.get(19) {
        Some(byte) => (*byte & 0x0f) as usize,
        None => return Err(OtpError::ProcessingError),
    };
    // 2. Parse 4-bytes starting from the offset as a u32 and ignore the first bit
    let mut offset_bytes = [0_u8; 4];
    offset_bytes.copy_from_slice(&hmac[offset..offset+4]);

    let truncated_value: u32 = byte_helper::bytes_to_u32(offset_bytes) & 0x7fffffff;

    // 3. Return final value with specified number of digits
    Ok(truncated_value % 10u32.pow(digits))
}

pub fn gen_sha1_hmac(key: &[u8], message: &[u8]) -> Result<[u8; DIGEST_SIZE], OtpError> {
    // Only supports up to 64-byte keys (512-bits)
    if key.len() > BLOCK_SIZE {
        return Err(OtpError::InputSizeError);
    }

    // Initialize Pads
    let mut inner_pad = [b'6'; BLOCK_SIZE];
    let mut outer_pad = [b'\\'; BLOCK_SIZE];
    for (i, key_val) in key.iter().enumerate() {
        inner_pad[i] ^= key_val;
        outer_pad[i] ^= key_val;
    }

    // Inner Hash
    let inner_hash = gen_sha1_digest(&inner_pad, Some(&message))?;

    // Outer Hash and Result
    gen_sha1_digest(&outer_pad, Some(&inner_hash))
}

pub fn gen_sha1_digest(message: &[u8], appendix_message: Option<&[u8]>) -> Result<[u8; DIGEST_SIZE], OtpError> {
    // ***********************************
    // *** 5.1.1 - Padding the Message ***
    // ***********************************

    // // Allocated padded message with `0` bytes
    // let mut padded_message = [0_u8; BLOCK_SIZE];

    let total_message_length: usize = message.len() + appendix_message.unwrap_or(&[]).len();
    // Number of padding 0x00 bytes (excluding 0x80 byte for first pad)
    let padding_length: usize = match total_message_length % BLOCK_SIZE {
        0..=PAD_TARGET => PAD_TARGET - ((total_message_length + 1) % BLOCK_SIZE),
        _ => BLOCK_SIZE - ((total_message_length + 1) % BLOCK_SIZE) + PAD_TARGET
    };
    // Starting index of the bit-length bytes
    let final_bytes_index: usize = total_message_length + padding_length + 1;
    let total_padded_message_length = total_message_length + padding_length + 1 + 8;

    // // Copy original message
    // for (i, byte) in padded_message[0..message.len()].iter_mut().enumerate() {
        // *byte = message[i];
    // }

    // // Append appendix message, if one is provided
    // if let Some(appendix) = appendix_message {
        // for (i, byte) in padded_message[message.len()..message.len()+appendix.len()].iter_mut().enumerate() {
            // *byte = appendix[i];
        // }
    // }

    // // Append `0b10000000` as the first padding byte
    // padded_message[message.len() + appendix_message.unwrap_or(&[]).len()] = 0x80;

    // Determine the byte representation of the total message length
    let bit_length: u64 = total_message_length as u64 * 8;
    let length_bytes: [u8; 8] = byte_helper::u64_to_bytes(bit_length);

    // for (i, byte) in padded_message[BLOCK_SIZE-8..BLOCK_SIZE].iter_mut().enumerate() {
        // *byte = length_bytes[i];
    // }

    // **********************************************
    // *** 5.3.1 - Setting the Initial Hash Value ***
    // **********************************************
    let mut h: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];

    // **************************************
    // *** 6.1.2 - SHA-1 Hash Computation ***
    // **************************************
    for i in (0..total_padded_message_length).step_by(BLOCK_SIZE) {
        // 1. Prepare the message schedule (W)
        let mut w = [0_u32; 80];

        // The first 16 words of W are the 32-bit word representations of the current 64-bytes
        for t in 0..16 {
            let mut m_bytes = [0_u8; 4];
            for b in 0..4 {
                let curr_idx = i+t*4+b;

                // Use the message bytes for this index
                if curr_idx < message.len() {
                    m_bytes[b] = message[curr_idx];
                }
                // Use the appendix bytes for this index
                else if curr_idx < total_message_length {
                    let appendix = appendix_message.unwrap_or(&[]);
                    m_bytes[b] = appendix[curr_idx - message.len()];
                }
                // Add the 0x80 padding byte with three 0x00 bytes
                else if curr_idx == total_message_length {
                    m_bytes[b] = 0x80;
                }
                // Add padding bytes
                else if curr_idx < total_message_length + padding_length + 1 {
                    m_bytes[b] = 0x00;
                }
                // Add bit-length bytes
                else {
                    m_bytes[b] = length_bytes[ curr_idx - final_bytes_index ];
                }
            }

            w[t] = byte_helper::bytes_to_u32(m_bytes);
        }

        // Fill out the remainder of the message schedule
        for t in 16..80 {
            w[t] = rotate_left(w[t-3] ^ w[t-8] ^ w[t-14] ^ w[t-16], 1);
        }

        // 2. Initialize working variables
        let mut a: u32 = h[0];
        let mut b: u32 = h[1];
        let mut c: u32 = h[2];
        let mut d: u32 = h[3];
        let mut e: u32 = h[4];

        // 3. Process message schedule
        for t in 0..80 {
            let temp: u32 = rotate_left(a, 5)
            .wrapping_add(f(t, b, c, d))
            .wrapping_add(e)
            .wrapping_add(k(t))
            .wrapping_add(w[t]);

            e = d;
            d = c;
            c = rotate_left(b, 30);
            b = a;
            a = temp;
        }

        // 4. Compute Intermediate Hash Value
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    // Compile the h-array into the 20-byte digest
    let mut digest = [0_u8; DIGEST_SIZE];
    for (i, h_value) in h.iter().enumerate() {
        // Convert the H value into 4-bytes
        let h_bytes = byte_helper::u32_to_bytes(*h_value);

        // Add the bytes to the output digest
        for (digest_byte, h_byte) in digest[i*4..i*4+4].iter_mut().zip(h_bytes) {
            *digest_byte = h_byte; 
        }
    }

    Ok(digest)
}
    
fn f(t: usize, x: u32, y: u32, z: u32) -> u32 {
    match t {
         0 ..= 19 => choose(x, y, z),
        20 ..= 39 => parity(x, y, z),
        40 ..= 59 => majority(x, y, z),
            _     => parity(x, y, z),
    }
}

fn choose(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (!x & z)
}

fn parity(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}

fn majority(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (x & z) | (y & z)
}

fn rotate_left(x: u32, n: u8) -> u32 {
    (x << n) | (x >> (32 - n))
}

fn k(t: usize) -> u32 {
    const K: [u32; 4] = [0x5a827999, 0x6ed9eba1, 0x8f1bbcdc, 0xca62c1d6];
    K[t / 20]
}