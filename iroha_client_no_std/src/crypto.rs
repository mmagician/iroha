//! This module contains structures and implementations related to the cryptographic parts of the
//! Iroha.
use alloc::collections::BTreeMap;
use alloc::{string::String, vec::Vec};
use core::ops::Deref;
use core::{
    convert::TryFrom,
    fmt::{self, Debug, Formatter},
};
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
// use ursa::{
//     blake2::{
//         digest::{Input, VariableOutput},
//         VarBlake2b,
//     },
//     keys::{PrivateKey as UrsaPrivateKey, PublicKey as UrsaPublicKey},
//     signatures::{ed25519::Ed25519Sha512, SignatureScheme, Signer},
// };
// use sp_core::sr25519::Public;
// use sp_core::hash::H256;

pub const HASH_LENGTH: usize = 32;
pub const ED_25519: &str = "ed25519";
pub const SECP_256_K1: &str = "secp256k1";

/// Represents hash of Iroha entities like `Block` or `Transaction.
pub type Hash = [u8; HASH_LENGTH];

/// Pair of Public and Private keys.
#[cfg_attr(feature = "std", derive(Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct KeyPair {
    /// Public Key.
    pub public_key: PublicKey,
    /// Private Key.
    pub private_key: PrivateKey,
}

/// Public Key used in signatures.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(
    Encode, Decode, Ord, PartialEq, Eq, PartialOrd, Debug, Clone, Hash, Default,
)]
pub struct PublicKey {
    pub digest_function: String,
    pub payload: Vec<u8>,
}

impl Deref for PublicKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

/// Private Key used in signatures.
#[cfg_attr(feature = "std", derive(Deserialize))]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PrivateKey {
    pub digest_function: String,
    #[serde(deserialize_with = "from_hex", serialize_with = "to_hex")]
    pub payload: Vec<u8>,
}

/// Type of digest function.
/// The corresponding byte codes are taken from [official multihash table](https://github.com/multiformats/multicodec/blob/master/table.csv)
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DigestFunction {
    Ed25519Pub = 0xed,
    Secp256k1Pub = 0xe7,
}

#[derive(Eq, PartialEq, Debug)]
pub struct Multihash {
    pub digest_function: DigestFunction,
    pub payload: Vec<u8>,
}

impl TryFrom<&Multihash> for PublicKey {
    type Error = String;

    fn try_from(multihash: &Multihash) -> Result<Self, Self::Error> {
        match multihash.digest_function {
            DigestFunction::Ed25519Pub => Ok(ED_25519.to_string()),
            DigestFunction::Secp256k1Pub => Ok(SECP_256_K1.to_string()),
        }
            .map(|digest_function| PublicKey {
                digest_function,
                payload: multihash.payload.clone(),
            })
    }
}

impl TryFrom<Vec<u8>> for PrivateKey {
    type Error = String;

    fn try_from(vector: Vec<u8>) -> Result<Self, Self::Error> {
        if vector.len() != 64 {
            Err(format!(
                "Failed to build PublicKey from vector: {:?}, expected length 32, found {}.",
                &vector,
                vector.len()
            ))
        } else {
            Ok(PrivateKey { digest_function: ED_25519.to_owned(), payload: vector })
        }
    }
}

pub type Ed25519Signature = [u8; 64];

// impl KeyPair {
//     /// Generates a pair of Public and Private key.
//     /// Returns `Err(String)` with error message if failed.
//     pub fn generate() -> Result<Self, String> {
//         let (public_key, ursa_private_key) = Ed25519Sha512
//             .keypair(Option::None)
//             .map_err(|e| format!("Failed to generate Ed25519Sha512 key pair: {}", e))?;
//         let public_key: [u8; 32] = public_key[..]
//             .try_into()
//             .map_err(|e| format!("Public key should be [u8;32]: {}", e))?;
//         let mut private_key = [0; 64];
//         private_key.copy_from_slice(ursa_private_key.as_ref());
//         Ok(KeyPair {
//             public_key: PublicKey { inner: public_key },
//             private_key: PrivateKey::try_from(private_key.to_vec()).map_err(|e| {
//                 format!(
//                     "Failed to convert Ursa Private key to Iroha Private Key: {}",
//                     e
//                 )
//             })?,
//         })
//     }
// }

// /// Calculates hash of the given bytes.
// pub fn hash(bytes: Vec<u8>) -> Hash {
//     let vec_hash = VarBlake2b::new(32)
//         .expect("Failed to initialize variable size hash")
//         .chain(bytes)
//         .vec_result();
//     let mut hash = [0; 32];
//     hash.copy_from_slice(&vec_hash);
//     hash
// }

/// Represents signature of the data (`Block` or `Transaction` for example).
#[derive(Clone, Encode, Decode)]
pub struct Signature {
    /// Ed25519 (Edwards-curve Digital Signature Algorithm scheme using SHA-512 and Curve25519)
    /// public-key of an approved authority.
    pub public_key: PublicKey,
    /// Ed25519 signature is placed here.
    pub signature: Vec<u8>,
}

// impl Signature {
//     /// Creates new `Signature` by signing payload via `private_key`.
//     pub fn new(key_pair: KeyPair, payload: &[u8]) -> Result<Signature, String> {
//         let private_key = UrsaPrivateKey(key_pair.private_key.inner.to_vec());
//         let transaction_signature = Signer::new(&Ed25519Sha512, &private_key)
//             .sign(payload)
//             .map_err(|e| format!("Failed to sign payload: {}", e))?;
//         let mut signature = [0; 64];
//         signature.copy_from_slice(&transaction_signature);
//         Ok(Signature {
//             public_key: key_pair.public_key,
//             signature,
//         })
//     }
//
//     /// Verify `message` using signed data and `public_key`.
//     pub fn verify(&self, message: &[u8]) -> Result<(), String> {
//         Ed25519Sha512::new()
//             .verify(
//                 message,
//                 &self.signature,
//                 &UrsaPublicKey(self.public_key.inner.to_vec()),
//             )
//             .map_err(|e| e.to_string())
//             .map(|_| ())
//     }
// }

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.public_key == other.public_key && self.signature.to_vec() == other.signature.to_vec()
    }
}

impl Eq for Signature {}

impl Debug for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signature")
            .field("public_key", &self.public_key)
            .field("signature", &self.signature.to_vec())
            .finish()
    }
}

/// Container for multiple signatures.
#[derive(Debug, Clone, Encode, Decode, Default)]
pub struct Signatures {
    pub signatures: BTreeMap<PublicKey, Signature>,
}

impl Signatures {
    /// Adds multiple signatures and replaces the duplicates.
    pub fn append(&mut self, signatures: &[Signature]) {
        for signature in signatures.iter().cloned() {
            self.add(signature.clone())
        }
    }

    /// Adds a signature. If the signature with this key was present, replaces it.
    pub fn add(&mut self, signature: Signature) {
        let _option = self.signatures.insert(signature.clone().public_key, signature);
    }

    /// Whether signatures contain a signature with the specified `public_key`
    pub fn contains(&self, public_key: &PublicKey) -> bool {
        self.signatures.contains_key(public_key)
    }

    /// Removes all signatures
    pub fn clear(&mut self) {
        self.signatures.clear()
    }

    // /// Returns signatures that have passed verification.
    // pub fn verified(&self, payload: &[u8]) -> Vec<Signature> {
    //     self.signatures
    //         .iter()
    //         .filter(|&(_, signature)| signature.verify(payload).is_ok())
    //         .map(|(_, signature)| signature)
    //         .cloned()
    //         .collect()
    // }

    /// Returns all signatures.
    pub fn values(&self) -> Vec<Signature> {
        self.signatures
            .iter()
            .map(|(_, signature)| signature)
            .cloned()
            .collect()
    }
}

fn from_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: serde::Deserializer<'de>,
{
    hex::decode(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
}

fn to_hex<S>(payload: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    use hex_literal::hex;
    use ursa::blake2::{
        digest::{Input, VariableOutput},
        VarBlake2b,
    };

    #[test]
    fn create_signature() {
        let key_pair = KeyPair::generate().expect("Failed to generate key pair.");
        let result = Signature::new(key_pair.clone(), b"Test message to sign.")
            .expect("Failed to create signature.");
        assert_eq!(result.public_key, key_pair.public_key);
    }

    #[test]
    fn blake2_32b() {
        let mut hasher = VarBlake2b::new(32).unwrap();
        hasher.input(hex!("6920616d2064617461"));
        hasher.variable_result(|res| {
            assert_eq!(
                res[..],
                hex!("ba67336efd6a3df3a70eeb757860763036785c182ff4cf587541a0068d09f5b2")[..]
            );
        })
    }
}
