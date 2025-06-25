use crate::crypto::get_iv;

use super::{
    AsymmetricCipher, AsymmetricCipherPrivate, AsymmetricCipherPublic, CryptographyAlgorithmSet,
    HashAlgorithm, KeyDerivationAlgorithm, RandomNumberGenerator, SymmetricCipher,
};
use bee2_rs::{
    bash_hash::Bash512,
    belt::{BeltDwp, BeltEncryptionAlgorithm, BeltKey256},
    bign::{BignKey, BignParameters, BignParametersConfiguration},
    brng::{Brng, CtrRng},
    errors::Bee2Result,
};

#[derive(Debug, Clone)]
pub struct Belt256 {
    key: [u8; 32],
    belt_key: BeltKey256,
}

impl Belt256 {
    pub fn new(key: [u8; 32]) -> Self {
        let belt_key = BeltKey256::new(key);
        Self { key, belt_key }
    }
}

// TODO: Add HMAC.
impl SymmetricCipher<CtrRng> for Belt256 {
    fn encrypt(&mut self, data: &[u8], rng: &mut CtrRng) -> Box<[u8]> {
        let mut iv = vec![];
        iv.reserve_exact(16);
        iv.extend([0u8; 16]);
        RandomNumberGenerator::next_buffer(rng, &mut iv);
        let ciphertext = self
            .belt_key
            .clone()
            .ctr((*iv.as_slice()).try_into().unwrap())
            .encrypt(data);
        let mut result = vec![];
        result.reserve_exact(iv.len() + ciphertext.len());
        result.extend(iv);
        result.extend(ciphertext);
        result.into_boxed_slice()
    }

    fn decrypt(&mut self, data: &[u8]) -> Option<Box<[u8]>> {
        let iv = &data[..16];
        let Ok(iv) = std::convert::TryInto::<&[u8; 16]>::try_into(iv) else {
            return None;
        };
        let Ok(data) = self.belt_key.clone().ctr(*iv).decrypt(data.into()) else {
            return None;
        };
        Some(data)
    }

    fn into_key_bytes(self) -> Box<[u8]> {
        Box::new(self.key)
    }
}

#[derive(Debug, Clone)]
pub struct Bign {
    pub_key: Box<[u8]>,
    // Option<`private_key`, `bign_key`>
    priv_data: Option<(Box<[u8]>, BignKey)>,
}

impl Bign {
    pub fn try_new(public_key: &[u8], private_key: Option<&[u8]>) -> Bee2Result<Self> {
        if let Some(private_key) = private_key {
            let bign_key = BignKey::try_load(
                BignParameters::try_new(BignParametersConfiguration::B3)?,
                public_key,
                private_key,
            )?;
            let priv_key: Box<[u8]> = private_key.into();
            Ok(Self {
                pub_key: public_key.into(),
                priv_data: Some((priv_key, bign_key)),
            })
        } else {
            Ok(Self {
                pub_key: public_key.into(),
                priv_data: None,
            })
        }
    }
}

impl AsymmetricCipherPublic for Bign {
    fn verify(&mut self, data: &[u8], signature: &[u8]) -> bool {
        let bign_key = BignKey {
            public_key: Box::new([]),
            private_key: Box::new([]),
            params: BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
        };
        let hash = Bash512::hash(data).unwrap();
        bign_key.verify(&self.pub_key, &hash, signature).is_ok()
    }

    fn into_public_key_bytes(self) -> Box<[u8]> {
        self.pub_key
    }
}

impl AsymmetricCipherPrivate<CtrRng> for Bign {
    fn sign(&mut self, data: &[u8], rng: &mut CtrRng) -> Box<[u8]> {
        let hash = Bash512::hash(data).unwrap();
        self.priv_data.as_ref().unwrap().1.sign(&hash, rng).unwrap()
    }

    fn into_private_key_bytes(self) -> Box<[u8]> {
        self.priv_data.unwrap().0
    }
}

impl AsymmetricCipher<CtrRng, Belt256> for Bign {
    fn diffie_hellman(&mut self, other_pubkey: &[u8]) -> Belt256 {
        let key_bytes = self
            .priv_data
            .as_mut()
            .unwrap()
            .1
            .diffie_hellman(other_pubkey, 32)
            .unwrap();
        Belt256::new((*key_bytes).try_into().unwrap())
    }
}

pub struct Pbkdf2 {}

impl KeyDerivationAlgorithm<CtrRng, Belt256, Bign> for Pbkdf2 {
    fn as_symmetric_key(&mut self, password: &[u8]) -> Belt256 {
        // `10_000` is specified as recommended minimum in standard. There may be more rainbow
        // tables for that specific value, so I'm going with `10000` + random value.
        let belt_key = BeltKey256::pbkdf2(password, 10447, &[]).unwrap();
        Belt256 {
            key: *belt_key.get_bytes(),
            belt_key,
        }
    }

    fn as_asymmetric_key(&mut self, password: &[u8]) -> Bign {
        let tmp_key = BeltKey256::pbkdf2(password, 10448, &[]).unwrap();
        let key_bytes: [u8; 32] = *tmp_key.get_bytes();
        let mut rng = CtrRng::new(key_bytes, None);
        let bign_key = BignKey::try_new(
            BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
            &mut rng,
        )
        .unwrap();
        Bign {
            pub_key: bign_key.public_key.clone(),
            priv_data: Some((bign_key.private_key.clone(), bign_key)),
        }
    }
}

impl HashAlgorithm for Bash512 {
    fn hash(data: &[u8]) -> Box<[u8]> {
        Self::hash(data).unwrap()
    }

    fn update(&mut self, data: &[u8]) {
        self.update(data);
    }

    fn compute_hash(&mut self) -> Box<[u8]> {
        self.get_hash()
    }
}

impl RandomNumberGenerator for CtrRng {
    fn next_buffer(&mut self, buffer: &mut [u8]) {
        Brng::next_buffer(self, buffer);
    }
}

pub type DefaultRng = CtrRng;

pub(crate) fn rng() -> DefaultRng {
    CtrRng::new(get_iv(), None)
}

pub type CryptoSet = CryptographyAlgorithmSet<DefaultRng, Belt256, Bign, Pbkdf2, Bash512>;

pub(crate) fn cryptoset(password: &[u8], iv: [u8; 32]) -> CryptoSet {
    let mut kdf = Pbkdf2 {};
    let hash = Bash512::new();
    let rng = CtrRng::new(kdf.as_symmetric_key(password).key, Some(iv));
    CryptographyAlgorithmSet::new(kdf, hash, rng, password)
}

pub(crate) fn hash(data: &[u8]) -> Box<[u8]> {
    Bash512::hash(data).unwrap()
}

pub(crate) fn generate_keypair() -> (Box<[u8]>, Box<[u8]>) {
    let key = BignKey::try_new(BignParameters::try_new(BignParametersConfiguration::B3).unwrap(), &mut rng()).unwrap();
    (key.private_key, key.public_key)
}

pub(crate) fn sign(private_key: &[u8], public_key: &[u8], hash: &[u8]) -> Box<[u8]> {
    let key = BignKey::try_load(BignParameters::try_new(BignParametersConfiguration::B3).unwrap(), public_key, private_key).unwrap();
    key.sign(hash, &mut rng()).unwrap()
}

pub(crate) fn verify(public_key: &[u8], hash: &[u8], signature: &[u8]) -> bool {
    let key = BignKey {
        private_key: Box::new([]),
        public_key: Box::from(public_key),
        params: BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
    };
    key.verify(public_key, hash, signature).is_ok()
}

pub(crate) fn diffie_hellman(self_public_key: &[u8], self_private_key: &[u8], other_public_key: &[u8]) -> Box<[u8]> {
    let mut key = BignKey::try_load(
        BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
        self_public_key,
        self_private_key,
    ).unwrap();
    key.diffie_hellman(other_public_key, 32).unwrap()
}

// TODO: Add to upstream library.
pub(crate) fn kdf(data: &[u8], result_len: usize) -> Box<[u8]> {
    let mut result = vec![];

    for _ in 0..=result_len / 32 {
        let mut key = vec![0u8; 32];
        let code = unsafe {
            bee2_rs::bindings::bakeKDF(
                key.as_mut_ptr(),
                data.as_ptr(),
                data.len(),
                std::ptr::null(),
                0,
                0,
            )
        };
        assert!(code == 0);
        result.extend(key);
    }

    Box::from(&result[..result_len])
}

pub(crate) fn aead_wrap(plaintext: &[u8], key: &[u8], public_data: &[u8]) -> (Box<[u8]>, Box<[u8]>) {
    let key = BeltKey256::new(key.try_into().unwrap());
    let iv = key.clone().to_key128().get_bytes();
    let (ciphertext, mac) = BeltDwp::wrap(plaintext, public_data, &key, *iv).unwrap();
    (ciphertext, Box::from(mac))
}

pub(crate) fn aead_unwrap(ciphertext: &[u8], public_data: &[u8], mac: &[u8], key: &[u8]) -> Option<Box<[u8]>> {
    let key = BeltKey256::new(key.try_into().unwrap());
    let iv = key.clone().to_key128().get_bytes();
    match BeltDwp::unwrap(ciphertext, public_data, mac.try_into().unwrap(), &key, *iv) {
        Ok(data) => Some(data),
        Err(_) => None,
    }
}
