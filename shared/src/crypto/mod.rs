#[cfg(feature = "bee2-rs")]
pub mod bee2rs;

use std::fmt::Debug;

use rand::RngCore;

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
