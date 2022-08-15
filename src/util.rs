// Copyright 2019-2022 Artem Vorotnikov and Monero Rust Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{self, Display};

/// Get bytes and parse from `str` interface.
pub trait HashType: Sized {
    /// Get bytes representation.
    fn bytes(&self) -> &[u8]
    where
        Self: AsRef<[u8]>,
    {
        self.as_ref()
    }
    /// Parse from `str`.
    fn from_str(v: &str) -> anyhow::Result<Self>;
}

macro_rules! hash_type_impl {
    ($name:ty) => {
        impl HashType for $name {
            fn from_str(v: &str) -> anyhow::Result<Self> {
                Ok(v.parse()?)
            }
        }
    };
}

hash_type_impl!(monero::util::address::PaymentId);
hash_type_impl!(monero::cryptonote::hash::Hash);

impl HashType for Vec<u8> {
    fn from_str(v: &str) -> anyhow::Result<Self> {
        let v = v.strip_prefix("0x").unwrap_or(v);
        Ok(hex::decode(v)?)
    }
}

/// Wrapper type to help serializating types through string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashString<T>(pub T);

impl<T> Display for HashString<T>
where
    T: HashType + AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.bytes()))
    }
}

impl<T> Serialize for HashString<T>
where
    T: HashType + AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, T> Deserialize<'de> for HashString<T>
where
    T: HashType,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(T::from_str(&s).map_err(serde::de::Error::custom)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn trait_hash_type_for_payment_id() {
        use monero::util::address::PaymentId;

        let payment_id = PaymentId([0, 1, 2, 3, 4, 5, 6, 7]);

        assert_eq!(payment_id.bytes(), &[0, 1, 2, 3, 4, 5, 6, 7]);

        assert!(<PaymentId as HashType>::from_str("")
            .unwrap_err()
            .is::<rustc_hex::FromHexError>());
        assert!(<PaymentId as HashType>::from_str("0x01234567")
            .unwrap_err()
            .is::<rustc_hex::FromHexError>());
        assert!(<PaymentId as HashType>::from_str("0xgg")
            .unwrap_err()
            .is::<rustc_hex::FromHexError>());

        assert_eq!(
            <PaymentId as HashType>::from_str("0x0001020304050607").unwrap(),
            payment_id
        );
        assert_eq!(
            <PaymentId as HashType>::from_str("0001020304050607").unwrap(),
            payment_id
        );
    }

    #[test]
    fn trait_hash_type_for_cryptonote_hash() {
        use monero::cryptonote::hash::Hash;

        let hash = Hash([250; 32]);

        assert_eq!(hash.bytes(), [250; 32].as_slice());

        assert!(<Hash as HashType>::from_str("")
            .unwrap_err()
            .is::<rustc_hex::FromHexError>());
        assert!(<Hash as HashType>::from_str("0x01234567")
            .unwrap_err()
            .is::<rustc_hex::FromHexError>());
        assert!(<Hash as HashType>::from_str("0xgg")
            .unwrap_err()
            .is::<rustc_hex::FromHexError>());

        let hash_str = "fa".repeat(32);
        assert_eq!(<Hash as HashType>::from_str(&hash_str).unwrap(), hash);

        let hash_str = format!("0x{}", hash_str);
        assert_eq!(<Hash as HashType>::from_str(&hash_str).unwrap(), hash);
    }

    #[test]
    fn trait_hash_type_for_vec_u8() {
        let vec_non_empty = vec![0, 1, 2, 3, 4];

        assert_eq!(vec_non_empty.bytes(), &[0, 1, 2, 3, 4]);

        assert_eq!(
            <Vec<u8> as HashType>::from_str("").unwrap(),
            Vec::<u8>::new()
        );
        assert!(<Vec<u8> as HashType>::from_str("0xgg")
            .unwrap_err()
            .is::<hex::FromHexError>());

        assert_eq!(
            <Vec<u8> as HashType>::from_str("0x0001020304").unwrap(),
            vec_non_empty
        );
        assert_eq!(
            <Vec<u8> as HashType>::from_str("0001020304").unwrap(),
            vec_non_empty
        );
    }

    #[test]
    fn display_for_hash_string() {
        let vec = vec![0, 1, 2, 3, 4];
        let hash_string = HashString(vec);
        assert_eq!(hash_string.to_string(), "0001020304");
    }

    #[test]
    fn se_de_for_hash_string() {
        let vec = vec![0, 1, 2, 3, 4];
        let hash_string = HashString(vec);

        assert_tokens(&hash_string, &[Token::Str("0001020304")]);
    }
}
