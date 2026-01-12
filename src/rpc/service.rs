use remoc::rtc;

use crate::automation::types::WindowInfo;
use crate::gemini::GeminiClient;
use crate::rpc::types::*;

#[cfg(windows)]
use crate::automation::windows::{capture_screenshot, list_windows, parse_hwnd, ScreenshotMethod};
#[cfg(windows)]
use crate::executor::Executor;
#[cfg(windows)]
use crate::gemini::retry::RetryStrategy;

/// Desktop automation service trait - remotely callable via RTC
#[rtc::remote]
pub trait DesktopService: Sync {
    /// List all visible windows, optionally filtered
    async fn list_windows(&self, req: ListWindowsRequest) -> Result<Vec<WindowInfo>, ServiceError>;

    /// Take a screenshot of a window
    async fn take_screenshot(&self, req: ScreenshotRequest) -> Result<Screenshot, ServiceError>;

    /// Execute natural language instructions on a window
    async fn execute_instructions(
        &mut self,
        req: ExecuteRequest,
    ) -> Result<ExecutionSummary, ServiceError>;

    /// Detect UI elements using visual grounding
    async fn detect_elements(&self, req: DetectRequest) -> Result<DetectionResult, ServiceError>;
}

/// Implementation of the desktop service
pub struct DesktopServiceImpl {
    pub gemini_client: GeminiClient,
    pub allowed_executables: Vec<String>,
}

impl DesktopServiceImpl {
    pub fn new(gemini_client: GeminiClient, allowed_executables: Vec<String>) -> Self {
        Self {
            gemini_client,
            allowed_executables,
        }
    }
}

impl DesktopService for DesktopServiceImpl {
    #[cfg(windows)]
    async fn list_windows(&self, req: ListWindowsRequest) -> Result<Vec<WindowInfo>, ServiceError> {
        // If allowed_executables is configured, filter by it
        let exe_filter = if !self.allowed_executables.is_empty() {
            if let Some(ref exe) = req.executable_filter {
                if !self
                    .allowed_executables
                    .iter()
                    .any(|allowed| exe.contains(allowed))
                {
                    return Err(ServiceError::ConfigError(format!(
                        "Executable '{}' not in allowed list",
                        exe
                    )));
                }
                Some(exe.as_str())
            } else {
                Some("") // This will match nothing
            }
        } else {
            req.executable_filter.as_deref()
        };

        list_windows(exe_filter, req.title_pattern.as_deref())
            .map_err(|e| ServiceError::AutomationError(e.to_string()))
    }

    #[cfg(not(windows))]
    async fn list_windows(&self, _req: ListWindowsRequest) -> Result<Vec<WindowInfo>, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn take_screenshot(&self, req: ScreenshotRequest) -> Result<Screenshot, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;
        let method = req
            .method
            .as_ref()
            .and_then(|m| ScreenshotMethod::from_str(m))
            .unwrap_or_default();

        let screenshot = capture_screenshot(hwnd, method)
            .map_err(|e| ServiceError::ScreenshotError(e.to_string()))?;

        Ok(Screenshot {
            base64_image: screenshot.base64_image,
            width: screenshot.width,
            height: screenshot.height,
            format: screenshot.format,
        })
    }

    #[cfg(not(windows))]
    async fn take_screenshot(&self, _req: ScreenshotRequest) -> Result<Screenshot, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn execute_instructions(
        &mut self,
        req: ExecuteRequest,
    ) -> Result<ExecutionSummary, ServiceError> {
        let retry_strategy = req
            .retry_strategy
            .as_ref()
            .and_then(|s| RetryStrategy::from_str(s))
            .unwrap_or_default();

        let executor = Executor::new(self.gemini_client.clone());
        let summary = executor
            .execute_instructions(&req.hwnd, req.instructions, retry_strategy, true)
            .await
            .map_err(|e| match e {
                crate::error::DesktopCliError::ExecutionError { step, reason } => {
                    ServiceError::ExecutionError { step, reason }
                }
                _ => ServiceError::AutomationError(e.to_string()),
            })?;

        Ok(ExecutionSummary {
            total_steps: summary.total_steps,
            completed_steps: summary.completed_steps,
            success: summary.success,
            last_error: summary.last_error,
            coordinates: summary.coordinates,
        })
    }

    #[cfg(not(windows))]
    async fn execute_instructions(
        &mut self,
        _req: ExecuteRequest,
    ) -> Result<ExecutionSummary, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn detect_elements(&self, req: DetectRequest) -> Result<DetectionResult, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;
        let screenshot = capture_screenshot(hwnd, ScreenshotMethod::default())
            .map_err(|e| ServiceError::ScreenshotError(e.to_string()))?;

        let result = self
            .gemini_client
            .detect_element(&req.query, &screenshot.base64_image)
            .await
            .map_err(|e| ServiceError::GeminiError(e.to_string()))?;

        Ok(DetectionResult {
            element_found: result.element_found,
            label: result.label,
            bounding_box: Some(result.bounding_box),
            action_type: result.action_type,
            confidence: result.confidence,
        })
    }

    #[cfg(not(windows))]
    async fn detect_elements(&self, _req: DetectRequest) -> Result<DetectionResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }
}
