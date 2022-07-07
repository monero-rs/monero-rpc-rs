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
    fn bytes(&self) -> &[u8];
    /// Parse from `str`.
    fn from_str(v: &str) -> anyhow::Result<Self>;
}

macro_rules! hash_type_impl {
    ($name:ty) => {
        impl HashType for $name {
            fn bytes(&self) -> &[u8] {
                self.as_bytes()
            }
            fn from_str(v: &str) -> anyhow::Result<Self> {
                Ok(v.parse()?)
            }
        }
    };
}

hash_type_impl!(monero::util::address::PaymentId);
hash_type_impl!(monero::cryptonote::hash::Hash);

impl HashType for Vec<u8> {
    fn bytes(&self) -> &[u8] {
        &*self
    }
    fn from_str(v: &str) -> anyhow::Result<Self> {
        Ok(hex::decode(v)?)
    }
}

/// Wrapper type to help serializating types through string.
#[derive(Clone, Debug)]
pub struct HashString<T>(pub T);

impl<T> Display for HashString<T>
where
    T: HashType,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.bytes()))
    }
}

impl<T> Serialize for HashString<T>
where
    T: HashType,
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
