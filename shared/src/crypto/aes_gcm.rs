use aes_gcm::{aead::Aead, aes::{cipher::{BlockDecrypt, BlockEncrypt}, Aes128Enc, Aes192Dec, Aes192Enc, Aes256Enc}, Aes128Gcm, Aes256Gcm, KeyInit};

use crate::crypto::{get_iv, PrivateKey};

pub(super) fn aead_wrap(
    plaintext: &[u8],
    key: PrivateKey,
    public_data: &[u8],
) -> (Box<[u8]>, Box<[u8]>) {
    todo!()
}

pub(super) fn aead_unwrap(
    ciphertext: &[u8],
    public_data: &[u8],
    mac: &[u8],
    key: PrivateKey,
) -> Option<Box<[u8]>> {
    todo!()
}

pub(super) fn symmetric_encrypt(plaintext: &[u8], key: &[u8]) -> Box<[u8]> {
    let nonce: [u8; 12] = get_iv()[..12].try_into().unwrap();
    let mut result = vec![];
    result.extend(nonce);
    result.extend(if key.len() == 16 {
        let aes = Aes128Gcm::new(key.into());
        aes.encrypt(&nonce.into(), plaintext).unwrap()
    } else if key.len() == 24 {
        let mut plaintext: Vec<u8> = Vec::from(plaintext);
        let aes = Aes192Enc::new(key.into());
        for block in plaintext.chunks_mut(16) {
            aes.encrypt_block(block.into());
        }
        plaintext
    } else if key.len() == 32 {
        let aes = Aes256Gcm::new(key.into());
        aes.encrypt(&nonce.into(), plaintext).unwrap()
    } else {
        panic!();
    });
    result.into_boxed_slice()
}

pub(super) fn symmetric_decrypt(ciphertext: &[u8], key: &[u8]) -> Option<Box<[u8]>> {
    let Ok(nonce) = ciphertext[..12].try_into() else {
        return None;
    };
    let _: [u8; 12] = nonce;
    let ciphertext = &ciphertext[12..];
    let value = if key.len() == 16 {
        let aes = Aes128Gcm::new(key.into());
        aes.decrypt(&nonce.into(), ciphertext)
    } else if key.len() == 24 {
        let mut ciphertext: Vec<u8> = Vec::from(ciphertext);
        let aes = Aes192Dec::new(key.into());
        for block in ciphertext.chunks_mut(16) {
            aes.decrypt_block(block.into());
        }
        Ok(ciphertext)
    } else if key.len() == 32 {
        let aes = Aes256Gcm::new(key.into());
        aes.decrypt(&nonce.into(), ciphertext)
    } else {
        panic!();
    };
    let Ok(value) = value else {
        return None;
    };
    Some(value.into_boxed_slice())
}
