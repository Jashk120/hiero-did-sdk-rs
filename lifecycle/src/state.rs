#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerStatus {
    Success,
    Error,
    Pause,
}

#[derive(Debug, Clone)]
pub struct RunnerState<M, C = ()> {
    pub message: M,
    pub context: C,
    pub status: RunnerStatus,
    pub index: isize,
    pub label: String,
    pub error: Option<String>,
}

impl<M, C> RunnerState<M, C> {
    pub(crate) fn success(message: M, context: C) -> Self {
        Self {
            message,
            context,
            status: RunnerStatus::Success,
            index: -1,
            label: String::new(),
            error: None,
        }
    }

    pub(crate) fn error(message: M, context: C, error: String) -> Self {
        Self {
            message,
            context,
            status: RunnerStatus::Error,
            index: -1,
            label: String::new(),
            error: Some(error),
        }
    }

    pub(crate) fn pause(message: M, context: C, index: usize, label: String) -> Self {
        Self {
            message,
            context,
            status: RunnerStatus::Pause,
            index: index as isize,
            label,
            error: None,
        }
    }
}