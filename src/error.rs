use thiserror::Error;

/// Main error type for the desktop cli
#[derive(Error, Debug)]
pub enum DesktopCliError {
    #[error("Window not found: {0}")]
    WindowNotFound(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotError(String),

    #[error("Automation error: {0}")]
    AutomationError(String),

    #[error("Invalid coordinates: {0}")]
    CoordinateError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type alias for desktop CLI operations
pub type Result<T> = std::result::Result<T, DesktopCliError>;
