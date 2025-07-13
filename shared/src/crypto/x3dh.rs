use std::{error::Error, fmt::Display};

use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct X3DhReceiverKeysPublic {
    pub algorithms: CryptoAlgorithms,
    pub ik: PublicKey,
    pub spk: PublicKey,
    pub spk_signature: Box<[u8]>,
    pub opks: Vec<PublicKey>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct X3DhReceiverKeysPrivate {
    pub ik: PrivateKey,
    pub spk: PrivateKey,
    pub opks: Vec<PrivateKey>,
}

pub fn generate_receiver_keys(
    algorithms: &CryptoAlgorithms,
) -> Option<(X3DhReceiverKeysPrivate, X3DhReceiverKeysPublic)> {
    let (ik_priv, ik_pub) = generate_keypair(algorithms)?;
    let (spk_priv, spk_pub) = generate_keypair(algorithms)?;
    let spk_signature = sign(algorithms, ik_priv.clone(), ik_pub.clone(), &spk_pub.pk)?;

    let mut opks_priv = Vec::new();
    let mut opks_pub = Vec::new();
    for _ in 0..10 {
        let (opk_priv, opk_pub) = generate_keypair(algorithms)?;
        opks_priv.push(opk_priv);
        opks_pub.push(opk_pub);
    }

    Some((
        X3DhReceiverKeysPrivate {
            ik: ik_priv,
            spk: spk_priv,
            opks: opks_priv,
        },
        X3DhReceiverKeysPublic {
            algorithms: algorithms.clone(),
            ik: ik_pub,
            spk: spk_pub,
            opks: opks_pub,
            spk_signature,
        },
    ))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct X3DhData {
    pub ek_pub: PublicKey,
    pub opk_id: Option<u32>,
    pub ciphertext: Box<[u8]>,
    pub mac: Box<[u8]>,
    pub signature: Box<[u8]>,
}

#[derive(Debug, Clone)]
pub enum X3DhError {
    AlgorithmNotSupported,
    InvalidSignature,
    DecryptionFailure,
    InvalidOpkKeyId,
}

impl Display for X3DhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            Self::AlgorithmNotSupported => "Algorithm not supported",
            Self::InvalidSignature => "Invalid signature",
            Self::DecryptionFailure => "Decryption failure",
            Self::InvalidOpkKeyId => "Invalid OPK key id",
        })
    }
}

impl Error for X3DhError {}

pub fn encode_x3dh(
    data: &[u8],
    ik_priv: PrivateKey,
    ik_pub: PublicKey,
    other_keys: X3DhReceiverKeysPublic,
) -> Result<X3DhData, X3DhError> {
    let algorithms= &other_keys.algorithms;

    match verify(
        algorithms,
        other_keys.ik.clone(),
        &other_keys.spk.pk,
        &other_keys.spk_signature,
    ) {
        Some(true) => {}
        Some(false) => return Err(X3DhError::InvalidSignature),
        None => return Err(X3DhError::AlgorithmNotSupported),
    }

    let Some((ek_priv, ek_pub)) = generate_keypair(algorithms) else {
        return Err(X3DhError::AlgorithmNotSupported);
    };

    let dh1 = diffie_hellman(
        algorithms,
        ik_priv.clone(),
        ik_pub.clone(),
        other_keys.spk.clone(),
    )
    .unwrap();
    let dh2 = diffie_hellman(
        algorithms,
        ek_priv.clone(),
        ek_pub.clone(),
        other_keys.ik.clone(),
    )
    .unwrap();
    let dh3 = diffie_hellman(algorithms, ek_priv, ek_pub.clone(), other_keys.spk).unwrap();
    let mut combined_dh = vec![];
    combined_dh.extend(dh1);
    combined_dh.extend(dh2);
    combined_dh.extend(dh3);

    let opk_id = if other_keys.opks.is_empty() {
        None
    } else {
        let mut buffer = [0u8; 4];
        rng_fill(algorithms, &mut buffer);
        Some(u32::from_ne_bytes(buffer) % other_keys.opks.len() as u32)
    };
    let opk = if let Some(opk_id) = opk_id {
        other_keys.opks.get(opk_id as usize)
    } else {
        None
    };

    if let Some(opk) = opk {
        combined_dh.extend(opk.pk.clone());
    }

    let sk = kdf(algorithms, &combined_dh, 32).unwrap();
    let sk2 = kdf(algorithms, &sk, 32).unwrap();
    let sk2 = PrivateKey { sk: sk2 };

    let mut ad = vec![];
    ad.extend(ik_pub.pk.clone());
    ad.extend(other_keys.ik.pk);

    let (ciphertext, mac) = aead_wrap(algorithms, data, sk2, &ad).unwrap();

    let mut signed_data = vec![];
    signed_data.extend(ek_pub.pk.clone());
    if let Some(opk) = opk {
        signed_data.extend(opk.pk.clone());
    }
    signed_data.extend(ciphertext.clone());
    // TODO: Idk with which key to sign as it's not specified by documentation provided. So I
    // assume it's `ik_priv`.
    let signature = sign(algorithms, ik_priv, ik_pub, &signed_data).unwrap();

    Ok(X3DhData {
        ek_pub,
        opk_id,
        ciphertext,
        mac,
        signature,
    })
}

pub fn decode_x3dh(
    data: X3DhData,
    other_ik_pub: PublicKey,
    self_keys_public: X3DhReceiverKeysPublic,
    self_keys_private: X3DhReceiverKeysPrivate,
) -> Result<Box<[u8]>, X3DhError> {
    let algorithms= &self_keys_public.algorithms;

    let mut signed_data = vec![];
    signed_data.extend(data.ek_pub.pk.clone());
    let mut opk = None;
    if let Some(opk_id) = data.opk_id {
        let Some(opk_bytes) = self_keys_public.opks.get(opk_id as usize) else {
            return Err(X3DhError::InvalidOpkKeyId);
        };
        opk = Some(opk_bytes);
        signed_data.extend(opk_bytes.pk.clone());
    }
    signed_data.extend(data.ciphertext.clone());

    match verify(
        algorithms,
        other_ik_pub.clone(),
        &signed_data,
        &data.signature,
    ) {
        Some(true) => {}
        Some(false) => return Err(X3DhError::InvalidSignature),
        None => return Err(X3DhError::AlgorithmNotSupported),
    }

    let dh1 = diffie_hellman(
        algorithms,
        self_keys_private.spk.clone(),
        self_keys_public.spk.clone(),
        other_ik_pub.clone(),
    )
    .unwrap();
    let dh2 = diffie_hellman(
        algorithms,
        self_keys_private.ik,
        self_keys_public.ik.clone(),
        data.ek_pub.clone(),
    )
    .unwrap();
    let dh3 = diffie_hellman(
        algorithms,
        self_keys_private.spk,
        self_keys_public.spk,
        data.ek_pub,
    )
    .unwrap();
    let mut combined_dh = vec![];
    combined_dh.extend(dh1);
    combined_dh.extend(dh2);
    combined_dh.extend(dh3);

    if let Some(opk) = opk {
        combined_dh.extend(opk.pk.clone());
    }

    let sk = kdf(algorithms, &combined_dh, 32).unwrap();
    let sk2 = kdf(algorithms, &sk, 32).unwrap();
    let sk2 = PrivateKey { sk: sk2 };

    let mut ad = vec![];
    ad.extend(other_ik_pub.pk);
    ad.extend(self_keys_public.ik.pk);

    match aead_unwrap(algorithms, &data.ciphertext, &ad, &data.mac, sk2) {
        Some(Some(plaintext)) => Ok(plaintext),
        Some(None) => Err(X3DhError::DecryptionFailure),
        None => Err(X3DhError::AlgorithmNotSupported),
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::{x3dh::{decode_x3dh, encode_x3dh, generate_receiver_keys}, CryptoAlgorithms};

    #[test]
    fn test_x3dh() {
        let random_keys_a = generate_receiver_keys(&CryptoAlgorithms::prequantum_bee2rs()).unwrap();
        let random_keys_b = generate_receiver_keys(&CryptoAlgorithms::prequantum_bee2rs()).unwrap();
        let message = "Hello, World!".as_bytes();
        let encode_data = encode_x3dh(
            message,
            random_keys_a.0.ik,
            random_keys_a.1.ik.clone(),
            random_keys_b.1.clone(),
        )
        .unwrap();
        let decoded_data = decode_x3dh(
            encode_data,
            random_keys_a.1.ik,
            random_keys_b.1,
            random_keys_b.0,
        )
        .unwrap();
        assert_eq!(*message, *decoded_data);
    }
}
