//! Serde's `Serialize`/`Deserialize` trait implementations for [`VoluntaryServitude`]
//!
//! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html#implementations
//!
//! Enable the feature:
//!
//! **Cargo.toml**
//!
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "serde-traits" }
//! ```

use serde_lib::{de::SeqAccess, de::Visitor, ser::SerializeSeq};
use serde_lib::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, fmt::Formatter, marker::PhantomData};
use voluntary_servitude::{Inner, VoluntaryServitude};

/// Abstracts deserializer visitor
struct InnerVisitor<'a, T: Deserialize<'a>>(pub PhantomData<(&'a (), T)>);

impl<'a, T: Deserialize<'a>> Visitor<'a> for InnerVisitor<'a, T> {
    type Value = Inner<T>;

    #[inline]
    fn expecting(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "a list")
    }

    #[inline]
    fn visit_seq<A: SeqAccess<'a>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let inner: Inner<T> = Inner::default();
        while let Some(value) = seq.next_element()? {
            inner.append(value);
        }
        Ok(inner)
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Inner<T> {
    #[inline]
    fn deserialize<D: Deserializer<'a>>(des: D) -> Result<Self, D::Error> {
        debug!("Deserialize Inner");
        des.deserialize_seq(InnerVisitor(PhantomData))
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "serde-traits")))]
impl<T: Serialize> Serialize for VoluntaryServitude<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        trace!("Serialize VoluntaryServitude");
        let len = self.len();
        let mut sequence = ser.serialize_seq(Some(len))?;
        for (el, _) in self.iter().zip(0..len) {
            sequence.serialize_element(el)?;
        }
        sequence.end()
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "serde-traits")))]
impl<'a, T: Deserialize<'a>> Deserialize<'a> for VoluntaryServitude<T> {
    #[inline]
    fn deserialize<D: Deserializer<'a>>(des: D) -> Result<Self, D::Error> {
        Inner::deserialize(des).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_json;
    use VS;

    #[derive(Serialize, Deserialize)]
    struct Derive<T>(pub VS<T>);

    #[test]
    fn derive_json() {
        let string = serde_json::to_string(&Derive(vs![1u8, 2u8, 3u8, 4u8])).unwrap();
        let vs: Derive<u8> = serde_json::from_str(&string).unwrap();
        assert_eq!(
            vs.0.iter().collect::<Vec<_>>(),
            vec![&1u8, &2u8, &3u8, &4u8]
        );
    }

    #[test]
    fn json() {
        let string = serde_json::to_string(&vs![1u8, 2u8, 3u8, 4u8]).unwrap();
        let vs: VS<u8> = serde_json::from_str(&string).unwrap();
        assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1u8, &2u8, &3u8, &4u8]);
    }
}
