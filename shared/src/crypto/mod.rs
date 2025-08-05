#[cfg(feature = "aes-gcm")]
pub mod aes_gcm;
#[cfg(feature = "bee2-rs")]
pub mod bee2rs;
pub mod x3dh;

use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use rand::RngCore;
use serde::{Deserialize, Serialize};

fn get_iv() -> [u8; 32] {
    let mut iv_buffer: [u8; 32] = [0; 32];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut iv_buffer);
    iv_buffer
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey {
    pub pk: Box<[u8]>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateKey {
    pub sk: Box<[u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoAlgorithms {
    pub hash: String,
    pub kdf: String,
    pub diffie_hellman: String,
    pub signature: String,
    pub symmetric_encryption: String,
    pub aead: String,
    pub rng: String,
}

impl FromStr for CryptoAlgorithms {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_string(s.to_owned()))
    }
}

impl Display for CryptoAlgorithms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        result += &self.hash.replace("::", "__");
        result.push('.');
        result += &self.kdf.replace("::", "__");
        result.push('.');
        result += &self.diffie_hellman.replace("::", "__");
        result.push('.');
        result += &self.signature.replace("::", "__");
        result.push('.');
        result += &self.symmetric_encryption.replace("::", "__");
        result.push('.');
        result += &self.aead.replace("::", "__");
        result.push('.');
        result += &self.rng.replace("::", "__");
        f.write_str(&result)
    }
}

impl CryptoAlgorithms {
    pub fn from_string(alg_name: String) -> Self {
        Self {
            hash: alg_name.clone(),
            kdf: alg_name.clone(),
            diffie_hellman: alg_name.clone(),
            signature: alg_name.clone(),
            symmetric_encryption: alg_name.clone(),
            aead: alg_name.clone(),
            rng: alg_name,
        }
    }

    #[cfg(feature = "bee2-rs")]
    pub fn prequantum_bee2rs() -> Self {
        Self {
            hash: "bee2-rs::bash512".to_owned(),
            kdf: "bee2-rs::pbkdf2".to_owned(),
            diffie_hellman: "bee2-rs::bignb3".to_owned(),
            signature: "bee2-rs::bignb3".to_owned(),
            symmetric_encryption: "bee2-rs::belt-ctr".to_owned(),
            aead: "bee2-rs::belt256-dwp".to_owned(),
            rng: "bee2-rs::belt-ctr".to_owned(),
        }
    }

    #[cfg(all(feature = "aes-gcm", feature = "curve25519-dalek", feature = "pbkdf2"))]
    pub fn prequantum_standard() -> Self {
        Self {
            hash: "rustcrypto::aes-gcm".to_owned(),
            kdf: "rustcrypto::pbkdf2".to_owned(),
            diffie_hellman: "dalek::x25519".to_owned(),
            signature: "dalek::ed25519".to_owned(),
            symmetric_encryption: "rustcrypto::aes-gcm".to_owned(),
            aead: "rustcrypto::aes-gcm".to_owned(),
            rng: "default".to_owned(),
        }
    }

    pub fn encryption_method(&self) -> String {
        self.symmetric_encryption.split_once("::").map_or_else(
            || self.symmetric_encryption.clone(),
            |(_, value)| value.to_owned(),
        )
    }
}

pub fn hash(algorithms: &CryptoAlgorithms, data: &[u8]) -> Option<Box<[u8]>> {
    match &algorithms.hash as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::bash512" => Some(bee2rs::hash(data)),
        _ => None,
    }
}

pub fn generate_keypair(algorithms: &CryptoAlgorithms) -> Option<(PrivateKey, PublicKey)> {
    match &algorithms.rng as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::belt-ctr" => Some(bee2rs::generate_keypair(&algorithms.signature)),
        _ => None,
    }
}

pub fn sign(
    algorithms: &CryptoAlgorithms,
    private_key: PrivateKey,
    public_key: PublicKey,
    data: &[u8],
) -> Option<Box<[u8]>> {
    let hash = hash(algorithms, data)?;
    match &algorithms.signature as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::bignb3" => Some(bee2rs::sign(private_key, public_key, &hash)),
        _ => None,
    }
}

pub fn verify(
    algorithms: &CryptoAlgorithms,
    public_key: PublicKey,
    data: &[u8],
    signature: &[u8],
) -> Option<bool> {
    let hash = hash(algorithms, data)?;
    match &algorithms.signature as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::bignb3" => Some(bee2rs::verify(public_key, &hash, signature)),
        _ => None,
    }
}

pub fn diffie_hellman(
    algorithms: &CryptoAlgorithms,
    self_private_key: PrivateKey,
    self_public_key: PublicKey,
    other_public_key: PublicKey,
) -> Option<Box<[u8]>> {
    match &algorithms.diffie_hellman as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::bignb3" => Some(bee2rs::diffie_hellman(
            self_private_key,
            self_public_key,
            other_public_key,
        )),
        _ => None,
    }
}

pub fn kdf(algorithms: &CryptoAlgorithms, data: &[u8], result_len: usize) -> Option<Box<[u8]>> {
    match &algorithms.kdf as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::pbkdf2" => Some(bee2rs::kdf(data, result_len)),
        _ => None,
    }
}

pub fn kdf_keypair(algorithms: &CryptoAlgorithms, data: &[u8]) -> Option<(PrivateKey, PublicKey)> {
    match &algorithms.kdf as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::pbkdf2" => Some(bee2rs::kdf_keypair(&algorithms.signature, data)),
        _ => None,
    }
}

type ByteData = Box<[u8]>;

pub fn aead_wrap(
    algorithms: &CryptoAlgorithms,
    plaintext: &[u8],
    key: PrivateKey,
    public_data: &[u8],
) -> Option<(ByteData, ByteData)> {
    match &algorithms.aead as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::belt256-dwp" => Some(bee2rs::aead_wrap(plaintext, key, public_data)),
        #[cfg(feature = "aes-gcm")]
        "rustcrypto::aes-gcm" => Some(aes_gcm::aead_wrap(plaintext, key, public_data)),
        _ => None,
    }
}

pub fn aead_unwrap(
    algorithms: &CryptoAlgorithms,
    ciphertext: &[u8],
    public_data: &[u8],
    mac: &[u8],
    key: PrivateKey,
) -> Option<Option<Box<[u8]>>> {
    match &algorithms.aead as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::belt256-dwp" => Some(bee2rs::aead_unwrap(ciphertext, public_data, mac, key)),
        #[cfg(feature = "aes-gcm")]
        "rustcrypto::aes-gcm" => Some(aes_gcm::aead_unwrap(ciphertext, public_data, mac, key)),
        _ => None,
    }
}

pub fn symmetric_encrypt(
    algorithms: &CryptoAlgorithms,
    plaintext: &[u8],
    key: &[u8],
) -> Option<Box<[u8]>> {
    match &algorithms.symmetric_encryption as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::belt-ctr" => Some(bee2rs::symmetric_encrypt(plaintext, key)),
        #[cfg(feature = "aes-gcm")]
        "rustcrypto::aes-gcm" => Some(aes_gcm::symmetric_encrypt(plaintext, key)),
        _ => None,
    }
}

pub fn symmetric_decrypt(
    algorithms: &CryptoAlgorithms,
    ciphertext: &[u8],
    key: &[u8],
) -> Option<Option<Box<[u8]>>> {
    match &algorithms.symmetric_encryption as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::belt-ctr" => Some(bee2rs::symmetric_decrypt(ciphertext, key)),
        #[cfg(feature = "aes-gcm")]
        "rustcrypto::aes-gcm" => Some(aes_gcm::symmetric_decrypt(ciphertext, key)),
        _ => None,
    }
}

pub enum KeyStrength {
    High,
    VeryHigh,
    ExtremelyHigh,
}

pub fn symmetric_genkey(algorithms: &CryptoAlgorithms, strength: KeyStrength) -> Option<Box<[u8]>> {
    match &algorithms.rng as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::belt-ctr" => Some(bee2rs::symmetric_genkey(
            &algorithms.symmetric_encryption,
            strength,
        )),
        _ => None,
    }
}

pub fn rng_fill(algorithms: &CryptoAlgorithms, buffer: &mut [u8]) -> Option<()> {
    match &algorithms.rng as &str {
        #[cfg(feature = "bee2-rs")]
        "bee2-rs::belt-ctr" => {
            bee2rs::rng_fill(buffer);
            Some(())
        }
        "default" => {
            rand::rng().fill_bytes(buffer);
            Some(())
        }
        _ => None,
    }
}

pub fn supported_algorithms() -> Vec<CryptoAlgorithms> {
    vec![
        #[cfg(feature = "bee2-rs")]
        CryptoAlgorithms::prequantum_bee2rs(),
        #[cfg(all(feature = "aes-gcm", feature = "curve25519-dalek", feature = "pbkdf2"))]
        CryptoAlgorithms::prequantum_standard(),
    ]
}

pub fn preferred_alogirthm() -> CryptoAlgorithms {
    supported_algorithms()[0].clone()
}
