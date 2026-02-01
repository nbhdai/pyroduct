/// A trait for errors that can define their own retry strategy.
pub trait Retryable: std::error::Error {
    /// Returns true if the operation that caused this error should be retried.
    fn is_retryable(&self) -> bool;
}

/// A trait that generic libraries can implement to define how their errors
/// map to the unified UserError.
pub trait Reportable: std::error::Error {
    fn to_user_error(&self) -> PyroductError;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PyroductError {
    pub message: String,
    pub category: String,
    pub is_retryable: bool,
    pub details: Option<String>,
}
