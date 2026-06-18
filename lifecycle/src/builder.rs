use hiero_did_core::DIDError;

use crate::step::{
    LifecycleStep,
    LifecycleStepKind,
};
use crate::types::{
    CatchStep,
    LifecycleFuture,
};

pub struct LifecycleBuilder<M, C = ()> {
    pub(crate) steps: Vec<LifecycleStep<M, C>>,
    pub(crate) catch_step: Option<(String, CatchStep)>,
}

impl<M, C> Default for LifecycleBuilder<M, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M, C> LifecycleBuilder<M, C> {
    pub fn new() -> Self {
        Self { steps: Vec::new(), catch_step: None }
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn get_by_index(&self, index: usize) -> Option<&LifecycleStep<M, C>> {
        self.steps.get(index)
    }

    pub fn get_by_label(&self, label: &str) -> Option<&LifecycleStep<M, C>> {
        self.steps.iter().find(|step| step.label == label)
    }

    pub fn get_index_by_label(&self, label: &str) -> Option<usize> {
        self.steps.iter().position(|step| step.label == label)
    }

    pub fn catch_label(&self) -> Option<&str> {
        self.catch_step.as_ref().map(|(label, _)| label.as_str())
    }

    pub fn callback<F>(mut self, label: impl Into<String>, callback: F) -> Result<Self, DIDError>
    where
        F: for<'a> Fn(&'a mut M, &'a mut C) -> LifecycleFuture<'a, Result<(), DIDError>>
            + Send
            + Sync
            + 'static,
    {
        let label = label.into();
        self.ensure_unique_label(&label)?;
        self.steps.push(LifecycleStep {
            label,
            kind: LifecycleStepKind::Callback,
            callback: Some(Box::new(callback)),
        });
        Ok(self)
    }

    pub fn sign_with_signer(mut self, label: impl Into<String>) -> Result<Self, DIDError> {
        let label = label.into();
        self.ensure_unique_label(&label)?;
        self.steps.push(LifecycleStep {
            label,
            kind: LifecycleStepKind::SignWithSigner,
            callback: None,
        });
        Ok(self)
    }

    pub fn attach_signature(mut self, label: impl Into<String>) -> Result<Self, DIDError> {
        let label = label.into();
        self.ensure_unique_label(&label)?;
        self.steps.push(LifecycleStep {
            label,
            kind: LifecycleStepKind::AttachSignature,
            callback: None,
        });
        Ok(self)
    }

    pub fn pause(mut self, label: impl Into<String>) -> Result<Self, DIDError> {
        let label = label.into();
        self.ensure_unique_label(&label)?;
        self.steps.push(LifecycleStep { label, kind: LifecycleStepKind::Pause, callback: None });
        Ok(self)
    }

    pub fn catch<F>(mut self, label: impl Into<String>, callback: F) -> Self
    where
        F: Fn(DIDError) -> LifecycleFuture<'static, Result<(), DIDError>> + Send + Sync + 'static,
    {
        self.catch_step = Some((label.into(), Box::new(callback)));
        self
    }

    fn ensure_unique_label(&self, label: &str) -> Result<(), DIDError> {
        if self.steps.iter().any(|step| step.label == label) {
            return Err(DIDError::InvalidArgument(format!(
                "Duplicate lifecycle step label: {label}"
            )));
        }
        Ok(())
    }
}
