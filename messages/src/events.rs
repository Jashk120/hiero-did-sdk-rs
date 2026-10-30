use hiero_did_core::{DIDError, KeysUtility};
use serde::{Deserialize, Serialize};

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
    ///
    /// Optional: the JS SDK's resolver also accepts a base58btc-only owner
    /// event (see `DIDOwnerEvent.publicKeyBase58` in @hiero-did-sdk/resolver).
    /// Accepting either form keeps us able to resolve DIDs written by other
    /// implementations of the method.
    #[serde(rename = "publicKeyMultibase", default, skip_serializing_if = "Option::is_none")]
    pub public_key_multibase: Option<String>,
    /// Raw base58btc owner key. Never emitted by the JS `DIDOwnerMessage`
    /// writer, but tolerated by its resolver, so we tolerate it too.
    #[serde(rename = "publicKeyBase58", default, skip_serializing_if = "Option::is_none")]
    pub public_key_base58: Option<String>,
}

impl DIDOwnerEventData {
    /// Resolve the owner key from whichever encoding this event carries,
    /// yielding raw 32-byte Ed25519 key material.
    ///
    /// Mirrors the JS resolver, which accepts `publicKeyMultibase` or
    /// `publicKeyBase58`. When both are present the JS implementation lets
    /// base58 win; we do the same so resolution never diverges.
    pub fn effective_key_bytes(&self) -> Result<[u8; 32], DIDError> {
        let bytes = match (&self.public_key_multibase, &self.public_key_base58) {
            (Some(_), Some(b58)) => KeysUtility::from_base58(b58)?.to_bytes().to_vec(),
            (Some(mb), None) => KeysUtility::from_multibase(mb)?.to_bytes().to_vec(),
            (None, Some(b58)) => KeysUtility::from_base58(b58)?.to_bytes().to_vec(),
            (None, None) => {
                return Err(DIDError::InvalidMultibase(
                    "DIDOwner event carries neither publicKeyMultibase nor publicKeyBase58".into(),
                ));
            }
        };

        bytes.try_into().map_err(|_| {
            DIDError::InvalidArgument("DIDOwner event key is not a 32-byte Ed25519 key".into())
        })
    }

    /// The owner key as a multibase string, which is the form written into a
    /// resolved DID Document. Falls back to encoding the base58 form so that a
    /// base58-only event still produces a well-formed document, matching the
    /// JS `DIDOwner.publicKeyMultibase ?? fromBase58(...).toMultibase()`.
    pub fn effective_key_multibase(&self) -> Result<String, DIDError> {
        if let Some(mb) = &self.public_key_multibase {
            return Ok(mb.clone());
        }
        let bytes = self.effective_key_bytes()?;
        Ok(KeysUtility::from_bytes(bytes.to_vec()).to_multibase())
    }
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
