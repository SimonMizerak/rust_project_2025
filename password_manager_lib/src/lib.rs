pub mod crypto;
pub mod encryption;
pub mod database;

#[cfg(test)]
mod tests {
    use super::crypto::*;
    use super::encryption::*;

    #[test]
    fn test_password_hashing() {
        let password = "DocentoveHeslo";

        let (hash, _) = hash_password(password);

        assert!(verify_password(&hash, password));
    }

    #[test]
    fn test_encryption_decryption() {
        let key = [0u8; 32]; // We shouldn't use zeroed key in production

        let plaintext = "DocentoveHeslo2";

        let encrypted = encrypt(plaintext, &key);

        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert_eq!(plaintext, decrypted);
    }
}