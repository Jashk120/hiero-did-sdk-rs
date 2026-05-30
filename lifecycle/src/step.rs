use crate::types::CallbackStep;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleStepKind {
    Callback,
    SignWithSigner,
    AttachSignature,
    Pause,
}

pub struct LifecycleStep<M, C> {
    pub(crate) label: String,
    pub(crate) kind: LifecycleStepKind,
    pub(crate) callback: Option<CallbackStep<M, C>>,
}

impl<M, C> LifecycleStep<M, C> {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn kind(&self) -> &LifecycleStepKind {
        &self.kind
    }
}
