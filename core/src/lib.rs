pub mod did;
pub mod did_url;
pub mod document;
pub mod error;
pub mod keys;
pub mod representation;
pub mod signer;

pub use did::HederaDid;
pub use did_url::HederaDidUrl;
pub use document::{
    DIDDocument,
    DIDDocumentMetadata,
    DIDResolution,
    DIDResolutionMetadata,
    KeyCapabilityMethod,
    Service,
    VerificationMethod,
    VerificationMethodBase58,
    VerificationMethodMultibase,
};
pub use error::DIDError;
pub use keys::KeysUtility;
pub use representation::{
    Accept,
    RepresentedDocument,
};
pub use signer::Signer;
