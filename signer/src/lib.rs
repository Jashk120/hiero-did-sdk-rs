mod internal;

#[cfg(feature = "vault")]
mod vault;
#[cfg(feature = "vault")]
mod vault_api;
#[cfg(feature = "vault")]
mod vault_config;

pub use internal::{InternalSigner, InternalVerifier};

#[cfg(feature = "vault")]
pub use vault::VaultSigner;
#[cfg(feature = "vault")]
pub use vault_config::{VaultAuth, VaultSignerConfig};
