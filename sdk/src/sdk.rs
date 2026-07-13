use hiero_did_anoncreds::HederaAnonCredsRegistry;
use hiero_did_client::{
    HederaClientConfiguration,
    HederaClientService,
};
use hiero_did_core::did::Network;
use hiero_did_core::{
    DIDError,
    DIDResolution,
    HederaDid,
    Signer,
};
use hiero_did_hcs::{
    HcsCacheService,
    HederaHcsService,
};
use hiero_did_registrar::{
    CreateDIDResult,
    CreateDIDWithSignerResult,
    CsmBatchSigningRequest,
    CsmBatchSubmitRequest,
    CsmBatchSubmitResult,
    CsmPrepareOptions,
    CsmSigningRequest,
    CsmSubmitRequest,
    CsmSubmitResult,
    DIDUpdateOperation,
    DeactivateDIDResult,
    UpdateDIDResult,
    create_did,
    create_did_with_signer,
    deactivate_did,
    deactivate_did_with_signer,
    prepare_create_did_csm,
    prepare_create_did_csm_with_options,
    prepare_deactivate_did_csm,
    prepare_deactivate_did_csm_with_options,
    prepare_update_did_csm,
    prepare_update_did_csm_with_options,
    submit_create_did_csm,
    submit_deactivate_did_csm,
    submit_update_did_csm,
    update_did,
    update_did_with_signer,
};
use hiero_did_resolver::TopicReader;
#[cfg(feature = "vault")]
use hiero_did_signer::{
    VaultSigner,
    VaultSignerConfig,
};
use hiero_did_utils::polling::poll_until;

/// The top-level SDK handler that simplifies interaction with the did:hedera SDK.
/// It wraps client configuration, HCS services, DID operations (lifecycle & CSM),
/// and AnonCreds registry functions to shield developer consumers from internal details.
#[derive(Clone)]
pub struct HieroDidSdk {
    client_service: HederaClientService,
    hcs_service: HederaHcsService,
    anoncreds_registry: HederaAnonCredsRegistry,
}

impl HieroDidSdk {
    /// Create a new `HieroDidSdk` handler from a `HederaClientService` and optional `HcsCacheService`.
    pub fn new(client_service: HederaClientService, cache: Option<HcsCacheService>) -> Self {
        let hcs_service = HederaHcsService::new(client_service.clone(), cache);
        let anoncreds_registry = HederaAnonCredsRegistry::new(hcs_service.clone());
        Self { client_service, hcs_service, anoncreds_registry }
    }

    /// Create a new `HieroDidSdk` handler from a `HederaClientConfiguration` and optional `HcsCacheService`.
    pub fn from_config(
        config: HederaClientConfiguration,
        cache: Option<HcsCacheService>,
    ) -> Result<Self, DIDError> {
        let client_service = HederaClientService::new(config)?;
        Ok(Self::new(client_service, cache))
    }

    /// Access the underlying `HederaClientService`.
    pub fn client_service(&self) -> &HederaClientService {
        &self.client_service
    }

    /// Access the underlying `HederaHcsService`.
    pub fn hcs_service(&self) -> &HederaHcsService {
        &self.hcs_service
    }

    /// Access the `HederaAnonCredsRegistry` instance.
    pub fn anoncreds(&self) -> &HederaAnonCredsRegistry {
        &self.anoncreds_registry
    }

    /// Factory method to create a `VaultSigner` using the specified config.
    /// Only available if the `"vault"` feature is enabled on the SDK.
    #[cfg(feature = "vault")]
    pub fn new_vault_signer(&self, config: VaultSignerConfig) -> Result<VaultSigner, DIDError> {
        VaultSigner::new(config)
    }

    // ── Did Lifecycle Operations ──────────────────────────────────────────────

    /// Creates a new did:hedera DID using the selected network config.
    pub async fn create_did(
        &self,
        network_name: Option<&str>,
        network: Network,
        controller: Option<String>,
    ) -> Result<CreateDIDResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        create_did(&client, network, controller).await
    }

    /// Creates a DID using an externally-managed signer (e.g. `VaultSigner`).
    pub async fn create_did_with_signer(
        &self,
        network_name: Option<&str>,
        network: Network,
        controller: Option<String>,
        signer: &dyn Signer,
    ) -> Result<CreateDIDWithSignerResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        create_did_with_signer(&client, network, controller, signer).await
    }

    /// Deactivates a DID.
    pub async fn deactivate_did(
        &self,
        network_name: Option<&str>,
        did: HederaDid,
        private_key_bytes: &[u8],
    ) -> Result<DeactivateDIDResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        deactivate_did(&client, did, private_key_bytes).await
    }

    /// Deactivates a DID using an externally-managed signer (e.g. `VaultSigner`).
    pub async fn deactivate_did_with_signer(
        &self,
        network_name: Option<&str>,
        did: HederaDid,
        signer: &dyn Signer,
    ) -> Result<DeactivateDIDResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        deactivate_did_with_signer(&client, did, signer).await
    }

    /// Updates a DID with new operations.
    pub async fn update_did(
        &self,
        network_name: Option<&str>,
        did: HederaDid,
        private_key_bytes: &[u8],
        operations: Vec<DIDUpdateOperation>,
    ) -> Result<UpdateDIDResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        update_did(&client, did, private_key_bytes, operations).await
    }

    /// Updates a DID with new operations using an externally-managed signer (e.g. `VaultSigner`).
    pub async fn update_did_with_signer(
        &self,
        network_name: Option<&str>,
        did: HederaDid,
        operations: Vec<DIDUpdateOperation>,
        signer: &dyn Signer,
    ) -> Result<UpdateDIDResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        update_did_with_signer(&client, did, signer, operations).await
    }

    // ── Did Cryptographic State Machine (CSM) Operations ──────────────────────

    /// Prepares a DID creation request to be signed offline/externally.
    pub async fn prepare_create_did_csm(
        &self,
        network_name: Option<&str>,
        network: Network,
        public_key_bytes: Vec<u8>,
        controller: Option<String>,
    ) -> Result<CsmSigningRequest, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        prepare_create_did_csm(&client, network, public_key_bytes, controller).await
    }

    /// Prepares a DID creation request with extra parameters.
    pub async fn prepare_create_did_csm_with_options(
        &self,
        network_name: Option<&str>,
        network: Network,
        public_key_bytes: Vec<u8>,
        controller: Option<String>,
        options: CsmPrepareOptions,
    ) -> Result<CsmSigningRequest, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        prepare_create_did_csm_with_options(&client, network, public_key_bytes, controller, options)
            .await
    }

    /// Submits a signed DID creation request.
    pub async fn submit_create_did_csm(
        &self,
        network_name: Option<&str>,
        request: CsmSubmitRequest,
    ) -> Result<CsmSubmitResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        submit_create_did_csm(&client, request).await
    }

    /// Prepares a DID deactivation request to be signed offline/externally.
    pub async fn prepare_deactivate_did_csm(
        &self,
        did: HederaDid,
    ) -> Result<CsmSigningRequest, DIDError> {
        prepare_deactivate_did_csm(did).await
    }

    /// Prepares a DID deactivation request with extra parameters.
    pub async fn prepare_deactivate_did_csm_with_options(
        &self,
        did: HederaDid,
        options: CsmPrepareOptions,
    ) -> Result<CsmSigningRequest, DIDError> {
        prepare_deactivate_did_csm_with_options(did, options).await
    }

    /// Submits a signed DID deactivation request.
    pub async fn submit_deactivate_did_csm(
        &self,
        network_name: Option<&str>,
        request: CsmSubmitRequest,
    ) -> Result<CsmSubmitResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        submit_deactivate_did_csm(&client, request).await
    }

    /// Prepares a DID update batch request to be signed offline/externally.
    pub async fn prepare_update_did_csm(
        &self,
        did: HederaDid,
        operations: Vec<DIDUpdateOperation>,
    ) -> Result<CsmBatchSigningRequest, DIDError> {
        prepare_update_did_csm(did, operations).await
    }

    /// Prepares a DID update batch request with extra parameters.
    pub async fn prepare_update_did_csm_with_options(
        &self,
        did: HederaDid,
        operations: Vec<DIDUpdateOperation>,
        options: CsmPrepareOptions,
    ) -> Result<CsmBatchSigningRequest, DIDError> {
        prepare_update_did_csm_with_options(did, operations, options).await
    }

    /// Submits signed DID update batch requests.
    pub async fn submit_update_did_csm(
        &self,
        network_name: Option<&str>,
        request: CsmBatchSubmitRequest,
    ) -> Result<CsmBatchSubmitResult, DIDError> {
        let client = self.client_service.get_client(network_name)?;
        submit_update_did_csm(&client, request).await
    }

    // ── Did Resolution ────────────────────────────────────────────────────────

    /// Resolves a `did:hedera` DID string using default mirror node reader or custom reader.
    pub async fn resolve_did(
        &self,
        did: &str,
        reader: Option<&dyn TopicReader>,
    ) -> Result<DIDResolution, DIDError> {
        hiero_did_resolver::resolve_did(did, reader).await
    }

    /// Dereferences a DID URL (e.g. `did:hedera:testnet:...#key-1`).
    pub async fn dereference_did_url(
        &self,
        did_url: &str,
        reader: Option<&dyn TopicReader>,
    ) -> Result<hiero_did_resolver::dereference::DereferencedResource, DIDError> {
        hiero_did_resolver::dereference_did_url(did_url, reader).await
    }
    /// Resolves a DID repeatedly until `predicate` returns true or `timeout_secs` elapses.
    ///
    /// Use this instead of a fixed delay after `update_did`/`deactivate_did` when the
    /// caller needs to observe their own write — mirror ingestion lag means an
    /// immediate `resolve_did` call is not guaranteed to reflect a just-submitted change.
    pub async fn resolve_did_until<F>(
        &self,
        did: &str,
        timeout_secs: u64,
        predicate: F,
    ) -> Result<DIDResolution, DIDError>
    where
        F: Fn(&DIDResolution) -> bool,
    {
        poll_until(
            || async {
                let resolution = self.resolve_did(did, None).await.ok()?;
                if predicate(&resolution) { Some(Ok(resolution)) } else { None }
            },
            timeout_secs,
            300,
        )
        .await
        .unwrap_or_else(|| {
            Err(DIDError::InternalError(format!(
                "Timed out waiting for expected resolved state on {did}"
            )))
        })
    }
}
