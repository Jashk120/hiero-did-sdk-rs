use serde::{
    Deserialize,
    Serialize,
};

/// Media types a caller can request when resolving or dereferencing a DID.
/// Mirrors the `Accept` union in the JS SDK (`packages/resolver/src/interfaces/accept.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accept {
    DidJson,
    DidLdJson,
    DidResolution,
    DidCbor,
}

impl Accept {
    pub fn as_content_type(&self) -> &'static str {
        match self {
            Accept::DidJson => "application/did+json",
            Accept::DidLdJson => "application/did+ld+json",
            Accept::DidResolution => {
                "application/ld+json;profile=\"https://w3id.org/did-resolution\""
            }
            Accept::DidCbor => "application/did+cbor",
        }
    }
}

impl std::str::FromStr for Accept {
    type Err = crate::error::DIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "application/did+json" => Ok(Accept::DidJson),
            "application/did+ld+json" => Ok(Accept::DidLdJson),
            "application/ld+json;profile=\"https://w3id.org/did-resolution\"" => {
                Ok(Accept::DidResolution)
            }
            "application/did+cbor" => Ok(Accept::DidCbor),
            other => Err(crate::error::DIDError::RepresentationNotSupported(other.to_string())),
        }
    }
}

/// Output of representation negotiation: either a JSON-shaped value or raw CBOR bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RepresentedDocument {
    Json(serde_json::Value),
    Cbor(Vec<u8>),
}
