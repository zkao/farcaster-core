//! Cryptographic type definitions and primitives supported in Farcaster

use std::error;
use std::fmt::Debug;

use strict_encoding::{StrictDecode, StrictEncode};
use thiserror::Error;

use crate::consensus::{self};
use crate::role::{Acc, Accordant, Arbitrating, Blockchain};
use crate::swap::Swap;

/// List of cryptographic errors that can be encountered when processing cryptographic operation
/// such as signatures, proofs, key derivation, or commitments.
#[derive(Error, Debug)]
pub enum Error {
    /// The signature does not pass the validation tests.
    #[error("The signature does not pass the validation")]
    InvalidSignature,
    /// The adaptor signature does not pass the validation tests.
    #[error("The adaptor signature does not pass the validation")]
    InvalidAdaptorSignature,
    /// The proof does not pass the validation tests.
    #[error("The proof does not pass the validation")]
    InvalidProof,
    /// The commitment does not match the given value.
    #[error("The commitment does not match the given value")]
    InvalidCommitment,
    /// Any cryptographic error not part of this list.
    #[error("Cryptographic error: {0}")]
    Other(Box<dyn error::Error + Send + Sync>),
}

impl Error {
    /// Creates a new cryptographic error of type other with an arbitrary payload.
    pub fn new<E>(error: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::Other(error.into())
    }

    /// Consumes the `Error`, returning its inner error (if any).
    ///
    /// If this [`enum@Error`] was constructed via [`new`] then this function will return [`Some`],
    /// otherwise it will return [`None`].
    ///
    /// [`new`]: Error::new
    ///
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        match self {
            Self::Other(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, StrictDecode, StrictEncode)]
#[strict_encoding_crate(strict_encoding)]
pub enum KeyType<Ctx>
where
    Ctx: Swap,
{
    PublicArbitrating(<Ctx::Ar as Keys>::PublicKey),
    PublicAccordant(<Ctx::Ac as Keys>::PublicKey),
    SharedPrivate(<Ctx::Ac as SharedPrivateKeys<Acc>>::SharedPrivateKey),
}

impl<Ctx> KeyType<Ctx>
where
    Ctx: Swap,
{
    pub fn try_into_arbitrating_pubkey(
        &self,
    ) -> Result<<Ctx::Ar as Keys>::PublicKey, consensus::Error> {
        match self {
            KeyType::PublicArbitrating(key) => Ok(key.clone()),
            _ => Err(consensus::Error::TypeMismatch),
        }
    }

    pub fn try_into_accordant_pubkey(
        &self,
    ) -> Result<<Ctx::Ac as Keys>::PublicKey, consensus::Error> {
        match self {
            KeyType::PublicAccordant(key) => Ok(key.clone()),
            _ => Err(consensus::Error::TypeMismatch),
        }
    }

    pub fn try_into_shared_private(
        &self,
    ) -> Result<<Ctx::Ac as SharedPrivateKeys<Acc>>::SharedPrivateKey, consensus::Error> {
        match self {
            KeyType::SharedPrivate(key) => Ok(key.clone()),
            _ => Err(consensus::Error::TypeMismatch),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            KeyType::PublicArbitrating(key) => <Ctx::Ar as Keys>::as_bytes(&key),
            KeyType::PublicAccordant(key) => <Ctx::Ac as Keys>::as_bytes(&key),
            KeyType::SharedPrivate(key) => <Ctx::Ac as SharedPrivateKeys<Acc>>::as_bytes(&key),
        }
    }
}

/// Type of signatures
#[derive(Clone, Debug, StrictDecode, StrictEncode)]
#[strict_encoding_crate(strict_encoding)]
pub enum SignatureType<S>
where
    S: Signatures,
{
    Adaptor(S::AdaptorSignature),
    Adapted(S::Signature),
    Regular(S::Signature),
}

impl<S> SignatureType<S>
where
    S: Signatures,
{
    pub fn try_into_adaptor(&self) -> Result<S::AdaptorSignature, consensus::Error> {
        match self {
            SignatureType::Adaptor(sig) => Ok(sig.clone()),
            _ => Err(consensus::Error::TypeMismatch),
        }
    }

    pub fn try_into_adapted(&self) -> Result<S::Signature, consensus::Error> {
        match self {
            SignatureType::Adapted(sig) => Ok(sig.clone()),
            _ => Err(consensus::Error::TypeMismatch),
        }
    }

    pub fn try_into_regular(&self) -> Result<S::Signature, consensus::Error> {
        match self {
            SignatureType::Regular(sig) => Ok(sig.clone()),
            _ => Err(consensus::Error::TypeMismatch),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ArbitratingKey {
    Fund,
    Buy,
    Cancel,
    Refund,
    Punish,
}

#[derive(Debug, Clone, Copy)]
pub enum AccordantKey {
    Spend,
}

#[derive(Debug, Clone, Copy)]
pub enum SharedPrivateKey {
    View,
}

/// This trait is required for blockchains to fix the concrete cryptographic key types. The public
/// key associated type is shared across the network.
pub trait Keys {
    /// Private key type given the blockchain and the crypto engine.
    type PrivateKey;

    /// Public key type given the blockchain and the crypto engine.
    type PublicKey: Clone + PartialEq + Debug + StrictEncode + StrictDecode;

    /// Get the bytes from the public key.
    fn as_bytes(pubkey: &Self::PublicKey) -> Vec<u8>;
}

/// Generate the keys for a blockchain from a master seed.
pub trait FromSeed<T>: Keys
where
    T: Blockchain,
{
    /// Type of seed received as input
    type Seed;

    fn get_privkey(seed: &Self::Seed, key_type: T::KeyList) -> Result<Self::PrivateKey, Error>;

    fn get_pubkey(seed: &Self::Seed, key_type: T::KeyList) -> Result<Self::PublicKey, Error>;
}

/// This trait is required for blockchains for fixing the potential shared private key send over
/// the network.
pub trait SharedPrivateKeys<T>: FromSeed<T>
where
    T: Blockchain,
{
    /// A shareable private key type used to parse non-transparent blockchain
    type SharedPrivateKey: Clone + PartialEq + Debug + StrictEncode + StrictDecode;

    fn get_shared_privkey(
        seed: &Self::Seed,
        key_type: SharedPrivateKey,
    ) -> Result<Self::SharedPrivateKey, Error>;

    /// Get the bytes from the shared private key.
    fn as_bytes(privkey: &Self::SharedPrivateKey) -> Vec<u8>;
}

/// This trait is required for blockchains for fixing the commitment types of the keys and
/// parameters that must go through the commit/reveal scheme at the beginning of the protocol.
pub trait Commitment {
    /// Commitment type used in the commit/reveal scheme during swap parameters setup.
    type Commitment: Clone + PartialEq + Eq + Debug + StrictEncode + StrictDecode;

    /// Provides a generic method to commit to any value referencable as stream of bytes.
    fn commit_to<T: AsRef<[u8]>>(value: T) -> Self::Commitment;

    /// Validate the equality between a value and a commitment, return ok if the value commits to
    /// the same commitment's value.
    fn validate<T: AsRef<[u8]>>(value: T, commitment: Self::Commitment) -> Result<(), Error> {
        if Self::commit_to(value) == commitment {
            Ok(())
        } else {
            Err(Error::InvalidCommitment)
        }
    }
}

/// This trait is required for arbitrating blockchains for fixing the types of signatures and
/// adaptor signatures.
pub trait Signatures: Keys {
    /// Defines the signature format for the arbitrating blockchain
    type Signature: Clone + Debug + StrictEncode + StrictDecode;

    /// Defines the adaptor signature format for the arbitrating blockchain. Adaptor signature may
    /// have a different format from the signature depending on the cryptographic primitives used.
    type AdaptorSignature: Clone + Debug + StrictEncode + StrictDecode;

    /// Finalize an adaptor signature into an adapted signature following the regular signature
    /// format.
    fn adapt(key: &Self::PrivateKey, sig: Self::AdaptorSignature)
        -> Result<Self::Signature, Error>;

    /// Recover the encryption key based on the adaptor signature and the decrypted signature.
    fn recover_key(sig: Self::Signature, adapted_sig: Self::AdaptorSignature) -> Self::PrivateKey;
}

/// Define a proving system to link two different blockchain cryptographic group parameters.
pub trait DleqProof<Ar, Ac>: Clone + Debug + StrictEncode + StrictDecode
where
    Ar: Arbitrating,
    Ac: Accordant,
{
    fn project_over(ac_seed: &<Ac as FromSeed<Acc>>::Seed) -> Result<Ar::PrivateKey, Error>;

    fn generate(
        ac_seed: &<Ac as FromSeed<Acc>>::Seed,
    ) -> Result<(Ac::PublicKey, Ar::PublicKey, Self), Error>;

    fn verify(spend: &Ac::PublicKey, adaptor: &Ar::PublicKey, proof: Self) -> Result<(), Error>;
}
