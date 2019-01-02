//! Trait implementations to integrate with other crates

#[cfg(feature = "rayon-traits")]
mod rayon;

#[cfg(feature = "serde-traits")]
mod serde;

#[cfg(feature = "diesel-traits")]
mod diesel;
