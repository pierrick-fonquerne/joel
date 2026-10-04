//! Runner library surface (bootstrap wiring), kept testable.

pub mod bootstrap;
pub use bootstrap::{BootstrapError, bootstrap};
pub mod exchange_rate_refresh;
