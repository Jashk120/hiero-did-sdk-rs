pub mod error;
pub mod did;
pub mod document;
pub mod keys;
pub mod signer;

pub use error::DIDError;
pub use did::HederaDid;
pub use document::{
    DIDDocument,
    DIDDocumentMetadata,
    DIDResolution,
    DIDResolutionMetadata,
    VerificationMethod,
    VerificationMethodBase58,
    VerificationMethodMultibase,
    Service,
    KeyCapabilityMethod,
};
pub use keys::KeysUtility;
pub use signer::Signer;