use crate::error::{AdbError, Result};
use pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, RsaPrivateKey, RsaPublicKey};
use std::fs;
use std::path::Path;

const KEY_LENGTH_BITS: usize = 2048;
const KEY_LENGTH_WORDS: usize = 64; // 2048 / 32

#[derive(Clone)]
pub struct AdbKeyPair {
    private_key: RsaPrivateKey,
    pub public_key_bytes: Vec<u8>,
}

impl AdbKeyPair {
    pub fn new(private_key: RsaPrivateKey) -> Self {
        let public_key = RsaPublicKey::from(&private_key);
        let public_key_bytes = Self::generate_adb_public_key_bytes(&public_key);
        Self {
            private_key,
            public_key_bytes,
        }
    }

    pub fn generate() -> Result<Self> {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, KEY_LENGTH_BITS)
            .map_err(|e| AdbError::Auth(format!("Failed to generate RSA key: {e}")))?;
        Ok(Self::new(private_key))
    }

    pub fn read_from_file(private_key_path: impl AsRef<Path>) -> Result<Self> {
        let pem_str = fs::read_to_string(private_key_path)
            .map_err(|e| AdbError::Auth(format!("Failed to read private key file: {e}")))?;
        let private_key = RsaPrivateKey::from_pkcs8_pem(&pem_str)
            .map_err(|e| AdbError::Auth(format!("Failed to parse PKCS#8 private key: {e}")))?;
        Ok(Self::new(private_key))
    }

    pub fn save_to_file(&self, private_key_path: impl AsRef<Path>) -> Result<()> {
        let pem = self
            .private_key
            .to_pkcs8_pem(pkcs8::LineEnding::LF)
            .map_err(|e| AdbError::Auth(format!("Failed to encode private key to PKCS#8 PEM: {e}")))?;
        fs::write(private_key_path, pem.as_str())?;
        Ok(())
    }

    pub fn sign_payload(&self, token: &[u8]) -> Result<Vec<u8>> {
        // SHA-1 DigestInfo header (15 bytes) matching Kotlin SIGNATURE_PADDING
        let sha1_digest_info: [u8; 15] = [
            0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2B, 0x0E, 0x03, 0x02, 0x1A, 0x05, 0x00, 0x04, 0x14,
        ];

        // Construct 256-byte PKCS#1 v1.5 padded block:
        // [0x00, 0x01, 0xFF * 218, 0x00, SHA1_HEADER (15 bytes), token (20 bytes)]
        let mut padded = Vec::with_capacity(256);
        padded.push(0x00);
        padded.push(0x01);
        padded.extend(std::iter::repeat(0xFF).take(256 - 3 - 15 - token.len()));
        padded.push(0x00);
        padded.extend_from_slice(&sha1_digest_info);
        padded.extend_from_slice(token);

        let padded_biguint = BigUint::from_bytes_be(&padded);
        let mut rng = rand::thread_rng();

        // Perform raw RSA modular exponentiation s = m^d mod n
        let sig_biguint = rsa::hazmat::rsa_decrypt(Some(&mut rng), &self.private_key, &padded_biguint)
            .map_err(|e| AdbError::Auth(format!("RSA raw signing failed: {e}")))?;

        let mut sig_bytes = sig_biguint.to_bytes_be();
        if sig_bytes.len() < 256 {
            let mut full_sig = vec![0u8; 256 - sig_bytes.len()];
            full_sig.append(&mut sig_bytes);
            Ok(full_sig)
        } else {
            Ok(sig_bytes)
        }
    }

    fn generate_adb_public_key_bytes(public_key: &RsaPublicKey) -> Vec<u8> {
        let n = public_key.n();
        let e = public_key.e();

        let mut buf = Vec::with_capacity(524 + 16);

        // 1. len: u32 (64 words)
        buf.extend_from_slice(&(KEY_LENGTH_WORDS as u32).to_le_bytes());

        // 2. n0inv: u32 = -N^{-1} mod 2^32
        let n0inv = Self::calculate_n0inv(n);
        buf.extend_from_slice(&n0inv.to_le_bytes());

        // 3. n: [u32; 64] (Modulus N as 64 little-endian 32-bit words)
        let n_words = Self::biguint_to_le_u32_words(n, KEY_LENGTH_WORDS);
        for word in n_words {
            buf.extend_from_slice(&word.to_le_bytes());
        }

        // 4. rr: [u32; 64] (R^2 mod N where R = 2^2048)
        let r = BigUint::from(1u32) << 2048;
        let r_squared = (&r * &r) % n;
        let rr_words = Self::biguint_to_le_u32_words(&r_squared, KEY_LENGTH_WORDS);
        for word in rr_words {
            buf.extend_from_slice(&word.to_le_bytes());
        }

        // 5. exponent: u32
        let exponent_bytes = e.to_bytes_le();
        let mut exp_u32_bytes = [0u8; 4];
        let len = exponent_bytes.len().min(4);
        exp_u32_bytes[..len].copy_from_slice(&exponent_bytes[..len]);
        let exponent_u32 = u32::from_le_bytes(exp_u32_bytes);
        buf.extend_from_slice(&exponent_u32.to_le_bytes());

        // Null terminator for RSAPublicKey struct + system user string
        buf.push(0);
        buf.extend_from_slice(b" host@radb\0");

        buf
    }

    fn calculate_n0inv(n: &BigUint) -> u32 {
        let two32 = BigUint::from(1u64 << 32);
        let n_mod_232 = n % &two32;
        let mut bytes = n_mod_232.to_bytes_le();
        bytes.resize(8, 0);
        let n_u64 = u64::from_le_bytes(bytes.try_into().unwrap());

        // Extended Euclidean Algorithm for modular inverse of n_u64 mod 2^32
        let mut inv: u64 = 1;
        for _ in 0..31 {
            inv = inv.wrapping_mul(inv.wrapping_mul(n_u64));
        }

        let n0inv = (0u64.wrapping_sub(inv)) & 0xFFFF_FFFF;
        n0inv as u32
    }

    fn biguint_to_le_u32_words(val: &BigUint, word_count: usize) -> Vec<u32> {
        let mut bytes = val.to_bytes_le();
        bytes.resize(word_count * 4, 0);
        bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect()
    }
}
