pub mod did_add_service;
pub mod did_add_verification_method;
pub mod did_deactivate;
pub mod did_owner;
pub mod did_remove_service;
pub mod did_remove_verification_method;
pub mod envelope;
pub mod events;

// update — verification methods
pub use did_add_verification_method::DIDAddVerificationMethodMessage;
// create
pub use did_owner::DIDOwnerMessage;
pub use did_remove_verification_method::DIDRemoveVerificationMethodMessage;
pub use envelope::{
    HcsEnvelope,
    HcsMessage,
};
pub use events::{
    DIDAddVerificationMethodEvent,
    DIDAddVerificationMethodEventData,
    DIDOwnerEvent,
    DIDOwnerEventData,
    DIDRemoveVerificationMethodEvent,
    DIDRemoveVerificationMethodEventData,
};
pub mod did_update;
// update — services
pub use did_add_service::DIDAddServiceMessage;
// deactivate
pub use did_deactivate::DIDDeactivateMessage;
pub use did_remove_service::DIDRemoveServiceMessage;
pub use did_update::DIDUpdateMessage;
pub use events::{
    DIDAddServiceEvent,
    DIDAddServiceEventData,
    DIDDeactivateEvent,
    DIDDeactivateEventData,
    DIDRemoveServiceEvent,
    DIDRemoveServiceEventData,
};
