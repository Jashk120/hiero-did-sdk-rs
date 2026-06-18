use serde::{
    Deserialize,
    Serialize,
};

// ---------------------------------------------------------------------------
// DIDOwner (create)
// ---------------------------------------------------------------------------

/// Top-level event wrapper — key determines event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DIDEvent {
    Owner(DIDOwnerEvent),
    AddVerificationMethod(DIDAddVerificationMethodEvent),
    RemoveVerificationMethod(DIDRemoveVerificationMethodEvent),
    AddService(DIDAddServiceEvent),
    RemoveService(DIDRemoveServiceEvent),
    Deactivate(DIDDeactivateEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDOwnerEvent {
    #[serde(rename = "DIDOwner")]
    pub did_owner: DIDOwnerEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDOwnerEventData {
    /// The full DID string (no fragment)
    pub id: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub controller: String,
    /// 'z' + base58btc([0xed, 0x01, ...key_bytes])
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

// ---------------------------------------------------------------------------
// VerificationMethod (add)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDAddVerificationMethodEvent {
    #[serde(rename = "VerificationMethod")]
    pub verification_method: DIDAddVerificationMethodEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDAddVerificationMethodEventData {
    /// Fragment id, e.g. `did:hedera:...#key-1`
    pub id: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
    /// Which relationship bucket: "authentication", "assertionMethod", etc.
    #[serde(rename = "relationshipType")]
    pub relationship_type: String,
}

// ---------------------------------------------------------------------------
// VerificationMethod (remove)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDRemoveVerificationMethodEvent {
    #[serde(rename = "VerificationMethod")]
    pub verification_method: DIDRemoveVerificationMethodEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDRemoveVerificationMethodEventData {
    /// Fragment id to remove, e.g. `did:hedera:...#key-1`
    pub id: String,
}

// ---------------------------------------------------------------------------
// Service (add)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDAddServiceEvent {
    #[serde(rename = "Service")]
    pub service: DIDAddServiceEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDAddServiceEventData {
    /// Fragment id, e.g. `did:hedera:...#vcs`
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

// ---------------------------------------------------------------------------
// Service (remove)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDRemoveServiceEvent {
    #[serde(rename = "Service")]
    pub service: DIDRemoveServiceEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDRemoveServiceEventData {
    /// Fragment id to remove, e.g. `did:hedera:...#vcs`
    pub id: String,
}

// ---------------------------------------------------------------------------
// Deactivate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDeactivateEvent {
    #[serde(rename = "DIDOwner")]
    pub did_owner: DIDDeactivateEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDeactivateEventData {
    pub id: String,
}
