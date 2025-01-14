use crate::{Error, Result};
use parity_scale_codec::Encode;
use serde::{Deserialize, Serialize};
use sp_core::{
    crypto::{Derive, DeriveJunction as SrDeriveJunction},
    sr25519::{Pair, Public, Signature as SrSignature},
    Pair as _,
};

extern crate alloc;
use alloc::vec::Vec;

pub const PUBLIC_KEY_LENGTH: usize = 32;
pub const SIGNATURE_LENGTH: usize = 64;

/// An Schnorrkel/Ristretto x25519 (“sr25519”) key pair.
pub struct KeyPair(Pair);

/// An Schnorrkel/Ristretto x25519 (“sr25519”) signature.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Signature(SrSignature);

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<[u8; SIGNATURE_LENGTH]> for Signature {
    fn as_ref(&self) -> &[u8; SIGNATURE_LENGTH] {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PublicKey(Public);

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<[u8; PUBLIC_KEY_LENGTH]> for PublicKey {
    fn as_ref(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        self.0.as_ref()
    }
}

impl Signature {
    /// A new instance from the given slice that should be 64 bytes long.
    ///
    /// NOTE: No checking goes on to ensure this is a real signature.
    /// Only use it if you are certain that the array actually is a signature.

    pub fn from_slice(data: &[u8]) -> Result<Self> {
        Ok(Self(SrSignature::from_slice(data).ok_or(
            Error::InvalidArgumentError {
                alg: "bytes",
                expected: "a valid secret key byte array",
            },
        )?))
    }

    /// A new instance from the given 64-byte data.
    ///
    /// NOTE: No checking goes on to ensure this is a real signature.
    /// Only use it if you are certain that the array actually is a signature, or if you immediately verify the
    /// signature. All functions that verify signatures will fail if the Signature is not actually a valid
    /// signature.
    pub fn from_raw(data: [u8; SIGNATURE_LENGTH]) -> Self {
        Self(SrSignature::from_raw(data))
    }

    /// Gets the wrapped sp_core's [`SrSignature`] reference.
    pub fn inner(&self) -> &SrSignature {
        &self.0
    }
}

/// A since derivation junction description.
/// It is the single parameter used when creating a new secret key from an existing secret key.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DeriveJunction {
    /// Soft (vanilla) derivation. Public keys have a correspondent derivation.
    Soft([u8; 32]),
    /// Hard (“hardened”) derivation. Public keys do not have a correspondent derivation.
    Hard([u8; 32]),
}

impl From<DeriveJunction> for SrDeriveJunction {
    fn from(j: DeriveJunction) -> SrDeriveJunction {
        match j {
            DeriveJunction::Soft(p) => SrDeriveJunction::Soft(p),
            DeriveJunction::Hard(p) => SrDeriveJunction::Hard(p),
        }
    }
}

impl From<SrDeriveJunction> for DeriveJunction {
    fn from(j: SrDeriveJunction) -> DeriveJunction {
        match j {
            SrDeriveJunction::Soft(p) => DeriveJunction::Soft(p),
            SrDeriveJunction::Hard(p) => DeriveJunction::Hard(p),
        }
    }
}

impl DeriveJunction {
    /// Create a new soft (vanilla) DeriveJunction from a given, encodable, value.
    ///
    /// If you need a hard junction, use `hard()`.
    pub fn soft<T: Encode>(index: T) -> Self {
        SrDeriveJunction::soft(index).into()
    }

    pub fn hard<T: Encode>(index: T) -> Self {
        SrDeriveJunction::hard(index).into()
    }
}

impl PublicKey {
    /// A new instance from the given 64-byte data.
    ///
    /// NOTE: No checking goes on to ensure this is a real public key.
    /// Only use it if you are certain that the array actually is a pubkey.
    pub fn from_raw(data: [u8; PUBLIC_KEY_LENGTH]) -> Self {
        Self(Public::from_raw(data))
    }

    /// Derive a child key from a series of given junctions.
    ///
    /// None if there are any hard junctions in there.
    pub fn derive<I: Iterator<Item = DeriveJunction>>(&self, path: I) -> Option<Self> {
        self.0.derive(path.map(Into::into)).map(Self)
    }

    /// Verify a signature on a message. Returns true if the signature is good.
    pub fn verify<M: AsRef<[u8]>>(&self, sig: &Signature, message: M) -> bool {
        Pair::verify(&sig.0, message, &self.0)
    }

    /// Gets the wrapped sp_core's [`Public`] reference.
    pub fn inner(&self) -> &Public {
        &self.0
    }
}

impl KeyPair {
    #[cfg(feature = "random")]
    #[cfg_attr(docsrs, doc(cfg(feature = "random")))]
    pub fn generate() -> crate::Result<Self> {
        let mut bs = [0u8; 32];
        crate::utils::rand::fill(&mut bs)?;
        Ok(Self::from_seed(&bs))
    }

    #[cfg(feature = "rand")]
    pub fn generate_with<R: rand::CryptoRng + rand::RngCore>(rng: &mut R) -> Self {
        let mut bs = [0_u8; 32];
        rng.fill_bytes(&mut bs);
        Self::from_seed(&bs)
    }

    /// Returns the KeyPair from the English BIP39 mnemonic or seed, or an error if it’s invalid.
    pub fn from_string(s: &str, password: Option<&str>) -> Result<Self> {
        let pair = Pair::from_string(s, password).map_err(|_| Error::InvalidArgumentError {
            alg: "KeyPair#from_string",
            expected: "a valid seed or english BIP39 mnemonic",
        })?;
        Ok(Self(pair))
    }

    /// Get the public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.public())
    }

    /// Signs a message.
    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.0.sign(message))
    }

    /// Derive a child key from a series of given junctions.
    pub fn derive<I: Iterator<Item = DeriveJunction>>(&self, path: I, seed: Option<[u8; 32]>) -> Result<Self> {
        let (pair, _) = self.0.derive(path.map(Into::into), seed).unwrap();
        Ok(Self(pair))
    }

    /// Gets the seed as raw vec.
    pub fn seed(&self) -> Vec<u8> {
        self.0.to_raw_vec()
    }

    pub fn from_seed(seed: &[u8]) -> Self {
        Self(Pair::from_seed_slice(seed).expect("invalid seed length"))
    }
}

impl From<PublicKey> for [u8; 32] {
    fn from(x: PublicKey) -> [u8; 32] {
        <[u8; 32]>::from(x.0)
    }
}
