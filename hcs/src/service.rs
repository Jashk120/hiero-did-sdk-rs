use hiero_did_client::HederaClientService;
use hiero_did_core::{DIDError, Signer};
use hiero_sdk::TopicId;
use std::sync::Arc;

use crate::cache::HcsCacheService;
use crate::hcs::{
    CreateTopicProps, DeleteTopicProps, GetTopicMessagesProps, HcsFileService, HcsMessage,
    HcsTopic, ResolveFileProps, SubmitFileProps, SubmitMessageResult, TopicInfo, TopicMessageData,
    UpdateTopicProps,
};

pub struct HederaHcsService {
    client_service: HederaClientService,
    cache: Option<HcsCacheService>,
}

impl HederaHcsService {
    pub fn new(client_service: HederaClientService, cache: Option<HcsCacheService>) -> Self {
        Self {
            client_service,
            cache,
        }
    }

    pub async fn create_topic(&self, network_name: Option<&str>) -> Result<TopicId, DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsTopic::create(&client).await
            })
            .await
    }

    pub async fn create_topic_with_memo(
        &self,
        network_name: Option<&str>,
        memo: &str,
    ) -> Result<TopicId, DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsTopic::create_with_memo(&client, memo).await
            })
            .await
    }

    pub async fn create_topic_with_props(
        &self,
        network_name: Option<&str>,
        props: CreateTopicProps,
    ) -> Result<TopicId, DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsTopic::create_with_props(&client, props).await
            })
            .await
    }

    pub async fn update_topic(
        &self,
        network_name: Option<&str>,
        props: UpdateTopicProps,
    ) -> Result<(), DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsTopic::update(&client, props).await
            })
            .await
    }

    pub async fn delete_topic(
        &self,
        network_name: Option<&str>,
        topic_id: TopicId,
    ) -> Result<(), DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsTopic::delete(&client, topic_id).await
            })
            .await
    }

    pub async fn delete_topic_with_props(
        &self,
        network_name: Option<&str>,
        props: DeleteTopicProps,
    ) -> Result<(), DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsTopic::delete_with_props(&client, props).await
            })
            .await
    }

    pub async fn get_topic_info(
        &self,
        network_name: Option<&str>,
        topic_id: TopicId,
    ) -> Result<TopicInfo, DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsTopic::get_info(&client, topic_id).await
            })
            .await
    }

    pub async fn submit_message(
        &self,
        network_name: Option<&str>,
        topic_id: TopicId,
        message: impl Into<Vec<u8>>,
        submit_key_signer: Option<Arc<dyn Signer>>,
    ) -> Result<SubmitMessageResult, DIDError> {
        self.client_service
            .with_client(network_name, |client| async move {
                HcsMessage::submit(&client, topic_id, message, submit_key_signer).await
            })
            .await
    }

    pub async fn get_topic_messages(
        &self,
        network_name: Option<&str>,
        props: GetTopicMessagesProps,
    ) -> Result<Vec<TopicMessageData>, DIDError> {
        let cache = self.cache.clone();
        let cache_network_key = network_name.unwrap_or("default").to_string();
        self.client_service
            .with_client(network_name, |client| async move {
                HcsMessage::get_topic_messages_with_cache(
                    &client,
                    props,
                    &cache_network_key,
                    cache.as_ref(),
                )
                .await
            })
            .await
    }

    pub async fn submit_file(
        &self,
        network_name: Option<&str>,
        props: SubmitFileProps,
    ) -> Result<String, DIDError> {
        let cache = self.cache.clone();
        let cache_network_key = network_name.unwrap_or("default").to_string();
        self.client_service
            .with_client(network_name, |client| async move {
                let svc = HcsFileService::new(&client, cache_network_key, cache);
                svc.submit_file(props).await
            })
            .await
    }

    pub async fn resolve_file(
        &self,
        network_name: Option<&str>,
        props: &ResolveFileProps,
    ) -> Result<Vec<u8>, DIDError> {
        let cache = self.cache.clone();
        let cache_network_key = network_name.unwrap_or("default").to_string();
        self.client_service
            .with_client(network_name, |client| async move {
                let svc = HcsFileService::new(&client, cache_network_key, cache);
                svc.resolve_file(props).await
            })
            .await
    }
}
