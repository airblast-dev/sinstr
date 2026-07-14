use serde::{Deserialize, Serialize, de::Visitor};

use crate::{NonEmptySinStr, SinStr};

struct NonEmptySinStrVisitor;

impl<'de> Visitor<'de> for NonEmptySinStrVisitor {
    type Value = NonEmptySinStr;
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        NonEmptySinStr::new(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(v),
            &self,
        ))
    }

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        write!(formatter, "Expected string with at least a 1 byte of data")
    }
}

impl<'de> Deserialize<'de> for NonEmptySinStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(NonEmptySinStrVisitor)
    }
}

impl Serialize for NonEmptySinStr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

struct SinStrVisitor;

impl<'de> Visitor<'de> for SinStrVisitor {
    type Value = SinStr;
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(SinStr::new(v))
    }

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        write!(formatter, "Expected string")
    }
}

impl<'de> Deserialize<'de> for SinStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(SinStrVisitor)
    }
}

impl Serialize for SinStr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
