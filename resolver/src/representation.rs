use hiero_did_core::{Accept, DIDError, DIDResolution, RepresentedDocument};

/// Render a resolved DID document into the representation the caller asked for.
/// Mirrors JS's `DIDDereferenceBuilder.toJson/toJsonLd/toResolution/toCbor`.
pub fn represent(
    resolution: &DIDResolution,
    accept: Accept,
) -> Result<RepresentedDocument, DIDError> {
    match accept {
        Accept::DidJson => {
            let value = serde_json::to_value(&resolution.did_document)
                .map_err(|e| DIDError::SerializationError(e.to_string()))?;
            Ok(RepresentedDocument::Json(value))
        }
        Accept::DidLdJson => {
            // Same shape as DidJson today since DIDDocument already carries @context;
            // kept as a separate arm so JSON-LD-specific framing can diverge later
            // without touching callers.
            let value = serde_json::to_value(&resolution.did_document)
                .map_err(|e| DIDError::SerializationError(e.to_string()))?;
            Ok(RepresentedDocument::Json(value))
        }
        Accept::DidResolution => {
            let value = serde_json::to_value(resolution)
                .map_err(|e| DIDError::SerializationError(e.to_string()))?;
            Ok(RepresentedDocument::Json(value))
        }
        Accept::DidCbor => {
            let mut bytes = Vec::new();
            ciborium::ser::into_writer(&resolution.did_document, &mut bytes)
                .map_err(|e| DIDError::SerializationError(e.to_string()))?;
            Ok(RepresentedDocument::Cbor(bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hiero_did_core::{DIDDocument, DIDDocumentMetadata, DIDResolutionMetadata};
    
    fn fixture_resolution() -> DIDResolution {
        DIDResolution {
            did_document: DIDDocument {
                context: vec!["https://www.w3.org/ns/did/v1".to_string()],
                id: "did:hedera:testnet:abc_0.0.1".to_string(),
                controller: "did:hedera:testnet:abc_0.0.1".to_string(),
                verification_method: vec![],
                service: None,
                authentication: None,
                assertion_method: None,
                key_agreement: None,
                capability_invocation: None,
                capability_delegation: None,
            },
            did_document_metadata: DIDDocumentMetadata {
                created: None,
                updated: None,
                deactivated: Some(false),
            },
            did_resolution_metadata: DIDResolutionMetadata {
                content_type: "application/ld+json;profile=\"https://w3id.org/did-resolution\""
                    .to_string(),
            },
        }
    }

    #[test]
    fn did_json_returns_document_only() {
        let resolution = fixture_resolution();
        let result = represent(&resolution, Accept::DidJson).unwrap();
        match result {
            RepresentedDocument::Json(v) => assert_eq!(v["id"], "did:hedera:testnet:abc_0.0.1"),
            _ => panic!("expected JSON"),
        }
    }

    #[test]
    fn did_resolution_wraps_full_envelope() {
        let resolution = fixture_resolution();
        let result = represent(&resolution, Accept::DidResolution).unwrap();
        match result {
            RepresentedDocument::Json(v) => assert!(v.get("didDocument").is_some()),
            _ => panic!("expected JSON"),
        }
    }

    #[test]
    fn did_cbor_produces_bytes() {
        let resolution = fixture_resolution();
        let result = represent(&resolution, Accept::DidCbor).unwrap();
        match result {
            RepresentedDocument::Cbor(bytes) => assert!(!bytes.is_empty()),
            _ => panic!("expected CBOR"),
        }
    }
}