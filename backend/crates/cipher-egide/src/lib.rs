//! Egide Transit adapter for the wealth [`domain::wealth::FieldCipher`] port:
//! envelope encryption with a data key wrapped by Egide.

pub mod egide_client;
pub mod envelope_cipher;

pub use egide_client::{EgideClient, EgideError, GeneratedDatakey};
pub use envelope_cipher::{EgideEnvelopeCipher, UnlockError, WEALTH_KEY_NAME, rewrap_stored_key};
