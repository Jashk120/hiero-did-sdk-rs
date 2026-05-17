pub mod envelope;
pub mod did_owner;
pub mod did_deactivate;
pub mod did_add_verification_method;
pub mod did_remove_verification_method;
pub mod did_add_service;
pub mod did_remove_service;
pub mod events;

pub use envelope::{HcsEnvelope, HcsMessage};

// create
pub use did_owner::DIDOwnerMessage;
pub use events::{DIDOwnerEvent, DIDOwnerEventData};

// update — verification methods
pub use did_add_verification_method::DIDAddVerificationMethodMessage;
pub use did_remove_verification_method::DIDRemoveVerificationMethodMessage;
pub use events::{
    DIDAddVerificationMethodEvent, DIDAddVerificationMethodEventData,
    DIDRemoveVerificationMethodEvent, DIDRemoveVerificationMethodEventData,
};

// update — services
pub use did_add_service::DIDAddServiceMessage;
pub use did_remove_service::DIDRemoveServiceMessage;
pub use events::{
    DIDAddServiceEvent, DIDAddServiceEventData,
    DIDRemoveServiceEvent, DIDRemoveServiceEventData,
};

// deactivate
pub use did_deactivate::DIDDeactivateMessage;
pub use events::{DIDDeactivateEvent, DIDDeactivateEventData};