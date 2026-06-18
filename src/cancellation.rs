use std::fmt;

#[derive(Debug, Clone)]
pub struct OperationCanceled {
    message: String,
}

impl OperationCanceled {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for OperationCanceled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for OperationCanceled {}

pub fn canceled(message: impl Into<String>) -> anyhow::Error {
    OperationCanceled::new(message).into()
}

pub fn is_canceled(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.downcast_ref::<OperationCanceled>().is_some())
}
