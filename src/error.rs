use crate::semantic::{BackendKind, CapabilityKind, ElementRef};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DesktopCliError {
    #[error("Window not found: {0}")] WindowNotFound(String),
    #[error("Screenshot failed: {0}")] ScreenshotError(String),
    #[error("Gemini API error: {0}")] GeminiError(#[from] GeminiError),
    #[error("Automation error: {0}")] AutomationError(String),
    #[error("Invalid coordinates: {0}")] CoordinateError(String),
    #[error("Execution failed at step {step}: {reason}")] ExecutionError { step: usize, reason: String },
    #[error("IO error: {0}")] IoError(#[from] std::io::Error),
    #[error("Configuration error: {0}")] ConfigError(String),
    #[error("Platform error: {0}")] Platform(String),
}

#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("API request failed: {0}")] RequestFailed(#[from] reqwest::Error),
    #[error("Rate limited (429). Retry after {retry_after_secs}s")] RateLimited { retry_after_secs: u64 },
    #[error("Invalid response schema: {0}")] InvalidSchema(String),
    #[error("Authentication failed: missing or invalid API key")] AuthError,
    #[error("Element not found: {query}")] ElementNotFound { query: String },
    #[error("Bounding box out of bounds: {0}")] BoundingBoxError(String),
    #[error("API returned error status {status}: {message}")] ApiError { status: u16, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DesktopError {
    #[error("permission denied for {backend:?}: {detail}")] PermissionDenied { backend: BackendKind, detail: String },
    #[error("backend unavailable {backend:?}: {detail}")] BackendUnavailable { backend: BackendKind, detail: String },
    #[error("provider disconnected: {backend:?}")] ProviderDisconnected { backend: BackendKind },
    #[error("element not found: {selector}")] ElementNotFound { selector: String },
    #[error("ambiguous selector {selector}: {matches} matches")] AmbiguousSelector { selector: String, matches: usize },
    #[error("stale element: {reference}")] StaleElement { reference: String },
    #[error("unsupported capability {capability:?}")] UnsupportedCapability { element: Option<ElementRef>, capability: CapabilityKind },
    #[error("native error {backend:?}: {message}")] NativeError { backend: BackendKind, code: Option<i64>, message: String },
    #[error("pack error: {0}")] PackError(String),
    #[error("query error: {0}")] QueryError(String),
    #[error("timeout during {operation} after {millis} ms")] Timeout { operation: String, millis: u64 },
    #[error("application not responding")] AppNotResponding { pid: Option<u32> },
    #[error("invalid native value: {type_name}")] InvalidNativeValue { type_name: String },
    #[error("protocol error: {0}")] ProtocolError(String),
    #[error("policy denied action: {0}")] PolicyDenied(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub schema_version: u32,
    pub code: String,
    pub message: String,
    pub retriable: bool,
    pub recovery_hint: Option<String>,
    pub native_backend: Option<BackendKind>,
    pub native_code: Option<i64>,
}

impl DesktopError {
    pub fn envelope(&self) -> ErrorEnvelope {
        let (code, retriable, hint, backend, native_code) = match self {
            Self::PermissionDenied { backend, .. } => ("permission-denied", false, Some("grant the platform accessibility permission and retry".into()), Some(*backend), None),
            Self::BackendUnavailable { backend, .. } => ("backend-unavailable", true, Some("verify the accessibility service/provider is running".into()), Some(*backend), None),
            Self::ProviderDisconnected { backend } => ("provider-disconnected", true, Some("refresh or recreate the desktop session".into()), Some(*backend), None),
            Self::ElementNotFound { .. } => ("element-not-found", true, Some("observe/query the current revision again".into()), None, None),
            Self::AmbiguousSelector { .. } => ("ambiguous-selector", false, Some("use a pack alias, stable id, relation, or narrower selector".into()), None, None),
            Self::StaleElement { .. } => ("stale-element", true, Some("observe/query again and use the new element reference".into()), None, None),
            Self::UnsupportedCapability { .. } => ("unsupported-capability", false, Some("choose another semantic action or an explicitly allowed fallback".into()), None, None),
            Self::NativeError { backend, code, .. } => ("native-error", true, None, Some(*backend), *code),
            Self::PackError(_) => ("pack-error", false, Some("run desktop pack validate/explain".into()), None, None),
            Self::QueryError(_) => ("query-error", false, Some("run query parsing/explain diagnostics".into()), None, None),
            Self::Timeout { .. } => ("timeout", true, Some("narrow the selector or increase the explicit budget".into()), None, None),
            Self::AppNotResponding { .. } => ("app-not-responding", true, None, None, None),
            Self::InvalidNativeValue { .. } => ("invalid-native-value", false, None, None, None),
            Self::ProtocolError(_) => ("protocol-error", false, Some("verify schema_version and request shape".into()), None, None),
            Self::PolicyDenied(_) => ("policy-denied", false, Some("adjust ActionPolicy explicitly if this operation is intended".into()), None, None),
        };
        ErrorEnvelope { schema_version: 2, code: code.into(), message: self.to_string(), retriable, recovery_hint: hint, native_backend: backend, native_code }
    }
}

pub type Result<T> = std::result::Result<T, DesktopCliError>;
pub type GeminiResult<T> = std::result::Result<T, GeminiError>;
pub type DesktopResult<T> = std::result::Result<T, DesktopError>;
