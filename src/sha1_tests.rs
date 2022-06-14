#[cfg(test)]
mod creating_digests {
    use crate::byte_helper;
    use crate::sha1;
    #[test]
    fn abc() {
        let msg = b"abc";
        let validator = byte_helper::hexstring_to_digest("a9993e364706816aba3e25717850c26c9cd0d89d").unwrap();
        let digest = sha1::gen_sha1_digest(msg, None).unwrap();
        assert_eq!(digest, validator);
    }
    
    #[test]
    fn greater_than_blocksize() {
        let msg = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
        let validator = byte_helper::hexstring_to_digest("a49b2446a02c645bf419f995b67091253a04a259").unwrap();
        let digest = sha1::gen_sha1_digest(msg, None).unwrap();
        assert_eq!(digest, validator);
    }

    #[test]
    fn  one_megabyte() {
        let msg = [b'a'; 1_000_000];
        let validator = byte_helper::hexstring_to_digest("34aa973cd4c4daa4f61eeb2bdbad27316534016f").unwrap();
        let digest = sha1::gen_sha1_digest(&msg, None).unwrap();
        assert_eq!(digest, validator);
    }
}

#[cfg(test)]
mod creating_hmac {
    use crate::byte_helper;
    use crate::sha1;

    #[test]
    fn simple() {
        let key = b"Jefe";
        let msg = b"what do ya want for nothing?";
        let validator = byte_helper::hexstring_to_digest("effcdf6ae5eb2fa2d27416d5f184df9c259a7c79").unwrap();
        let digest = sha1::gen_sha1_hmac(key, msg).unwrap();
        assert_eq!(digest, validator);
    }

    #[test]
    fn longer() {
        let key = byte_helper::hexstring_to_digest("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let msg = [0xdd_u8; 50];
        let validator = byte_helper::hexstring_to_digest("125d7342b9ac11cd91a39af48aa17b4f63f175d3").unwrap();
        let digest = sha1::gen_sha1_hmac(&key, &msg).unwrap();
        assert_eq!(digest, validator);
    }

    #[test]
    fn key_and_data_larger_than_blocksize() {
        let key = [0xaa_u8; 80];
        let msg = b"Test Using Larger Than Block-Size Key and Larger Than One Block-Size Data";
        // let validator = byte_helper::hexstring_to_digest("e8e99d0f45237d786d6bbaa7965c7808bbff1a91").unwrap();
        let digest = sha1::gen_sha1_hmac(&key, msg);
        assert!(digest.is_err());
    }
}

#[cfg(test)]
mod creating_hotp {
    use crate::sha1;

    #[test]
    fn simple() {
        let key = b"12345678901234567890";
        let validator: [u32; 10] = [
        755224, 287082, 359152, 969429, 338314,
        254676, 287922, 162583, 399871, 520489
        ];
        for counter in 0..10 {
            let otp = sha1::gen_sha1_hotp(key, counter, 6).unwrap();
            assert_eq!(otp, validator[counter as usize], "invalid OTP at counter {}", counter);
        }
    }
}

#[cfg(test)]
mod general_functionality {
    use crate::byte_helper;

    #[test]
    fn hexstring(){
        let hexstr = "11223344556677889910aabbccddeeff1a2a3a4a";
        let bytes = byte_helper::hexstring_to_digest(hexstr).unwrap();
        assert_eq!(bytes, [0x11,0x22,0x33,0x44,0x55,0x66,0x77,0x88,0x99,0x10,0xaa,0xbb,0xcc,0xdd,0xee,0xff,0x1a,0x2a,0x3a,0x4a]);
    }

    #[test]
    fn invalid_hexstring(){
        // This hex string contains invalid hex digits
        let bad_hexstr = "this doesn't make sense";
        let bytes = byte_helper::hexstring_to_digest(bad_hexstr);
        assert!(bytes.is_err());
    }

    #[test]
    fn odd_hexstring(){
        // This hex string only has 7 digits
        let hexstr = "abbccdd";
        // Function should add a `0` to the beginning of the hex string
        let bytes = byte_helper::hexstring_to_digest(hexstr);
        assert!(bytes.is_err());
    }

    #[test]
    fn valid_bytes_to_u32() {
        let num = byte_helper::bytes_to_u32([0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(num, 0xaabbccdd);
    }
}