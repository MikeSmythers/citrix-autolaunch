use const_random::const_random;
use crypter::{decrypt, encrypt};
use hex;

/// Generate a key at compile time which persists for all runs (128-bit key)\
/// - Re-compiling code will generate a new key
/// - Otherwise, the key persists for future runs
fn get_key() -> String {
    format!("{:x}", const_random!(u128))
}

/// Encrypt a string
/// - Accepts a string to encrypt
/// - Returns the encrypted string as a hex String
/// - Uses a key generated at compile time
/// - Returns an error String if encryption fails
pub fn encrypt_string(data: &str) -> Result<String, String> {
    match encrypt(&get_key(), data) {
        Some(enc) => return Ok(hex::encode(enc)),
        None => return Err(format!("Encryption failed: unknown error")),
    }
}

/// Decrypt a string
/// - Accepts a hex String to decrypt
/// - Returns the decrypted string as a String
/// - Uses a key generated at compile time
///   - Returns an error String if decryption fails
pub fn decrypt_string(data: String) -> Result<String, String> {
    // Try to decode the input string
    if let Ok(i) = hex::decode(data) {
        // Try to decrypt the decoded string
        match decrypt(&get_key(), i) {
            // Try to convert the decrypted bytes to a String
            Some(dec) => match String::from_utf8(dec) {
                Ok(o) => Ok(o),
                Err(e) => Err(format!("Decryption conversion failed: {:?}", e)),
            },
            None => Err(format!("Decryption failed: unknown error")),
        }
    } else {
        Err(format!("Decryption hex conversion failed"))
    }
}
