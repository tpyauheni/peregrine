#[cfg(feature = "bee2-rs")]
pub mod bee2rs;
pub mod x3dh;

use std::fmt::Debug;

use rand::RngCore;
use serde::{Deserialize, Serialize};

pub trait SymmetricCipher<Rng: RandomNumberGenerator>: Debug + Clone {
    fn encrypt(&mut self, data: &[u8], rng: &mut Rng) -> Box<[u8]>;
    fn decrypt(&mut self, data: &[u8]) -> Option<Box<[u8]>>;

    fn into_key_bytes(self) -> Box<[u8]>;
}

pub trait AsymmetricCipherPrivate<Rng: RandomNumberGenerator>: Debug + Clone {
    fn sign(&mut self, data: &[u8], rng: &mut Rng) -> Box<[u8]>;

    fn into_private_key_bytes(self) -> Box<[u8]>;
}

pub trait AsymmetricCipherPublic: Debug + Clone {
    fn verify(&mut self, data: &[u8], signature: &[u8]) -> bool;

    fn into_public_key_bytes(self) -> Box<[u8]>;
}

pub trait AsymmetricCipher<Rng: RandomNumberGenerator, CipherSym: SymmetricCipher<Rng>>:
    AsymmetricCipherPrivate<Rng> + AsymmetricCipherPublic
{
    fn diffie_hellman(&mut self, other_pubkey: &[u8]) -> CipherSym;
}

pub trait KeyDerivationAlgorithm<
    Rng: RandomNumberGenerator,
    CipherSym: SymmetricCipher<Rng>,
    CipherAsym: AsymmetricCipher<Rng, CipherSym>,
>
{
    fn as_symmetric_key(&mut self, password: &[u8]) -> CipherSym;
    fn as_asymmetric_key(&mut self, password: &[u8]) -> CipherAsym;
}

pub trait HashAlgorithm {
    fn hash(data: &[u8]) -> Box<[u8]>;

    fn update(&mut self, data: &[u8]);
    fn compute_hash(&mut self) -> Box<[u8]>;
}

pub trait RandomNumberGenerator {
    fn next_buffer(&mut self, buffer: &mut [u8]);
}

pub struct CryptographyAlgorithmSet<
    Rng: RandomNumberGenerator,
    CipherSym: SymmetricCipher<Rng>,
    CipherAsym: AsymmetricCipher<Rng, CipherSym>,
    Kdf: KeyDerivationAlgorithm<Rng, CipherSym, CipherAsym>,
    Hash: HashAlgorithm,
> {
    pub symmetric_cipher: CipherSym,
    pub asymmetric_cipher: CipherAsym,
    pub kdf: Kdf,
    pub hash: Hash,
    pub rng: Rng,
}

impl<
    Rng: RandomNumberGenerator,
    CipherSym: SymmetricCipher<Rng>,
    CipherAsym: AsymmetricCipher<Rng, CipherSym>,
    Kdf: KeyDerivationAlgorithm<Rng, CipherSym, CipherAsym>,
    Hash: HashAlgorithm,
> CryptographyAlgorithmSet<Rng, CipherSym, CipherAsym, Kdf, Hash>
{
    pub fn new(mut kdf: Kdf, hash: Hash, rng: Rng, password: &[u8]) -> Self {
        Self {
            symmetric_cipher: kdf.as_symmetric_key(password),
            asymmetric_cipher: kdf.as_asymmetric_key(password),
            kdf,
            hash,
            rng,
        }
    }
}

fn get_iv() -> [u8; 32] {
    let mut iv_buffer: [u8; 32] = [0; 32];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut iv_buffer);
    iv_buffer
}

#[cfg(feature = "bee2-rs")]
pub type Bee2RsCryptoset = bee2rs::CryptoSet;
#[cfg(not(feature = "bee2-rs"))]
pub type Bee2RsCryptoset = ();

pub fn bee2rs_cryptoset(password: &[u8], iv: Option<[u8; 32]>) -> Option<Bee2RsCryptoset> {
    if cfg!(feature = "bee2-rs") {
        Some(bee2rs::cryptoset(password, iv.unwrap_or_else(get_iv)))
    } else {
        None
    }
}

#[cfg(feature = "bee2-rs")]
pub fn default_cryptoset(password: &[u8], iv: Option<[u8; 32]>) -> bee2rs::CryptoSet {
    bee2rs::cryptoset(password, iv.unwrap_or_else(get_iv))
}
#[cfg(not(feature = "bee2-rs"))]
compile_error!("No cryptography algorithm sets configured");

#[cfg(feature = "bee2-rs")]
pub fn default_rng() -> bee2rs::DefaultRng {
    bee2rs::rng()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey {
    pub pk: Box<[u8]>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateKey {
    pub sk: Box<[u8]>,
}

pub fn hash(alg_name: &str, data: &[u8]) -> Option<Box<[u8]>> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::hash(data)),
        _ => None,
    }
}

pub fn cryptosets() -> Vec<String> {
    vec![
        #[cfg(feature = "bee2-rs")]
        "bycrypto".to_owned(),
    ]
}

pub fn generate_keypair(alg_name: &str) -> Option<(PrivateKey, PublicKey)> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::generate_keypair()),
        _ => None,
    }
}

pub fn sign(
    alg_name: &str,
    private_key: PrivateKey,
    public_key: PublicKey,
    data: &[u8],
) -> Option<Box<[u8]>> {
    let hash = hash("bycrypto", data)?;
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::sign(private_key, public_key, &hash)),
        _ => None,
    }
}

pub fn verify(
    alg_name: &str,
    public_key: PublicKey,
    data: &[u8],
    signature: &[u8],
) -> Option<bool> {
    let hash = hash(alg_name, data)?;
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::verify(public_key, &hash, signature)),
        _ => None,
    }
}

pub fn diffie_hellman(
    alg_name: &str,
    self_private_key: PrivateKey,
    self_public_key: PublicKey,
    other_public_key: PublicKey,
) -> Option<Box<[u8]>> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::diffie_hellman(
            self_private_key,
            self_public_key,
            other_public_key,
        )),
        _ => None,
    }
}

pub fn kdf(alg_name: &str, data: &[u8], result_len: usize) -> Option<Box<[u8]>> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::kdf(data, result_len)),
        _ => None,
    }
}

type ByteData = Box<[u8]>;

pub fn aead_wrap(
    alg_name: &str,
    plaintext: &[u8],
    key: PrivateKey,
    public_data: &[u8],
) -> Option<(ByteData, ByteData)> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::aead_wrap(plaintext, key, public_data)),
        _ => None,
    }
}

pub fn aead_unwrap(
    alg_name: &str,
    ciphertext: &[u8],
    public_data: &[u8],
    mac: &[u8],
    key: PrivateKey,
) -> Option<Option<Box<[u8]>>> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::aead_unwrap(ciphertext, public_data, mac, key)),
        _ => None,
    }
}

pub fn symmetric_encrypt(alg_name: &str, plaintext: &[u8], key: &[u8]) -> Option<Box<[u8]>> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::symmetric_encrypt(plaintext, key)),
        _ => None,
    }
}

pub fn symmetric_decrypt(
    alg_name: &str,
    ciphertext: Box<[u8]>,
    key: &[u8],
) -> Option<Option<Box<[u8]>>> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::symmetric_decrypt(ciphertext, key)),
        _ => None,
    }
}

pub enum KeyStrength {
    High,
    VeryHigh,
    ExtremelyHigh,
}

pub fn symmetric_genkey(alg_name: &str, strength: KeyStrength) -> Option<Box<[u8]>> {
    match alg_name {
        #[cfg(feature = "bee2-rs")]
        "bycrypto" => Some(bee2rs::symmetric_genkey(strength)),
        _ => None,
    }
}

pub fn supported_algorithms() -> Vec<&'static str> {
    vec![
        #[cfg(feature = "bee2-rs")]
        "bycrypto",
    ]
}

pub fn to_encryption_method(alg_name: &str) -> String {
    #[cfg(feature = "bee2-rs")]
    if alg_name == "bycrypto" {
        return "belt-ctr".to_owned();
    }
    "none".to_owned()
}

pub fn preferred_alogirthm() -> &'static str {
    supported_algorithms()[0]
}
