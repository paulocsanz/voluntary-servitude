//! Serde's Serialize/Deserialize trait implementations behind the 'serde-traits' feature
//!
//! # Serialize
//!  - [`VoluntaryServitude`]
//!  - [`VSIter`]
//!
//! # Deserialize
//!  - [`VoluntaryServitude`]
//!
//! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html#implementations
//! [`VSIter`]: ../struct.VSIter.html#implementations
//!
//! Enable the feature:
//!
//! **Cargo.toml**
//! [dependencies]
//! ```voluntary_servitude = { version = "3", features = "serde-traits" }```
//!
//! For testing the feature "serde-tests" must be enabled
//! ```bash
//! cargo test --features "serde-traits serde-tests"
//! ```

use iterator::VSIter;
use serde_lib::{
    de::SeqAccess, de::Visitor, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, fmt::Formatter, marker::PhantomData};
use voluntary_servitude::{VSInner, VoluntaryServitude};

struct VSInnerVisitor<'a, T: 'a + Deserialize<'a>>(pub PhantomData<&'a T>);

impl<'a, T: Deserialize<'a>> Visitor<'a> for VSInnerVisitor<'a, T> {
    type Value = VSInner<T>;

    #[inline]
    fn expecting(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "a list")
    }

    #[inline]
    fn visit_seq<A: SeqAccess<'a>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let inner = VSInner::<T>::default();
        while let Some(value) = seq.next_element()? {
            inner.append(value);
        }
        Ok(inner)
    }
}

impl<'a, T: Serialize> Serialize for VSIter<'a, T> {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        debug!("Serialize VSIter");
        let len = self.len();
        let mut seq = ser.serialize_seq(Some(len))?;
        for (el, _) in self.clone().zip(0..len) {
            seq.serialize_element(el)?;
        }
        seq.end()
    }
}

impl<'a, T: 'a + Deserialize<'a>> Deserialize<'a> for VSInner<T> {
    #[inline]
    fn deserialize<D: Deserializer<'a>>(des: D) -> Result<Self, D::Error> {
        debug!("Deserialize VSInner");
        des.deserialize_seq(VSInnerVisitor(PhantomData))
    }
}

impl<T: Serialize> Serialize for VoluntaryServitude<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        trace!("Serialize VoluntaryServitude");
        self.iter().serialize(ser)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde-tests")]
    extern crate serde_json;
    use VoluntaryServitude;

    #[test]
    #[cfg(not(feature = "serde-tests"))]
    fn improperly_testing_serde() {
        #[cfg(not(feature = "serde-tests"))]
        compile_error!(
            "You must enable 'serde-tests', or disable 'serde-traits' to properly test the library"
        );
    }

    #[test]
    fn serde() {
        let string = serde_json::to_string(&vs![1u8, 2u8, 3u8, 4u8]).unwrap();
        let vs: VoluntaryServitude<u8> = serde_json::from_str(&string).unwrap();
        assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1u8, &2u8, &3u8, &4u8]);
    }
}
