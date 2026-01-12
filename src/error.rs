use thiserror::Error;

/// Main error type for the desktop cli server
#[derive(Error, Debug)]
pub enum DesktopCliError {
    #[error("Window not found: {0}")]
    WindowNotFound(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotError(String),

    #[error("Gemini API error: {0}")]
    GeminiError(#[from] GeminiError),

    #[error("Automation error: {0}")]
    AutomationError(String),

    #[error("Invalid coordinates: {0}")]
    CoordinateError(String),

    #[error("Execution failed at step {step}: {reason}")]
    ExecutionError { step: usize, reason: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Errors specific to Gemini API integration
#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Rate limited (429). Retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Invalid response schema: {0}")]
    InvalidSchema(String),

    #[error("Authentication failed: missing or invalid API key")]
    AuthError,

    #[error("Element not found: {query}")]
    ElementNotFound { query: String },

    #[error("Bounding box out of bounds: {0}")]
    BoundingBoxError(String),

    #[error("API returned error status {status}: {message}")]
    ApiError { status: u16, message: String },
}

/// Result type alias for desktop CLI operations
pub type Result<T> = std::result::Result<T, DesktopCliError>;

/// Result type alias for Gemini operations
pub type GeminiResult<T> = std::result::Result<T, GeminiError>;
