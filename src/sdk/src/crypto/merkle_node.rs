use core::str::FromStr;
use std::io;

use darkfi_serial::{SerialDecodable, SerialEncodable};
use pasta_curves::{group::ff::PrimeField, pallas};

/// The `MerkleNode` is represented as a base field element.
#[repr(C)]
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, SerialEncodable, SerialDecodable)]
pub struct MerkleNode(pallas::Base);

impl MerkleNode {
    /// Reference the raw inner base field element
    pub fn inner(&self) -> pallas::Base {
        self.0
    }

    /// Try to create a `MerkleNode` type from the given 32 bytes.
    /// Returns `Some` if the bytes fit in the base field, and `None` if not.
    pub fn from_bytes(bytes: [u8; 32]) -> Option<Self> {
        let n = pallas::Base::from_repr(bytes);
        match bool::from(n.is_some()) {
            true => Some(Self(n.unwrap())),
            false => None,
        }
    }

    /// Convert the `MerkleNode` type into 32 raw bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_repr()
    }
}

impl From<pallas::Base> for MerkleNode {
    fn from(x: pallas::Base) -> Self {
        Self(x)
    }
}

impl FromStr for MerkleNode {
    type Err = io::Error;

    /// Tries to decode a base58 string into a `MerkleNode` type.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = match bs58::decode(s).into_vec() {
            Ok(v) => v,
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
        };

        if bytes.len() != 32 {
            return Err(io::Error::new(io::ErrorKind::Other, "Length of decoded bytes is not 32"))
        }

        if let Some(merkle_node) = Self::from_bytes(bytes.try_into().unwrap()) {
            return Ok(merkle_node)
        }

        Err(io::Error::new(io::ErrorKind::Other, "Invalid bytes for MerkleNode"))
    }
}