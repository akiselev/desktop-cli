#[cfg(windows)]
use crate::automation::windows::{capture_screenshot, click_at_coords, parse_hwnd, type_text, ScreenshotMethod};
use crate::error::{DesktopCliError, Result};
use crate::executor::parser::{is_dangerous_instruction, validate_instructions};
use crate::executor::state::{ExecutionState, ExecutionSummary};
use crate::gemini::client::GeminiClient;
#[cfg(windows)]
use crate::gemini::bounding_box::{convert_to_pixels, NormalizedBoundingBox};
#[cfg(windows)]
use crate::gemini::retry::{detect_element_with_retry, RetryStrategy};
#[cfg(windows)]
use windows::Win32::Foundation::HWND;

/// Multi-step instruction executor
pub struct Executor {
    gemini_client: GeminiClient,
}

impl Executor {
    /// Create a new executor with Gemini client
    pub fn new(gemini_client: GeminiClient) -> Self {
        Self { gemini_client }
    }

    /// Execute a list of natural language instructions on a target window
    #[cfg(windows)]
    pub async fn execute_instructions(
        &self,
        hwnd_str: &str,
        instructions: Vec<String>,
        retry_strategy: RetryStrategy,
        warn_on_dangerous: bool,
    ) -> Result<ExecutionSummary> {
        // Validate instructions
        let validated_instructions = validate_instructions(&instructions)?;

        // Warn about dangerous instructions
        if warn_on_dangerous {
            for (i, instruction) in validated_instructions.iter().enumerate() {
                if is_dangerous_instruction(instruction) {
                    tracing::warn!(
                        "Potentially dangerous instruction detected at step {}: {}",
                        i + 1,
                        instruction
                    );
                }
            }
        }

        // Parse HWND and convert to isize (Send-safe)
        let hwnd = parse_hwnd(hwnd_str)?;
        let hwnd_raw = hwnd.0 as isize;

        // Initialize execution state
        let mut state = ExecutionState::new(validated_instructions.len());

        tracing::info!(
            "Starting execution of {} instructions on window {}",
            validated_instructions.len(),
            hwnd_str
        );

        // Execute each instruction
        for (i, instruction) in validated_instructions.iter().enumerate() {
            tracing::info!("Step {}/{}: {}", i + 1, validated_instructions.len(), instruction);

            match self
                .execute_single_instruction(hwnd_raw, instruction, &retry_strategy)
                .await
            {
                Ok(coords) => {
                    state.record_success(instruction.clone(), Some(coords));
                    tracing::info!("Step {} completed successfully", i + 1);
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    state.record_failure(instruction.clone(), error_msg.clone());
                    tracing::error!("Step {} failed: {}", i + 1, error_msg);
                    // Continue with remaining steps instead of failing entirely
                }
            }
        }

        let summary = state.to_summary();
        tracing::info!(
            "Execution complete: {}/{} steps successful",
            summary.executed_steps - summary.failed_steps,
            summary.executed_steps
        );

        Ok(summary)
    }

    /// Execute a single instruction
    #[cfg(windows)]
    async fn execute_single_instruction(
        &self,
        hwnd_raw: isize,
        instruction: &str,
        retry_strategy: &RetryStrategy,
    ) -> Result<(i32, i32)> {
        // Step 1: Capture screenshot
        tracing::debug!("Capturing screenshot of window");
        let screenshot = capture_screenshot(HWND(hwnd_raw as *mut _), ScreenshotMethod::default())
            .map_err(|e| DesktopCliError::ExecutionError {
                step: 0,
                reason: format!("Screenshot failed: {}", e),
            })?;

        // Step 2: Send to Gemini for element detection
        tracing::debug!("Detecting element with Gemini: {}", instruction);
        let detection_result = detect_element_with_retry(
            &self.gemini_client,
            instruction,
            &screenshot.base64_image,
            retry_strategy,
        )
        .await
        .map_err(|e| DesktopCliError::ExecutionError {
            step: 1,
            reason: format!("Element detection failed: {}", e),
        })?;

        if !detection_result.element_found {
            return Err(DesktopCliError::ExecutionError {
                step: 1,
                reason: format!("Element not found: {}", instruction),
            });
        }

        tracing::debug!(
            "Element found: '{}' with confidence {}",
            detection_result.label,
            detection_result.confidence
        );

        // Step 3: Convert bounding box to pixel coordinates
        if detection_result.bounding_box.len() != 4 {
            return Err(DesktopCliError::ExecutionError {
                step: 2,
                reason: "Invalid bounding box format".to_string(),
            });
        }

        let normalized_bbox = NormalizedBoundingBox::from_array([
            detection_result.bounding_box[0],
            detection_result.bounding_box[1],
            detection_result.bounding_box[2],
            detection_result.bounding_box[3],
        ]);

        let pixel_bbox = convert_to_pixels(&normalized_bbox, screenshot.width, screenshot.height)
            .map_err(|e| DesktopCliError::ExecutionError {
                step: 2,
                reason: format!("Coordinate conversion failed: {}", e),
            })?;

        let (center_x, center_y) = pixel_bbox.center();
        tracing::debug!("Target coordinates: ({}, {})", center_x, center_y);

        // Step 4: Execute the action based on action type
        match detection_result.action_type.as_str() {
            "click" => {
                tracing::debug!("Executing click at ({}, {})", center_x, center_y);
                click_at_coords(HWND(hwnd_raw as *mut _), center_x, center_y).map_err(|e| {
                    DesktopCliError::ExecutionError {
                        step: 3,
                        reason: format!("Click failed: {}", e),
                    }
                })?;
            }

            "type" => {
                // First click to focus, then type
                tracing::debug!("Clicking to focus at ({}, {})", center_x, center_y);
                click_at_coords(HWND(hwnd_raw as *mut _), center_x, center_y).map_err(|e| {
                    DesktopCliError::ExecutionError {
                        step: 3,
                        reason: format!("Click to focus failed: {}", e),
                    }
                })?;

                // Extract text to type from instruction or action_params
                let text_to_type = if let Some(params) = detection_result.action_params {
                    params["text"]
                        .as_str()
                        .unwrap_or(instruction)
                        .to_string()
                } else {
                    // Try to extract text from instruction (e.g., "type hello world" -> "hello world")
                    extract_text_from_instruction(instruction)
                };

                tracing::debug!("Typing text: {}", text_to_type);
                type_text(&text_to_type).map_err(|e| DesktopCliError::ExecutionError {
                    step: 3,
                    reason: format!("Typing failed: {}", e),
                })?;
            }

            "scroll" | "drag" | "hover" => {
                // These actions are not yet implemented
                tracing::warn!("Action type '{}' not yet implemented", detection_result.action_type);
                return Err(DesktopCliError::ExecutionError {
                    step: 3,
                    reason: format!("Action type '{}' not implemented", detection_result.action_type),
                });
            }

            _ => {
                return Err(DesktopCliError::ExecutionError {
                    step: 3,
                    reason: format!("Unknown action type: {}", detection_result.action_type),
                });
            }
        }

        // Add small delay to let UI respond
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        Ok((center_x, center_y))
    }
}

/// Extract text to type from an instruction like "type hello world"
fn extract_text_from_instruction(instruction: &str) -> String {
    let lower = instruction.to_lowercase();

    // Try common patterns
    if let Some(idx) = lower.find("type ") {
        return instruction[idx + 5..].trim().to_string();
    }

    if let Some(idx) = lower.find("enter ") {
        return instruction[idx + 6..].trim().to_string();
    }

    if let Some(idx) = lower.find("input ") {
        return instruction[idx + 6..].trim().to_string();
    }

    // Fallback: return the whole instruction
    instruction.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_instruction() {
        assert_eq!(
            extract_text_from_instruction("type hello world"),
            "hello world"
        );
        assert_eq!(
            extract_text_from_instruction("Type Hello World"),
            "Hello World"
        );
        assert_eq!(
            extract_text_from_instruction("enter test@example.com"),
            "test@example.com"
        );
        assert_eq!(
            extract_text_from_instruction("input 12345"),
            "12345"
        );
        assert_eq!(
            extract_text_from_instruction("just text"),
            "just text"
        );
    }
}
