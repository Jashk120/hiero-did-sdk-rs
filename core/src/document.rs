use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum VerificationMethod {
    Base58(VerificationMethodBase58),
    Multibase(VerificationMethodMultibase),
}

impl VerificationMethod {
    pub fn id(&self) -> &str {
        match self {
            Self::Base58(v) => &v.id,
            Self::Multibase(v) => &v.id,
        }
    }

    pub fn controller(&self) -> &str {
        match self {
            Self::Base58(v) => &v.controller,
            Self::Multibase(v) => &v.controller,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationMethodBase58 {
    pub id: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyBase58")]
    pub public_key_base58: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationMethodMultibase {
    pub id: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Service {
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum KeyCapabilityMethod {
    Reference(String),
    Embedded(VerificationMethod),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDocument {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    pub controller: String,
    #[serde(rename = "verificationMethod")]
    pub verification_method: Vec<VerificationMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Vec<Service>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Vec<KeyCapabilityMethod>>,
    #[serde(rename = "assertionMethod", skip_serializing_if = "Option::is_none")]
    pub assertion_method: Option<Vec<KeyCapabilityMethod>>,
    #[serde(rename = "keyAgreement", skip_serializing_if = "Option::is_none")]
    pub key_agreement: Option<Vec<KeyCapabilityMethod>>,
    #[serde(rename = "capabilityInvocation", skip_serializing_if = "Option::is_none")]
    pub capability_invocation: Option<Vec<KeyCapabilityMethod>>,
    #[serde(rename = "capabilityDelegation", skip_serializing_if = "Option::is_none")]
    pub capability_delegation: Option<Vec<KeyCapabilityMethod>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDocumentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deactivated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDResolutionMetadata {
    #[serde(rename = "contentType")]
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDResolution {
    #[serde(rename = "didDocument")]
    pub did_document: DIDDocument,
    #[serde(rename = "didDocumentMetadata")]
    pub did_document_metadata: DIDDocumentMetadata,
    #[serde(rename = "didResolutionMetadata")]
    pub did_resolution_metadata: DIDResolutionMetadata,
}
