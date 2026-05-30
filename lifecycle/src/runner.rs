use crate::builder::LifecycleBuilder;
use crate::message::LifecycleMessage;
use crate::state::RunnerState;
use crate::state::RunnerStatus;
use crate::step::LifecycleStepKind;
use crate::types::Hook;
use crate::types::LifecycleFuture;
use hiero_did_core::DIDError;
use hiero_did_core::Signer;
use std::collections::HashMap;

pub struct LifecycleRunnerOptions<'a, C> {
    pub context: C,
    pub signer: Option<&'a dyn Signer>,
    pub signature: Option<Vec<u8>>,
}

impl<C> LifecycleRunnerOptions<'_, C> {
    pub fn new(context: C) -> Self {
        Self {
            context,
            signer: None,
            signature: None,
        }
    }
}

pub struct LifecycleRunner<M, C = ()> {
    builder: LifecycleBuilder<M, C>,
    hooks: HashMap<String, Vec<Hook<M>>>,
}

impl<M, C> LifecycleRunner<M, C>
where
    M: LifecycleMessage + Send,
    C: Send,
{
    pub fn new(builder: LifecycleBuilder<M, C>) -> Self {
        Self {
            builder,
            hooks: HashMap::new(),
        }
    }

    pub fn on_complete<F>(&mut self, label: impl Into<String>, hook: F)
    where
        F: for<'a> Fn(&'a M) -> LifecycleFuture<'a, Result<(), DIDError>> + Send + Sync + 'static,
    {
        self.hooks
            .entry(label.into())
            .or_default()
            .push(Box::new(hook));
    }

    pub async fn process(
        &self,
        message: M,
        options: LifecycleRunnerOptions<'_, C>,
    ) -> Result<RunnerState<M>, DIDError> {
        self.process_from(message, options, None).await
    }

    pub async fn resume(
        &self,
        state: RunnerState<M>,
        options: LifecycleRunnerOptions<'_, C>,
    ) -> Result<RunnerState<M>, DIDError> {
        if state.status != RunnerStatus::Pause {
            return Err(DIDError::InvalidArgument(
                "Only paused lifecycle states can be resumed".into(),
            ));
        }
        let label = state.label.clone();
        self.process_from(state.message, options, Some(label)).await
    }

    async fn process_from(
        &self,
        mut message: M,
        mut options: LifecycleRunnerOptions<'_, C>,
        resume_after_label: Option<String>,
    ) -> Result<RunnerState<M>, DIDError> {
        let start_index = if let Some(label) = resume_after_label {
            let pause_index = self.builder.get_index_by_label(&label).ok_or_else(|| {
                DIDError::InvalidArgument(format!("Unknown lifecycle resume label: {label}"))
            })?;
            self.run_hooks(&label, &message).await?;
            pause_index + 1
        } else {
            0
        };

        match self
            .execute_steps(&mut message, &mut options, start_index)
            .await
        {
            Ok(Some((index, label))) => Ok(RunnerState::pause(message, index, label)),
            Ok(None) => Ok(RunnerState::success(message)),
            Err(error) => {
                if let Some((_, catch_step)) = &self.builder.catch_step {
                    let message_text = error.to_string();
                    catch_step(error).await?;
                    Ok(RunnerState::error(message, message_text))
                } else {
                    Err(error)
                }
            }
        }
    }

    async fn execute_steps(
        &self,
        message: &mut M,
        options: &mut LifecycleRunnerOptions<'_, C>,
        start_index: usize,
    ) -> Result<Option<(usize, String)>, DIDError> {
        for (index, step) in self.builder.steps.iter().enumerate().skip(start_index) {
            match step.kind {
                LifecycleStepKind::Callback => {
                    let callback = step.callback.as_ref().ok_or_else(|| {
                        DIDError::InternalError(format!(
                            "Callback step has no callback: {}",
                            step.label
                        ))
                    })?;
                    callback(message, &mut options.context).await?;
                    self.run_hooks(&step.label, message).await?;
                }
                LifecycleStepKind::SignWithSigner => {
                    let signer = options.signer.ok_or_else(|| {
                        DIDError::InvalidArgument(format!(
                            "Lifecycle step '{}' requires a signer",
                            step.label
                        ))
                    })?;
                    let bytes = message.message_bytes()?;
                    let signature = signer.sign_bytes(&bytes)?;
                    message.set_signature(signature)?;
                    self.run_hooks(&step.label, message).await?;
                }
                LifecycleStepKind::AttachSignature => {
                    let signature = options.signature.take().ok_or_else(|| {
                        DIDError::InvalidArgument(format!(
                            "Lifecycle step '{}' requires a signature",
                            step.label
                        ))
                    })?;
                    message.set_signature(signature)?;
                    self.run_hooks(&step.label, message).await?;
                }
                LifecycleStepKind::Pause => {
                    return Ok(Some((index, step.label.clone())));
                }
            }
        }

        Ok(None)
    }

    async fn run_hooks(&self, label: &str, message: &M) -> Result<(), DIDError> {
        if let Some(hooks) = self.hooks.get(label) {
            for hook in hooks {
                hook(message).await?;
            }
        }
        Ok(())
    }
}
