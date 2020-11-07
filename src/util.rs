use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{self, Display};

pub trait HashType: Sized {
    fn bytes(&self) -> &[u8];
    fn from_str(v: &str) -> Result<Self, super::Error>;
}

macro_rules! hash_type_impl {
    ($name:ty) => {
        impl HashType for $name {
            fn bytes(&self) -> &[u8] {
                self.as_bytes()
            }
            fn from_str(v: &str) -> Result<Self, crate::Error> {
                Ok(v.parse().map_err($crate::Error::from_parse_error)?)
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
    fn from_str(v: &str) -> Result<Self, crate::Error> {
        Ok(hex::decode(v).map_err(crate::Error::from_parse_error)?)
    }
}

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

impl<'a, T> Serialize for HashString<T>
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
