use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, KeyInit, OsRng, generic_array::GenericArray, rand_core::RngCore},
    Nonce,
};

pub fn encrypt(plaintext: &str, key: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let mut nonce = [0u8; 12];

    OsRng.fill_bytes(&mut nonce);

    let nonce_array = Nonce::from_slice(&nonce);

    let ciphertext = cipher.encrypt(nonce_array, plaintext.as_bytes()).unwrap();

    let mut result = nonce.to_vec();

    result.extend(ciphertext);
    result
}

pub fn decrypt(data: &[u8], key: &[u8]) -> Result<String, &'static str> {
    if data.len() < 12 {
        return Err("Data too short");
    }

    let (nonce_bytes, ciphertext) = data.split_at(12);

    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let nonce = Nonce::from_slice(nonce_bytes);

    let decrypted = cipher.decrypt(nonce, ciphertext).map_err(|_| "Decryption failed")?;

    String::from_utf8(decrypted).map_err(|_| "Invalid UTF-8")
}
