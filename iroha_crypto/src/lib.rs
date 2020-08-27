//! This module contains structures and implementations related to the c{ digest_function: (), payload: ()}digest_function: (), payload: ()}ptographic parts of the
//! Iroha.

pub mod multihash;

use multihash::{DigestFunction as MultihashDigestFunction, Multihash};
use parity_scale_codec::{Decode, Encode};
use serde::{de::Error as SerdeError, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    fmt::{self, Debug, Display, Formatter},
    str::FromStr,
};
use ursa::{
    blake2::{
        digest::{Input, VariableOutput},
        VarBlake2b,
    },
    keys::{
        KeyGenOption as UrsaKeyGenOption, PrivateKey as UrsaPrivateKey, PublicKey as UrsaPublicKey,
    },
    signatures::{ed25519::Ed25519Sha512, secp256k1::EcdsaSecp256k1Sha256, SignatureScheme},
};

pub const HASH_LENGTH: usize = 32;
pub const ED_25519: &str = "ed25519";
pub const SECP_256_K1: &str = "secp256k1";

/// Represents hash of Iroha entities like `Block` or `Transaction.
pub type Hash = [u8; HASH_LENGTH];

#[derive(Clone)]
pub enum Algorithm {
    Ed25519,
    Secp256k1,
}

impl Default for Algorithm {
    fn default() -> Self {
        Algorithm::Ed25519
    }
}

impl FromStr for Algorithm {
    type Err = String;
    fn from_str(algorithm: &str) -> Result<Self, Self::Err> {
        match algorithm {
            ED_25519 => Ok(Algorithm::Ed25519),
            SECP_256_K1 => Ok(Algorithm::Secp256k1),
            _ => Err(format!("The {} algorithm is not supported.", algorithm)),
        }
    }
}

impl Display for Algorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Algorithm::Ed25519 => write!(f, "{}", ED_25519),
            Algorithm::Secp256k1 => write!(f, "{}", SECP_256_K1),
        }
    }
}

pub enum KeyGenOption {
    UseSeed(Vec<u8>),
    FromPrivateKey(PrivateKey),
}

impl TryFrom<KeyGenOption> for UrsaKeyGenOption {
    type Error = String;

    fn try_from(key_gen_option: KeyGenOption) -> Result<Self, Self::Error> {
        match key_gen_option {
            KeyGenOption::UseSeed(seed) => Ok(UrsaKeyGenOption::UseSeed(seed)),
            KeyGenOption::FromPrivateKey(key) => {
                if key.digest_function == ED_25519 || key.digest_function == SECP_256_K1 {
                    Ok(UrsaKeyGenOption::FromSecretKey(UrsaPrivateKey(key.payload)))
                } else {
                    Err(format!(
                        "Ursa does not support {} digest function.",
                        key.digest_function
                    ))
                }
            }
        }
    }
}

#[derive(Default)]
pub struct KeyGenConfiguration {
    pub key_gen_option: Option<KeyGenOption>,
    pub algorithm: Algorithm,
}

impl KeyGenConfiguration {
    pub fn use_seed(mut self, seed: Vec<u8>) -> KeyGenConfiguration {
        self.key_gen_option = Some(KeyGenOption::UseSeed(seed));
        self
    }

    pub fn use_private_key(mut self, private_key: PrivateKey) -> KeyGenConfiguration {
        self.key_gen_option = Some(KeyGenOption::FromPrivateKey(private_key));
        self
    }

    pub fn with_algorithm(mut self, algorithm: Algorithm) -> KeyGenConfiguration {
        self.algorithm = algorithm;
        self
    }
}

/// Pair of Public and Private keys.
#[derive(Clone, Debug, Deserialize, Default, Serialize)]
pub struct KeyPair {
    /// Public Key.
    pub public_key: PublicKey,
    /// Private Key.
    pub private_key: PrivateKey,
}

impl KeyPair {
    /// Generates a pair of Public and Private key with `Algorithm::default()` selected as generation algorithm.
    /// Returns `Err(String)` with error message if failed.
    pub fn generate() -> Result<Self, String> {
        Self::generate_with_configuration(KeyGenConfiguration::default())
    }

    /// Generates a pair of Public and Private key with the corresponding `KeyGenConfiguration`.
    /// Returns `Err(String)` with error message if failed.
    pub fn generate_with_configuration(configuration: KeyGenConfiguration) -> Result<Self, String> {
        let key_gen_option: Option<UrsaKeyGenOption> = configuration
            .key_gen_option
            .map(|key_gen_option| key_gen_option.try_into())
            .transpose()?;
        let (public_key, private_key) = match configuration.algorithm {
            Algorithm::Ed25519 => Ed25519Sha512.keypair(key_gen_option),
            Algorithm::Secp256k1 => EcdsaSecp256k1Sha256::new().keypair(key_gen_option),
        }
        .map_err(|e| format!("Failed to generate key pair: {}", e))?;
        Ok(KeyPair {
            public_key: PublicKey {
                digest_function: configuration.algorithm.to_string(),
                payload: public_key.as_ref().to_vec(),
            },
            private_key: PrivateKey {
                digest_function: configuration.algorithm.to_string(),
                payload: private_key.as_ref().to_vec(),
            },
        })
    }
}

/// Public Key used in signatures.
#[derive(Encode, Decode, Ord, PartialEq, Eq, PartialOrd, Clone, Hash, Default)]
pub struct PublicKey {
    pub digest_function: String,
    pub payload: Vec<u8>,
}

impl Debug for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PublicKey")
            .field("digest_function", &self.digest_function)
            .field("payload", &format!("{:X?}", self.payload))
            .finish()
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let multihash: &Multihash = &self
            .try_into()
            .expect("Failed to get multihash representation.");
        let bytes: Vec<u8> = multihash
            .try_into()
            .expect("Failed to convert multihash to bytes.");
        write!(f, "{}", hex::encode(bytes))
    }
}

impl TryFrom<&Multihash> for PublicKey {
    type Error = String;

    fn try_from(multihash: &Multihash) -> Result<Self, Self::Error> {
        match multihash.digest_function {
            MultihashDigestFunction::Ed25519Pub => Ok(ED_25519.to_string()),
            MultihashDigestFunction::Secp256k1Pub => Ok(SECP_256_K1.to_string()),
        }
        .map(|digest_function| PublicKey {
            digest_function,
            payload: multihash.payload.clone(),
        })
    }
}

impl TryFrom<&PublicKey> for Multihash {
    type Error = String;

    fn try_from(public_key: &PublicKey) -> Result<Self, Self::Error> {
        match public_key.digest_function.as_ref() {
            ED_25519 => Ok(MultihashDigestFunction::Ed25519Pub),
            SECP_256_K1 => Ok(MultihashDigestFunction::Secp256k1Pub),
            _ => Err("Digest function not implemented.".to_string()),
        }
        .map(|digest_function| Multihash {
            digest_function,
            payload: public_key.payload.clone(),
        })
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = hex::decode(String::deserialize(deserializer)?).map_err(SerdeError::custom)?;
        let multihash: &Multihash = &bytes.try_into().map_err(SerdeError::custom)?;
        multihash.try_into().map_err(SerdeError::custom)
    }
}

/// Private Key used in signatures.
#[derive(Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct PrivateKey {
    pub digest_function: String,
    #[serde(deserialize_with = "from_hex", serialize_with = "to_hex")]
    pub payload: Vec<u8>,
}

fn from_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    hex::decode(String::deserialize(deserializer)?).map_err(SerdeError::custom)
}

fn to_hex<S>(payload: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(payload))
}

impl Debug for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateKey")
            .field("digest_function", &self.digest_function)
            .field("payload", &format!("{:X?}", self.payload))
            .finish()
    }
}

impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.payload))
    }
}

/// Calculates hash of the given bytes.
pub fn hash(bytes: Vec<u8>) -> Hash {
    let vec_hash = VarBlake2b::new(32)
        .expect("Failed to initialize variable size hash")
        .chain(bytes)
        .vec_result();
    let mut hash = [0; HASH_LENGTH];
    hash.copy_from_slice(&vec_hash);
    hash
}

/// Represents signature of the data (`Block` or `Transaction` for example).
#[derive(Clone, Encode, Decode)]
pub struct Signature {
    /// Ed25519 (Edwards-curve Digital Signature Algorithm scheme using SHA-512 and Curve25519)
    /// public-key of an approved authority.
    pub public_key: PublicKey,
    /// Ed25519 signature is placed here.
    signature: Vec<u8>,
}

impl Signature {
    /// Creates new `Signature` by signing payload via `private_key`.
    pub fn new(key_pair: KeyPair, payload: &[u8]) -> Result<Signature, String> {
        let private_key = UrsaPrivateKey(key_pair.private_key.payload.to_vec());
        let algorithm: Algorithm = key_pair.public_key.digest_function.parse()?;
        let signature = match algorithm {
            Algorithm::Ed25519 => Ed25519Sha512::new().sign(payload, &private_key),
            Algorithm::Secp256k1 => EcdsaSecp256k1Sha256::new().sign(payload, &private_key),
        }
        .map_err(|e| format!("Failed to sign payload: {}", e))?;
        Ok(Signature {
            public_key: key_pair.public_key,
            signature,
        })
    }

    /// Verify `message` using signed data and `public_key`.
    pub fn verify(&self, message: &[u8]) -> Result<(), String> {
        let public_key = UrsaPublicKey(self.public_key.payload.to_vec());
        let algorithm: Algorithm = self.public_key.digest_function.parse()?;
        match algorithm {
            Algorithm::Ed25519 => {
                Ed25519Sha512::new().verify(message, &self.signature, &public_key)
            }
            Algorithm::Secp256k1 => {
                EcdsaSecp256k1Sha256::new().verify(message, &self.signature, &public_key)
            }
        }
        .map_err(|e| e.to_string())
        .and_then(|verified| {
            if verified {
                Ok(())
            } else {
                Err("Signature did not pass verification.".to_string())
            }
        })
    }
}

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
            .field("signature", &format!("{:X?}", self.signature.to_vec()))
            .finish()
    }
}

/// Container for multiple signatures.
#[derive(Debug, Clone, Encode, Decode, Default)]
pub struct Signatures {
    signatures: BTreeMap<PublicKey, Signature>,
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
        let _option = self
            .signatures
            .insert(signature.public_key.clone(), signature);
    }

    /// Whether signatures contain a signature with the specified `public_key`
    pub fn contains(&self, public_key: &PublicKey) -> bool {
        self.signatures.contains_key(public_key)
    }

    /// Removes all signatures
    pub fn clear(&mut self) {
        self.signatures.clear()
    }

    /// Returns signatures that have passed verification.
    pub fn verified(&self, payload: &[u8]) -> Vec<Signature> {
        self.signatures
            .iter()
            .filter(|&(_, signature)| signature.verify(payload).is_ok())
            .map(|(_, signature)| signature)
            .cloned()
            .collect()
    }

    /// Returns all signatures.
    pub fn values(&self) -> Vec<Signature> {
        self.signatures
            .iter()
            .map(|(_, signature)| signature)
            .cloned()
            .collect()
    }
}

/// The prelude re-exports most commonly used traits, structs and macros from this crate.
pub mod prelude {
    pub use super::{KeyPair, PrivateKey, PublicKey, Signature, Signatures};
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use serde::Deserialize;
    use ursa::blake2::{
        digest::{Input, VariableOutput},
        VarBlake2b,
    };

    #[test]
    fn create_signature_ed25519() {
        let key_pair = KeyPair::generate_with_configuration(
            KeyGenConfiguration::default().with_algorithm(Algorithm::Ed25519),
        )
        .expect("Failed to generate key pair.");
        let message = b"Test message to sign.";
        let signature =
            Signature::new(key_pair.clone(), message).expect("Failed to create signature.");
        assert_eq!(signature.public_key, key_pair.public_key);
        assert!(signature.verify(message).is_ok())
    }

    #[test]
    fn create_signature_secp256k1() {
        let key_pair = KeyPair::generate_with_configuration(
            KeyGenConfiguration::default().with_algorithm(Algorithm::Secp256k1),
        )
        .expect("Failed to generate key pair.");
        let message = b"Test message to sign.";
        let signature =
            Signature::new(key_pair.clone(), message).expect("Failed to create signature.");
        assert_eq!(signature.public_key, key_pair.public_key);
        assert!(signature.verify(message).is_ok())
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

    #[test]
    fn display_public_key() {
        assert_eq!(
            format!(
                "{}",
                PublicKey {
                    digest_function: ED_25519.to_string(),
                    payload: hex::decode(
                        "1509a611ad6d97b01d871e58ed00c8fd7c3917b6ca61a8c2833a19e000aac2e4"
                    )
                    .expect("Failed to decode public key.")
                }
            ),
            "ed201509a611ad6d97b01d871e58ed00c8fd7c3917b6ca61a8c2833a19e000aac2e4"
        );
        assert_eq!(
            format!(
                "{}",
                PublicKey {
                    digest_function: SECP_256_K1.to_string(),
                    payload: hex::decode(
                        "0312273e8810581e58948d3fb8f9e8ad53aaa21492ebb8703915bbb565a21b7fcc"
                    )
                    .expect("Failed to decode public key.")
                }
            ),
            "e7210312273e8810581e58948d3fb8f9e8ad53aaa21492ebb8703915bbb565a21b7fcc"
        )
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct TestJson {
        public_key: PublicKey,
        private_key: PrivateKey,
    }

    #[test]
    fn tst() {
        use hex::encode;
        let pk = vec![52, 45, 84, 67, 137, 84, 47, 252, 35, 59, 237, 44, 144, 70, 71, 206, 243, 67, 8, 115, 247, 189, 204, 26, 181, 226, 232, 81, 123, 12, 81, 120];
        let sk = vec![18, 182, 246, 209, 68, 27, 219, 111, 25, 143, 14, 178, 64, 212, 107, 38, 113, 40, 79, 226, 81, 217, 198, 102, 12, 68, 238, 115, 162, 63, 242, 255, 52, 45, 84, 67, 137, 84, 47, 252, 35, 59, 237, 44, 144, 70, 71, 206, 243, 67, 8, 115, 247, 189, 204, 26, 181, 226, 232, 81, 123, 12, 81, 120];

        println!("{}", encode(pk));
        println!("{}", encode(sk));
        panic!();
    }

    #[test]
    fn deserialize_keys() {
        assert_eq!(
            serde_json::from_str::<'_, TestJson>("{
                \"public_key\": \"ed201509a611ad6d97b01d871e58ed00c8fd7c3917b6ca61a8c2833a19e000aac2e4\",
                \"private_key\": {
                    \"digest_function\": \"ed25519\",
                    \"payload\": \"3a7991af1abb77f3fd27cc148404a6ae4439d095a63591b77c788d53f708a02a1509a611ad6d97b01d871e58ed00c8fd7c3917b6ca61a8c2833a19e000aac2e4\"
                }
            }").expect("Failed to deserialize."),
            TestJson {
                public_key: PublicKey {
                    digest_function: ED_25519.to_string(),
                    payload: hex::decode(
                        "1509a611ad6d97b01d871e58ed00c8fd7c3917b6ca61a8c2833a19e000aac2e4"
                    )
                    .expect("Failed to decode public key.")
                },
                private_key: PrivateKey {
                    digest_function: ED_25519.to_string(),
                    payload: hex::decode("3a7991af1abb77f3fd27cc148404a6ae4439d095a63591b77c788d53f708a02a1509a611ad6d97b01d871e58ed00c8fd7c3917b6ca61a8c2833a19e000aac2e4")
                    .expect("Failed to decode private key"),
                }
            }
        );
        assert_eq!(
            serde_json::from_str::<'_, TestJson>("{
                \"public_key\": \"e7210312273e8810581e58948d3fb8f9e8ad53aaa21492ebb8703915bbb565a21b7fcc\",
                \"private_key\": {
                    \"digest_function\": \"secp256k1\",
                    \"payload\": \"4df4fca10762d4b529fe40a2188a60ca4469d2c50a825b5f33adc2cb78c69445\"
                }
            }").expect("Failed to deserialize."),
            TestJson {
                public_key: PublicKey {
                    digest_function: SECP_256_K1.to_string(),
                    payload: hex::decode(
                        "0312273e8810581e58948d3fb8f9e8ad53aaa21492ebb8703915bbb565a21b7fcc"
                    )
                    .expect("Failed to decode public key.")
                },
                private_key: PrivateKey {
                    digest_function: SECP_256_K1.to_string(),
                    payload: hex::decode("4df4fca10762d4b529fe40a2188a60ca4469d2c50a825b5f33adc2cb78c69445")
                    .expect("Failed to decode private key"),
                }
            }
        )
    }
}
