use crate::crypto::{KeyStrength, PrivateKey, PublicKey, get_iv};

use bee2_rs::{
    bash_hash::Bash512,
    belt::{BeltDwp, BeltEncryptionAlgorithm, BeltKey128, BeltKey192, BeltKey256},
    bign::{BignKey, BignParameters, BignParametersConfiguration},
    brng::{Brng, CtrRng},
};

fn rng() -> CtrRng {
    CtrRng::new(get_iv(), None)
}

pub(super) fn hash(data: &[u8]) -> Box<[u8]> {
    Bash512::hash(data).unwrap()
}

pub(super) fn generate_keypair(asymmetric_algorithm: &str) -> (PrivateKey, PublicKey) {
    assert_eq!(asymmetric_algorithm, "bee2-rs::bignb3");
    let key = BignKey::try_new(
        BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
        &mut rng(),
    )
    .unwrap();
    (
        PrivateKey {
            sk: key.private_key,
        },
        PublicKey { pk: key.public_key },
    )
}

pub(super) fn sign(private_key: PrivateKey, public_key: PublicKey, hash: &[u8]) -> Box<[u8]> {
    let key = BignKey::try_load(
        BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
        &public_key.pk,
        &private_key.sk,
    )
    .unwrap();
    key.sign(hash, &mut rng()).unwrap()
}

pub(super) fn verify(public_key: PublicKey, hash: &[u8], signature: &[u8]) -> bool {
    let key = BignKey {
        private_key: Box::new([]),
        public_key: Box::new([]),
        params: BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
    };
    key.verify(&public_key.pk, hash, signature).is_ok()
}

pub(super) fn diffie_hellman(
    self_private_key: PrivateKey,
    self_public_key: PublicKey,
    other_public_key: PublicKey,
) -> Box<[u8]> {
    let mut key = BignKey::try_load(
        BignParameters::try_new(BignParametersConfiguration::B3).unwrap(),
        &self_public_key.pk,
        &self_private_key.sk,
    )
    .unwrap();
    key.diffie_hellman(&other_public_key.pk, 32).unwrap()
}

pub(super) fn kdf_keypair(asymmetric_algorithm: &str, data: &[u8]) -> (PrivateKey, PublicKey) {
    assert_eq!(asymmetric_algorithm, "bee2-rs::bignb3");
    let mut rng = CtrRng::new((&kdf(data, 32) as &[u8]).try_into().unwrap(), None);
    let key = BignKey::try_new(BignParameters::try_new(BignParametersConfiguration::B3).unwrap(), &mut rng).unwrap();
    (PrivateKey { sk: key.private_key }, PublicKey { pk: key.public_key })
}

// TODO: Add to upstream library.
pub(super) fn kdf(data: &[u8], result_len: usize) -> Box<[u8]> {
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

pub(super) fn aead_wrap(
    plaintext: &[u8],
    key: PrivateKey,
    public_data: &[u8],
) -> (Box<[u8]>, Box<[u8]>) {
    let key = BeltKey256::new(((&key.sk) as &[u8]).try_into().unwrap());
    let iv = key.clone().to_key128().get_bytes();
    let (ciphertext, mac) = BeltDwp::wrap(plaintext, public_data, &key, *iv).unwrap();
    (ciphertext, Box::from(mac))
}

pub(super) fn aead_unwrap(
    ciphertext: &[u8],
    public_data: &[u8],
    mac: &[u8],
    key: PrivateKey,
) -> Option<Box<[u8]>> {
    let key = BeltKey256::new(((&key.sk) as &[u8]).try_into().unwrap());
    let iv = key.clone().to_key128().get_bytes();
    BeltDwp::unwrap(ciphertext, public_data, mac.try_into().unwrap(), &key, *iv).ok()
}

pub(super) fn symmetric_encrypt(plaintext: &[u8], key: &[u8]) -> Box<[u8]> {
    let iv = get_iv();
    let iv = iv[..16].try_into().unwrap();
    let mut result = Vec::from(iv);
    result.extend(if key.len() == 32 {
        let key = BeltKey256::new(key.try_into().unwrap());
        let mut ctr = key.ctr(iv);
        ctr.encrypt(plaintext)
    } else if key.len() == 24 {
        let key = BeltKey192::new(key.try_into().unwrap());
        let mut ctr = key.ctr(iv);
        ctr.encrypt(plaintext)
    } else if key.len() == 16 {
        let key = BeltKey128::new(key.try_into().unwrap());
        let mut ctr = key.ctr(iv);
        ctr.encrypt(plaintext)
    } else {
        panic!();
    });
    result.into_boxed_slice()
}

pub(super) fn symmetric_decrypt(ciphertext: &[u8], key: &[u8]) -> Option<Box<[u8]>> {
    let Ok(iv) = ciphertext[..16].try_into() else {
        return None;
    };
    let value = if key.len() == 32 {
        let key = BeltKey256::new(key.try_into().unwrap());
        let mut ctr = key.ctr(iv);
        ctr.decrypt(ciphertext[16..].iter().cloned().collect())
    } else if key.len() == 24 {
        let key = BeltKey192::new(key.try_into().unwrap());
        let mut ctr = key.ctr(iv);
        ctr.decrypt(ciphertext[16..].iter().cloned().collect())
    } else if key.len() == 16 {
        let key = BeltKey128::new(key.try_into().unwrap());
        let mut ctr = key.ctr(iv);
        ctr.decrypt(ciphertext[16..].iter().cloned().collect())
    } else {
        panic!();
    };

    if let Err(ref err) = value {
        println!("Failed to decrypt: {err:?}");
    }

    value.ok()
}

pub fn symmetric_genkey(symmetric_algorithm: &str, strength: KeyStrength) -> Box<[u8]> {
    assert_eq!(symmetric_algorithm, "bee2-rs::belt-ctr");
    let mut rng = CtrRng::new(get_iv(), None);
    match strength {
        KeyStrength::High => {
            let mut key: [u8; 16] = [0; _];
            rng.next_buffer(&mut key);
            Box::new(key)
        }
        KeyStrength::VeryHigh => {
            let mut key: [u8; 24] = [0; _];
            rng.next_buffer(&mut key);
            Box::new(key)
        }
        KeyStrength::ExtremelyHigh => {
            let mut key: [u8; 32] = [0; _];
            rng.next_buffer(&mut key);
            Box::new(key)
        }
    }
}

pub fn rng_fill(buffer: &mut [u8]) {
    let mut rng = rng();
    rng.next_buffer(buffer);
}
