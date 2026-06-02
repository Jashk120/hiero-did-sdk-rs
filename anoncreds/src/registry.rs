use std::sync::Arc;

use crate::utils::{
    compute_status_list_diff, pack_revocation_entry, unpack_revocation_entry,
    RevocationRegistryEntry, RevocationRegistryEntryValue,
};

use hiero_did_core::{DIDError, Signer};
use hiero_did_hcs::{
    HederaHcsService, ResolveFileProps, SubmitFileProps,
};

use crate::types::*;
use crate::utils::{build_anoncreds_identifier, parse_anoncreds_identifier, AnonCredsObjectType};

pub struct HederaAnonCredsRegistry {
    hcs_service: HederaHcsService,
}

impl HederaAnonCredsRegistry {
    pub fn new(hcs_service: HederaHcsService) -> Self {
        Self { hcs_service }
    }

    pub async fn register_schema(
        &self,
        network_name: Option<&str>,
        schema: AnonCredsSchema,
        signer: Arc<dyn Signer>,
    ) -> Result<String, DIDError> {
        let payload = serde_json::to_vec(&schema)
            .map_err(|e| DIDError::SerializationError(e.to_string()))?;

        let topic_id = self.hcs_service
            .submit_file(network_name, SubmitFileProps {
                payload,
                submit_key_signer: signer,
                wait_for_visibility: true,
                wait_timeout_ms: None,
            })
            .await?;

        Ok(build_anoncreds_identifier(
            &schema.issuer_id,
            &topic_id,
            AnonCredsObjectType::Schema,
        ))
    }

    pub async fn get_schema(
        &self,
        schema_id: &str,
    ) -> Result<AnonCredsSchema, DIDError> {
        let parsed = parse_anoncreds_identifier(schema_id)?;

        let payload = self.hcs_service
            .resolve_file(None, &ResolveFileProps { topic_id: parsed.topic_id })
            .await?;

        serde_json::from_slice(&payload)
            .map_err(|e| DIDError::SerializationError(e.to_string()))
    }

    pub async fn register_credential_definition(
        &self,
        network_name: Option<&str>,
        cred_def: AnonCredsCredentialDefinition,
        signer: Arc<dyn Signer>,
    ) -> Result<String, DIDError> {
        let payload = serde_json::to_vec(&cred_def)
            .map_err(|e| DIDError::SerializationError(e.to_string()))?;

        let topic_id = self.hcs_service
            .submit_file(network_name, SubmitFileProps {
                payload,
                submit_key_signer: signer,
                wait_for_visibility: true,
                wait_timeout_ms: None,
            })
            .await?;

        Ok(build_anoncreds_identifier(
            &cred_def.issuer_id,
            &topic_id,
            AnonCredsObjectType::PublicCredDef,
        ))
    }

    pub async fn get_credential_definition(
        &self,
        cred_def_id: &str,
    ) -> Result<AnonCredsCredentialDefinition, DIDError> {
        let parsed = parse_anoncreds_identifier(cred_def_id)?;

        let payload = self.hcs_service
            .resolve_file(None, &ResolveFileProps { topic_id: parsed.topic_id })
            .await?;

        serde_json::from_slice(&payload)
            .map_err(|e| DIDError::SerializationError(e.to_string()))
    }

    pub async fn register_revocation_registry_definition(
        &self,
        network_name: Option<&str>,
        rev_reg_def: AnonCredsRevocationRegistryDefinition,
        signer: Arc<dyn Signer>,
    ) -> Result<String, DIDError> {
        // Create a separate topic for revocation entries
        let entries_topic_id = self.hcs_service
            .create_topic_with_props(
                network_name,
                hiero_did_hcs::CreateTopicProps {
                    submit_key_signer: Some(Arc::clone(&signer)),
                    wait_for_visibility: true,
                    ..Default::default()
                },
            )
            .await?
            .to_string();

        // Bundle the rev reg def with the entries topic ID
        let with_metadata = AnonCredsRevocationRegistryDefinitionWithMetadata {
            rev_reg_def: rev_reg_def.clone(),
            hcs_metadata: HcsMetadata { entries_topic_id },
        };

        let payload = serde_json::to_vec(&with_metadata)
            .map_err(|e| DIDError::SerializationError(e.to_string()))?;

        let topic_id = self.hcs_service
            .submit_file(network_name, SubmitFileProps {
                payload,
                submit_key_signer: signer,
                wait_for_visibility: true,
                wait_timeout_ms: None,
            })
            .await?;

        Ok(build_anoncreds_identifier(
            &rev_reg_def.issuer_id,
            &topic_id,
            AnonCredsObjectType::RevReg,
        ))
    }

    pub async fn get_revocation_registry_definition(
        &self,
        rev_reg_def_id: &str,
    ) -> Result<AnonCredsRevocationRegistryDefinitionWithMetadata, DIDError> {
        let parsed = parse_anoncreds_identifier(rev_reg_def_id)?;

        let payload = self.hcs_service
            .resolve_file(None, &ResolveFileProps { topic_id: parsed.topic_id })
            .await?;

        serde_json::from_slice(&payload)
            .map_err(|e| DIDError::SerializationError(e.to_string()))
    }
    
    pub async fn register_revocation_status_list(
        &self,
        network_name: Option<&str>,
        rev_status_list: AnonCredsRevocationStatusList,
        signer: Arc<dyn Signer>,
    ) -> Result<(), DIDError> {
        let rev_reg_def_with_metadata = self
            .get_revocation_registry_definition(&rev_status_list.rev_reg_def_id)
            .await?;

        let entries_topic_id = rev_reg_def_with_metadata.hcs_metadata.entries_topic_id;

        // Get current status list to compute diff
        let current = self
            .resolve_revocation_status_list(
                network_name,
                &rev_status_list.rev_reg_def_id,
                rev_status_list.timestamp,
            )
            .await
            .ok()
            .flatten();

        let original: Vec<u8> = current
            .as_ref()
            .map(|s| s.revocation_list.clone())
            .unwrap_or_else(|| vec![0u8; rev_status_list.revocation_list.len()]);

        let (issued, revoked) =
            compute_status_list_diff(&original, &rev_status_list.revocation_list)?;

        let entry = RevocationRegistryEntry {
            value: RevocationRegistryEntryValue {
                accum: rev_status_list.current_accumulator.clone(),
                prev_accum: current.map(|s| s.current_accumulator),
                issued: if issued.is_empty() { None } else { Some(issued) },
                revoked: if revoked.is_empty() { None } else { Some(revoked) },
            },
        };

        let message = pack_revocation_entry(&entry)?;

        let topic_id: hiero_sdk::TopicId = entries_topic_id
            .parse()
            .map_err(|_| DIDError::InvalidArgument(format!("Invalid entries topic ID: {entries_topic_id}")))?;

        self.hcs_service
            .submit_message(network_name, topic_id, message.into_bytes(), Some(signer))
            .await?;

        Ok(())
    }

    pub async fn get_revocation_status_list(
        &self,
        network_name: Option<&str>,
        rev_reg_def_id: &str,
        timestamp: u64,
    ) -> Result<AnonCredsRevocationStatusList, DIDError> {
        self.resolve_revocation_status_list(network_name, rev_reg_def_id, timestamp)
            .await?
            .ok_or_else(|| DIDError::NotFound(format!("No revocation list found for {rev_reg_def_id}")))
    }

    async fn resolve_revocation_status_list(
        &self,
        network_name: Option<&str>,
        rev_reg_def_id: &str,
        timestamp: u64,
    ) -> Result<Option<AnonCredsRevocationStatusList>, DIDError> {
        let rev_reg_def_with_metadata = self
            .get_revocation_registry_definition(rev_reg_def_id)
            .await?;

        let entries_topic_id = rev_reg_def_with_metadata.hcs_metadata.entries_topic_id;
        let rev_reg_def = rev_reg_def_with_metadata.rev_reg_def;
        let parsed = parse_anoncreds_identifier(rev_reg_def_id)?;

        let selected_network = network_name.or(Some(parsed.network_name.as_str()));

        let messages = self.hcs_service
            .get_topic_messages(
                selected_network,
                hiero_did_hcs::GetTopicMessagesProps {
                    topic_id: entries_topic_id.parse().map_err(|_| {
                        DIDError::InvalidArgument(format!("Invalid entries topic ID: {entries_topic_id}"))
                    })?,
                    from_time: Some(time::OffsetDateTime::UNIX_EPOCH),
                    to_time: Some(time::OffsetDateTime::from_unix_timestamp(timestamp as i64)
                        .map_err(|e| DIDError::InvalidArgument(format!("Invalid timestamp: {e}")))?),
                    limit: None,
                    max_idle_seconds: Some(5),
                },
            )
            .await?;

        if messages.is_empty() {
            return Ok(None);
        }

        let entries: Vec<RevocationRegistryEntry> = messages
            .iter()
            .filter_map(|m| unpack_revocation_entry(&m.contents))
            .filter(|e| !e.value.accum.is_empty())
            .collect();

        if entries.is_empty() {
            return Ok(None);
        }

        let mut status_list = vec![0u8; rev_reg_def.value.max_cred_num as usize];
        for entry in &entries {
            for &i in entry.value.issued.as_deref().unwrap_or(&[]) {
                if (i as usize) < status_list.len() {
                    status_list[i as usize] = 0;
                }
            }
            for &i in entry.value.revoked.as_deref().unwrap_or(&[]) {
                if (i as usize) < status_list.len() {
                    status_list[i as usize] = 1;
                }
            }
        }

        let current_accumulator = entries.last().unwrap().value.accum.clone();

        Ok(Some(AnonCredsRevocationStatusList {
            issuer_id: rev_reg_def.issuer_id,
            rev_reg_def_id: rev_reg_def_id.to_string(),
            revocation_list: status_list,
            current_accumulator,
            timestamp,
        }))
    }
}
