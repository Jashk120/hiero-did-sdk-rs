#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerStatus {
    Success,
    Error,
    Pause,
}

#[derive(Debug, Clone)]
pub struct RunnerState<M> {
    pub message: M,
    pub status: RunnerStatus,
    pub index: isize,
    pub label: String,
    pub error: Option<String>,
}

impl<M> RunnerState<M> {
    pub(crate) fn success(message: M) -> Self {
        Self {
            message,
            status: RunnerStatus::Success,
            index: -1,
            label: String::new(),
            error: None,
        }
    }

    pub(crate) fn error(message: M, error: String) -> Self {
        Self {
            message,
            status: RunnerStatus::Error,
            index: -1,
            label: String::new(),
            error: Some(error),
        }
    }

    pub(crate) fn pause(message: M, index: usize, label: String) -> Self {
        Self {
            message,
            status: RunnerStatus::Pause,
            index: index as isize,
            label,
            error: None,
        }
    }
}
