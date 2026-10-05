//! HMAC-SHA256（RFC 2104 手写实现；M213 §11 Webhook 签名校验用）。
//!
//! 依赖 sha2（唯一新增依赖）；RFC 4231 测试向量锁定正确性。

use sha2::{Digest, Sha256};

/// HMAC-SHA256 → 小写 hex。
pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    hex(&hmac_sha256(key, message))
}

pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let mut hasher = Sha256::new();
        hasher.update(key);
        key_block[..32].copy_from_slice(&hasher.finalize());
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for index in 0..BLOCK {
        ipad[index] ^= key_block[index];
        opad[index] ^= key_block[index];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4231_case1() {
        // RFC 4231 Test Case 1：
        // key = 0x0b*20, data = "Hi There"
        let key = [0x0bu8; 20];
        let hmac = hmac_sha256_hex(&key, b"Hi There");
        assert_eq!(
            hmac,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn rfc4231_case2_short_key_data() {
        // RFC 4231 Test Case 2：key = "Jefe", data = "what do ya want for nothing?"
        let hmac = hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hmac,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn long_key_is_hashed_first() {
        // RFC 4231 Test Case 6：key = 0xaa*131（> block size ⇒ 先 hash）。
        let key = [0xaau8; 131];
        let hmac = hmac_sha256_hex(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        assert_eq!(
            hmac,
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }
}
