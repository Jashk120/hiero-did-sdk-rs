use hiero_did_core::{
    Accept,
    DIDError,
    DIDResolution,
    HederaDid,
    HederaDidUrl,
    did::Network,
};

use crate::builder::DidDocumentBuilder;
use crate::dereference::{DereferencedResource, dereference_did_with_accept};
use crate::mirror::MirrorNodeClient;
use crate::topic_reader::TopicReader;

/// Resolve a `did:hedera` DID string into a [`DIDResolution`].
///
/// Pass `reader: Some(...)` to use a custom [`TopicReader`]. When `None`,
/// defaults to [`MirrorNodeClient`] auto-selected from the network in the DID.
pub async fn resolve_did(
    did: &str,
    reader: Option<&dyn TopicReader>,
) -> Result<DIDResolution, DIDError> {
    let hedera_did: HederaDid = did
        .parse()
        .map_err(|_| DIDError::InvalidDid(format!("Invalid did:hedera string: {did}")))?;

    let r: &dyn TopicReader;
    let owned;
    if let Some(supplied) = reader {
        r = supplied;
    } else {
        owned = default_reader_for(&hedera_did.network);
        r = &owned;
    }

    DidDocumentBuilder::from_topic_reader(r, &hedera_did.topic_id)
        .await?
        .resolve(&hedera_did)
        .await
}

/// Dereference a `did:hedera` DID URL string into a [`DereferencedResource`].
///
/// Pass `reader: Some(...)` to use a custom [`TopicReader`]. When `None`,
/// defaults to [`MirrorNodeClient`] auto-selected from the network in the DID.
///
/// # Example
/// ```rust,no_run
/// use hiero_did_resolver::dereference_did_url;
///
/// # #[tokio::main]
/// # async fn main() {
/// let resource = dereference_did_url("did:hedera:testnet:zAbc_0.0.12345#did-root-key", None).await.unwrap();
/// # }
/// ```
pub async fn dereference_did_url(
    did_url: &str,
    reader: Option<&dyn TopicReader>,
) -> Result<DereferencedResource, DIDError> {
    dereference_did_url_with_accept(did_url, reader, Accept::DidLdJson).await
}

/// Same as [`dereference_did_url`] but with an explicit [`Accept`] format.
pub async fn dereference_did_url_with_accept(
    did_url: &str,
    reader: Option<&dyn TopicReader>,
    accept: Accept,
) -> Result<DereferencedResource, DIDError> {
    let parsed: HederaDidUrl = did_url
        .parse()
        .map_err(|_| DIDError::InvalidDid(format!("Invalid DID URL: {did_url}")))?;

    let r: &dyn TopicReader;
    let owned;
    if let Some(supplied) = reader {
        r = supplied;
    } else {
        owned = default_reader_for(&parsed.did.network);
        r = &owned;
    }

    let messages = r.get_topic_messages(&parsed.did.topic_id).await?;
    dereference_did_with_accept(&parsed, messages, accept).await
}

pub(crate) fn default_reader_for(network: &Network) -> MirrorNodeClient {
    match network {
        Network::Mainnet => MirrorNodeClient::for_mainnet(),
        Network::Local => MirrorNodeClient::for_local(),
        _ => MirrorNodeClient::for_testnet(),
    }
}