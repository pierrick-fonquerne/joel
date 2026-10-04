//! Egide Transit adapter for the wealth [`domain::wealth::FieldCipher`] port:
//! envelope encryption with a data key wrapped by Egide.

pub mod egide_client;

pub use egide_client::{EgideClient, EgideError, GeneratedDatakey};
