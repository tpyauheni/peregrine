use super::*;

#[derive(Clone)]
pub struct X3DhReceiverKeysPublic {
    pub alg_name: String,
    pub ik: Box<[u8]>,
    pub spk: Box<[u8]>,
    pub spk_signature: Box<[u8]>,
    pub opks: Vec<Box<[u8]>>,
}

#[derive(Clone)]
pub struct X3DhReceiverKeysPrivate {
    pub ik: Box<[u8]>,
    pub spk: Box<[u8]>,
    pub opks: Vec<Box<[u8]>>,
}

pub fn generate_receiver_keys(alg_name: &str) -> Option<(X3DhReceiverKeysPrivate, X3DhReceiverKeysPublic)> {
    let (ik_priv, ik_pub) = generate_keypair(alg_name)?;
    let (spk_priv, spk_pub) = generate_keypair(alg_name)?;
    let spk_signature = sign(alg_name, &ik_priv, &ik_pub, &spk_pub)?;

    let mut opks_priv = Vec::new();
    let mut opks_pub = Vec::new();
    for _ in 0..10 {
        let (opk_priv, opk_pub) = generate_keypair(alg_name)?;
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
            alg_name: alg_name.to_owned(),
            ik: ik_pub,
            spk: spk_pub,
            opks: opks_pub,
            spk_signature,
        },
    ))
}

#[derive(Clone)]
pub struct X3DhData {
    ek_pub: Box<[u8]>,
    opk_id: Option<u32>,
    ciphertext: Box<[u8]>,
    mac: Box<[u8]>,
    signature: Box<[u8]>,
}

#[derive(Clone)]
pub enum X3DhEncodeResult {
    AlgorithmNotSupported,
    InvalidSignature,
    Data(X3DhData),
}

pub fn encode_x3dh(
    data: &[u8],
    ik_priv: &[u8],
    ik_pub: &[u8],
    other_keys: X3DhReceiverKeysPublic,
) -> X3DhEncodeResult {
    let alg_name = &other_keys.alg_name;

    match verify(alg_name, &other_keys.ik, &other_keys.spk, &other_keys.spk_signature) {
        Some(true) => {},
        Some(false) => return X3DhEncodeResult::InvalidSignature,
        None => return X3DhEncodeResult::AlgorithmNotSupported,
    }

    let Some((ek_priv, ek_pub)) = generate_keypair(alg_name) else {
        return X3DhEncodeResult::AlgorithmNotSupported
    };

    let dh1 = diffie_hellman(alg_name, ik_pub, ik_priv, &other_keys.spk).unwrap();
    let dh2 = diffie_hellman(alg_name, &ek_pub, &ek_priv, &other_keys.ik).unwrap();
    let dh3 = diffie_hellman(alg_name, &ek_pub, &ek_priv, &other_keys.spk).unwrap();
    let mut combined_dh = vec![];
    combined_dh.extend(dh1);
    combined_dh.extend(dh2);
    combined_dh.extend(dh3);

    let opk_id = if other_keys.opks.is_empty() {
        None
    } else {
        let mut buffer = [0u8; 4];
        default_rng().next_buffer(&mut buffer);
        Some(u32::from_ne_bytes(buffer) % other_keys.opks.len() as u32)
    };
    let opk = if let Some(opk_id) = opk_id {
        other_keys.opks.get(opk_id as usize)
    } else {
        None
    };

    if let Some(opk) = opk {
        combined_dh.extend(opk);
    }

    let sk = kdf(alg_name, &combined_dh, 32).unwrap();
    let sk2 = kdf(alg_name, &sk, 32).unwrap();

    let mut ad = vec![];
    ad.extend(ik_pub);
    ad.extend(other_keys.ik);

    let (ciphertext, mac) = aead_wrap(alg_name, data, &sk2, &ad).unwrap();

    let mut signed_data = vec![];
    signed_data.extend(ek_pub.clone());
    if let Some(opk) = opk {
        signed_data.extend(opk);
    }
    signed_data.extend(ciphertext.clone());
    // TODO: Idk with which key to sign as it's not specified by documentation provided. So I
    // assume it's `ik_priv`.
    let signature = sign(alg_name, ik_priv, ik_pub, &signed_data).unwrap();

    X3DhEncodeResult::Data(X3DhData {
        ek_pub,
        opk_id,
        ciphertext,
        mac,
        signature,
    })
}

#[derive(Clone)]
pub enum X3DhDecodeResult {
    DecryptionFailure,
    InvalidSignature,
    InvalidOpkKeyId,
    UnsupportedAlgorithm,
    Data(Box<[u8]>),
}

pub fn decode_x3dh(
    data: X3DhData,
    other_ik_pub: &[u8],
    self_keys_public: X3DhReceiverKeysPublic,
    self_keys_private: X3DhReceiverKeysPrivate,
) -> X3DhDecodeResult {
    let alg_name = &self_keys_public.alg_name;

    let mut signed_data = vec![];
    signed_data.extend(data.ek_pub.clone());
    let mut opk = None;
    if let Some(opk_id) = data.opk_id {
        let Some(opk_bytes) = self_keys_public.opks.get(opk_id as usize) else {
            return X3DhDecodeResult::InvalidOpkKeyId;
        };
        opk = Some(opk_bytes);
        signed_data.extend(opk_bytes);
    }
    signed_data.extend(data.ciphertext.clone());

    match verify(alg_name, other_ik_pub, &signed_data, &data.signature) {
        Some(true) => {},
        Some(false) => return X3DhDecodeResult::InvalidSignature,
        None => return X3DhDecodeResult::UnsupportedAlgorithm,
    }

    let dh1 = diffie_hellman(alg_name, &self_keys_public.spk, &self_keys_private.spk, other_ik_pub).unwrap();
    let dh2 = diffie_hellman(alg_name, &self_keys_public.ik, &self_keys_private.ik, &data.ek_pub).unwrap();
    let dh3 = diffie_hellman(alg_name, &self_keys_public.spk, &self_keys_private.spk, &data.ek_pub).unwrap();
    let mut combined_dh = vec![];
    combined_dh.extend(dh1);
    combined_dh.extend(dh2);
    combined_dh.extend(dh3);

    if let Some(opk) = opk {
        combined_dh.extend(opk);
    }

    let sk = kdf(alg_name, &combined_dh, 32).unwrap();
    let sk2 = kdf(alg_name, &sk, 32).unwrap();

    let mut ad = vec![];
    ad.extend(other_ik_pub);
    ad.extend(self_keys_public.ik);

    match aead_unwrap(alg_name, &data.ciphertext, &ad, &data.mac, &sk2) {
        Some(Some(plaintext)) => X3DhDecodeResult::Data(plaintext),
        Some(None) => X3DhDecodeResult::DecryptionFailure,
        None => X3DhDecodeResult::UnsupportedAlgorithm,
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::x3dh::{decode_x3dh, encode_x3dh, generate_receiver_keys, X3DhDecodeResult};

    #[test]
    fn test_x3dh() {
        let random_keys_a = generate_receiver_keys("bycrypto").unwrap();
        let random_keys_b = generate_receiver_keys("bycrypto").unwrap();
        let message = "Hello, World!".as_bytes();
        let encode_data = match encode_x3dh(message, &random_keys_a.0.ik, &random_keys_a.1.ik, random_keys_b.1.clone()) {
            super::X3DhEncodeResult::Data(data) => data,
            _ => panic!(),
        };
        let decoded_data = match decode_x3dh(encode_data, &random_keys_a.1.ik, random_keys_b.1, random_keys_b.0) {
            X3DhDecodeResult::Data(data) => data,
            _ => panic!(),
        };
        assert_eq!(*message, *decoded_data);
    }
}
