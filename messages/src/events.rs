use serde::{Deserialize, Serialize};

/// Top-level event wrapper — key determines event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DIDEvent {
    Owner(DIDOwnerEvent),
    // future: VerificationMethod, Service, VerificationRelationship
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
