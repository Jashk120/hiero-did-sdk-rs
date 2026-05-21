use crate::builder::DidDocumentBuilder;
use hiero_did_core::{DIDError,HederaDidUrl, DIDDocument, VerificationMethod, Service};




pub enum DereferencedResource {
    VerificationMethod(VerificationMethod),
    Service(Service),
    Document(DIDDocument),
}

pub async fn dereference_did(
    did_url: &HederaDidUrl,
    messages: Vec<String>,
) -> Result<DereferencedResource, DIDError> {

   // resolve the document
    let builder = DidDocumentBuilder::from(messages);
    let resolution = builder.resolve(&did_url.did).await?;
    let doc = resolution.did_document;

    if did_url.path.is_some() || !did_url.params.is_empty() {
        return Err(DIDError::InvalidArgument("Path and query params are not yet supported".into()));
    }

    //no fragment = return whole document
    let fragment = match &did_url.fragment {
        None => return Ok(DereferencedResource::Document(doc)),
        Some(f) => f,
    };

    //build the full id to search for
    let full_id = format!("{}#{}", did_url.did.to_did_string(), fragment);
    
    //search verification_method by id
    if let Some(vm) = doc.verification_method.iter().find(|vm| vm.id() == full_id) {
        return Ok(DereferencedResource::VerificationMethod(vm.clone()));
    }

    if let Some(services) = &doc.service {
        if let Some(svc) = services.iter().find(|s| s.id == full_id) {
            return Ok(DereferencedResource::Service(svc.clone()));
        }
    }
    
    //nothing found
    Err(DIDError::NotFound(format!("No resource found for: {}", full_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hiero_did_core::{HederaDid, HederaDidUrl, KeysUtility};
    use hiero_did_core::did::Network;
    use hiero_did_signer::InternalSigner;
    use hiero_did_messages::DIDOwnerMessage;

    fn fixture() -> (HederaDid, InternalSigner) {
        let signer = InternalSigner::from_bytes(&[3u8; 32]).expect("signer");
        let did = HederaDid::new(
            Network::Testnet,
            KeysUtility::from_bytes(signer.verifying_key_bytes()).to_base58(),
            "0.0.321".to_string(),
        );
        (did, signer)
    }

    fn signed_create(did: &HederaDid, signer: &InternalSigner) -> String {
        let msg = DIDOwnerMessage::new(did.clone(), signer.verifying_key_bytes(), None);
        let sig = signer.sign(&msg.message_bytes().unwrap());
        msg.to_payload(&sig).unwrap()
    }

    #[tokio::test]
    async fn no_fragment_returns_document() {
        let (did, signer) = fixture();
        let messages = vec![signed_create(&did, &signer)];
        let did_url: HederaDidUrl = did.to_did_string().parse().expect("valid url");
        let result = dereference_did(&did_url, messages).await;
        assert!(matches!(result, Ok(DereferencedResource::Document(_))));
    }

    #[tokio::test]
    async fn fragment_not_found_returns_error() {
        let (did, signer) = fixture();
        let messages = vec![signed_create(&did, &signer)];
        let url_str = format!("{}#nonexistent", did.to_did_string());
        let did_url: HederaDidUrl = url_str.parse().expect("valid url");
        let result = dereference_did(&did_url, messages).await;
        assert!(matches!(result, Err(DIDError::NotFound(_))));
    }
}