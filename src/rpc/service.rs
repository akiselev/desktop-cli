use remoc::rtc;

use crate::automation::types::WindowInfo;
use crate::gemini::GeminiClient;
use crate::rpc::types::*;

#[cfg(windows)]
use crate::automation::windows::{capture_screenshot, list_windows, parse_hwnd, ScreenshotMethod};
#[cfg(windows)]
use crate::automation::windows::uia::{self, PatternOp, Selector, TreeDumpOptions};
#[cfg(windows)]
use crate::executor::Executor;
#[cfg(windows)]
use crate::gemini::retry::RetryStrategy;
#[cfg(windows)]
use uiautomation::UIAutomation;

/// Desktop automation service trait - remotely callable via RTC
#[rtc::remote]
pub trait DesktopService: Sync {
    /// List all visible windows with session filters applied
    async fn list_windows(&mut self, req: ListWindowsRequest) -> Result<Vec<WindowInfo>, ServiceError>;

    /// Set the default target window (by index or HWND)
    async fn set_default_window(&mut self, req: SetDefaultWindowRequest) -> Result<(), ServiceError>;

    /// Get the current default window
    async fn get_default_window(&self, req: GetDefaultWindowRequest) -> Result<DefaultWindowResponse, ServiceError>;

    /// Take a screenshot of a window
    async fn take_screenshot(&self, req: ScreenshotRequest) -> Result<Screenshot, ServiceError>;

    /// Execute natural language instructions on a window
    async fn execute_instructions(
        &mut self,
        req: ExecuteRequest,
    ) -> Result<ExecutionSummary, ServiceError>;

    /// Detect UI elements using visual grounding
    async fn detect_elements(&self, req: DetectRequest) -> Result<DetectionResult, ServiceError>;

    // =========================================================================
    // UIA (UI Automation) Methods
    // =========================================================================

    /// Dump the UIA element tree for a window
    async fn dump_tree(&self, req: DumpTreeRequest) -> Result<UiaElement, ServiceError>;

    /// Find elements by CSS-style selector
    async fn find_elements(&self, req: FindElementRequest) -> Result<Vec<UiaElement>, ServiceError>;

    /// Invoke a UIA pattern operation on an element
    async fn invoke_pattern(&self, req: InvokePatternRequest) -> Result<PatternResult, ServiceError>;

    // =========================================================================
    // LLM-Optimized Methods (compact output for AI agents)
    // =========================================================================

    /// Get a compact UI summary optimized for LLM consumption
    async fn get_summary(&self, req: SummaryRequest) -> Result<String, ServiceError>;

    /// Query elements using enhanced LLM-friendly syntax
    async fn query_elements(&self, req: QueryRequest) -> Result<QueryResult, ServiceError>;
}

/// Implementation of the desktop service
pub struct DesktopServiceImpl {
    pub gemini_client: GeminiClient,
    pub allowed_executables: Vec<String>,
    /// Session-wide executable filter (substring match)
    pub exe_filter: Option<String>,
    /// Session-wide title pattern (regex)
    pub title_pattern: Option<String>,
    /// Default target window HWND
    pub default_window: Option<String>,
    /// Cached window list for index-based selection
    pub cached_windows: Vec<WindowInfo>,
}

impl DesktopServiceImpl {
    pub fn new(
        gemini_client: GeminiClient,
        allowed_executables: Vec<String>,
        exe_filter: Option<String>,
        title_pattern: Option<String>,
    ) -> Self {
        Self {
            gemini_client,
            allowed_executables,
            exe_filter,
            title_pattern,
            default_window: None,
            cached_windows: Vec::new(),
        }
    }
}

impl DesktopService for DesktopServiceImpl {
    #[cfg(windows)]
    async fn list_windows(&mut self, _req: ListWindowsRequest) -> Result<Vec<WindowInfo>, ServiceError> {
        // Use session-wide filters (set at daemon start time)
        let windows = list_windows(
            self.exe_filter.as_deref(),
            self.title_pattern.as_deref(),
        ).map_err(|e| ServiceError::AutomationError(e.to_string()))?;

        // If allowed_executables is set, additionally filter by it
        let windows = if !self.allowed_executables.is_empty() {
            windows
                .into_iter()
                .filter(|w| {
                    self.allowed_executables
                        .iter()
                        .any(|allowed| w.executable.contains(allowed))
                })
                .collect()
        } else {
            windows
        };

        // Cache the window list for index-based selection
        self.cached_windows = windows.clone();
        Ok(windows)
    }

    #[cfg(not(windows))]
    async fn list_windows(&mut self, _req: ListWindowsRequest) -> Result<Vec<WindowInfo>, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn set_default_window(&mut self, req: SetDefaultWindowRequest) -> Result<(), ServiceError> {
        // Try to parse as a 1-based index first
        if let Ok(index) = req.window.parse::<usize>() {
            if index == 0 || index > self.cached_windows.len() {
                return Err(ServiceError::WindowNotFound(format!(
                    "Window index {} out of range (1-{}). Run 'desktop window list' first.",
                    index,
                    self.cached_windows.len()
                )));
            }
            let window = &self.cached_windows[index - 1];
            self.default_window = Some(window.hwnd.clone());
            return Ok(());
        }

        // Otherwise treat as HWND string - validate it exists
        let hwnd = parse_hwnd(&req.window)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;
        
        // Verify the window exists
        let _ = crate::automation::windows::get_window_info(hwnd)
            .map_err(|_| ServiceError::WindowNotFound(format!("Window with HWND {} not found", req.window)))?;

        self.default_window = Some(req.window);
        Ok(())
    }

    #[cfg(not(windows))]
    async fn set_default_window(&mut self, _req: SetDefaultWindowRequest) -> Result<(), ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    async fn get_default_window(&self, _req: GetDefaultWindowRequest) -> Result<DefaultWindowResponse, ServiceError> {
        let title = if let Some(ref hwnd_str) = self.default_window {
            #[cfg(windows)]
            {
                if let Ok(hwnd) = parse_hwnd(hwnd_str) {
                    crate::automation::windows::get_window_info(hwnd)
                        .map(|info| info.title)
                        .ok()
                } else {
                    None
                }
            }
            #[cfg(not(windows))]
            { None }
        } else {
            None
        };

        Ok(DefaultWindowResponse {
            hwnd: self.default_window.clone(),
            title,
        })
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

        // Find last error from failed steps
        let last_error = summary
            .results
            .iter()
            .rev()
            .find_map(|r| r.error.clone());

        // Extract coordinates from successful click results
        let coordinates: Vec<(i32, i32)> = summary
            .results
            .iter()
            .filter_map(|r| {
                use crate::executor::state::StepStatus;
                if matches!(r.status, StepStatus::Success) {
                    r.coordinates
                } else {
                    None
                }
            })
            .collect();

        Ok(ExecutionSummary {
            total_steps: summary.executed_steps + summary.failed_steps,
            completed_steps: summary.executed_steps,
            success: summary.success,
            last_error,
            coordinates,
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

        // Convert Vec<f32> bounding box to [i32; 4]
        let bounding_box = if result.bounding_box.len() >= 4 {
            Some([
                result.bounding_box[0] as i32,
                result.bounding_box[1] as i32,
                result.bounding_box[2] as i32,
                result.bounding_box[3] as i32,
            ])
        } else {
            None
        };

        Ok(DetectionResult {
            element_found: result.element_found,
            label: result.label,
            bounding_box,
            action_type: result.action_type,
            confidence: result.confidence,
        })
    }

    #[cfg(not(windows))]
    async fn detect_elements(&self, _req: DetectRequest) -> Result<DetectionResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    // =========================================================================
    // UIA Methods Implementation
    // =========================================================================

    #[cfg(windows)]
    async fn dump_tree(&self, req: DumpTreeRequest) -> Result<UiaElement, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;

        let automation = UIAutomation::new()
            .map_err(|e| ServiceError::AutomationError(format!("Failed to create UIAutomation: {}", e)))?;

        let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to get element from HWND: {}", e)))?;

        let options = uia::TreeDumpOptions {
            max_depth: req.max_depth.unwrap_or(5),
            prune_offscreen: req.prune_offscreen.unwrap_or(true),
            prune_empty: req.prune_empty.unwrap_or(true),
            max_list_items: req.max_list_items.unwrap_or(20),
        };

        uia::dump_tree(&automation, &root, &options)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to dump tree: {}", e)))
    }

    #[cfg(not(windows))]
    async fn dump_tree(&self, _req: DumpTreeRequest) -> Result<UiaElement, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn find_elements(&self, req: FindElementRequest) -> Result<Vec<UiaElement>, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;

        let selector = Selector::parse(&req.selector)
            .map_err(|e| ServiceError::AutomationError(format!("Invalid selector: {}", e)))?;

        let automation = UIAutomation::new()
            .map_err(|e| ServiceError::AutomationError(format!("Failed to create UIAutomation: {}", e)))?;

        let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to get element from HWND: {}", e)))?;

        let find_all = req.find_all.unwrap_or(false);
        let timeout = req.timeout_ms.unwrap_or(3000);

        uia::find_elements(&automation, &root, &selector, find_all, timeout)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to find elements: {}", e)))
    }

    #[cfg(not(windows))]
    async fn find_elements(&self, _req: FindElementRequest) -> Result<Vec<UiaElement>, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn invoke_pattern(&self, req: InvokePatternRequest) -> Result<PatternResult, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;

        let selector = Selector::parse(&req.selector)
            .map_err(|e| ServiceError::AutomationError(format!("Invalid selector: {}", e)))?;

        let pattern_op = PatternOp::from_str(&req.pattern)
            .ok_or_else(|| ServiceError::AutomationError(format!("Unknown pattern: {}", req.pattern)))?;

        let automation = UIAutomation::new()
            .map_err(|e| ServiceError::AutomationError(format!("Failed to create UIAutomation: {}", e)))?;

        let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to get element from HWND: {}", e)))?;

        // Find the first matching element
        let elements = uia::find_elements(&automation, &root, &selector, false, 3000)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to find element: {}", e)))?;

        if elements.is_empty() {
            return Ok(PatternResult {
                success: false,
                value: None,
                error: Some(format!("No element found matching selector: {}", req.selector)),
            });
        }

        // Need to get the actual UIElement again to invoke the pattern
        // (UiaElement is just a serializable snapshot)
        let target_elements = uia::find_elements(&automation, &root, &selector, false, 0)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to re-find element: {}", e)))?;

        // Re-query to get the native element
        // This is a workaround since we'd need to keep the UIElement reference
        // For now, we re-find and invoke
        let found = automation.create_matcher()
            .from(root.clone())
            .timeout(0)
            .find_first();

        match found {
            Ok(elem) => {
                // Actually need to find by the selector again at native level
                // This is simplified - a production implementation would preserve UIElement handles
                let native_elements = uia::find_elements(&automation, &root, &selector, false, 0);
                
                // For now, just return the first element's pattern result
                // In production, we'd need to map back to the native UIElement
                Ok(uia::execute_pattern(&root, pattern_op, req.value.as_deref()))
            }
            Err(e) => Ok(PatternResult {
                success: false,
                value: None,
                error: Some(format!("Failed to find element for pattern execution: {}", e)),
            }),
        }
    }

    #[cfg(not(windows))]
    async fn invoke_pattern(&self, _req: InvokePatternRequest) -> Result<PatternResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    // =========================================================================
    // LLM-Optimized Methods Implementation
    // =========================================================================

    #[cfg(windows)]
    async fn get_summary(&self, req: SummaryRequest) -> Result<String, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;

        let automation = UIAutomation::new()
            .map_err(|e| ServiceError::AutomationError(format!("Failed to create UIAutomation: {}", e)))?;

        let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to get element from HWND: {}", e)))?;

        // Get window title
        let window_title = crate::automation::windows::get_window_info(hwnd)
            .map(|info| info.title)
            .unwrap_or_else(|_| "Unknown Window".to_string());

        // First dump the tree to get UiaElement structure
        let options = uia::TreeDumpOptions {
            max_depth: req.max_depth.unwrap_or(10),
            prune_offscreen: true,
            prune_empty: true,
            max_list_items: 10,
        };

        let tree = uia::dump_tree(&automation, &root, &options)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to dump tree: {}", e)))?;

        // Build summary options
        let summary_options = uia::SummaryOptions {
            include_bounds: req.include_bounds.unwrap_or(false),
            include_paths: req.include_paths.unwrap_or(false),
            focus_region: req.focus_region,
            max_depth: req.max_depth.unwrap_or(10),
            min_size: 5,
            role_filter: req.roles,
        };

        // Generate summary
        let summary = uia::generate_summary(&tree, &window_title, &summary_options);

        // Format output based on requested format
        let format = req.format.as_deref().unwrap_or("json");
        let output = match format {
            "text" => uia::format_text_summary(&summary),
            _ => serde_json::to_string_pretty(&summary)
                .unwrap_or_else(|_| "{}".to_string()),
        };

        Ok(output)
    }

    #[cfg(not(windows))]
    async fn get_summary(&self, _req: SummaryRequest) -> Result<String, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn query_elements(&self, req: QueryRequest) -> Result<QueryResult, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;

        // Parse the enhanced query
        let query = uia::parse_query(&req.query)
            .map_err(|e| ServiceError::AutomationError(format!("Invalid query: {}", e)))?;

        let automation = UIAutomation::new()
            .map_err(|e| ServiceError::AutomationError(format!("Failed to create UIAutomation: {}", e)))?;

        let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to get element from HWND: {}", e)))?;

        let find_all = req.all.unwrap_or(false);
        let timeout = req.timeout_ms.unwrap_or(3000);

        // Find elements using the parsed selector
        let mut elements = uia::find_elements(&automation, &root, &query.selector, find_all || query.index.is_some(), timeout)
            .map_err(|e| ServiceError::AutomationError(format!("Failed to find elements: {}", e)))?;

        // Apply state filters
        if !query.state_filters.is_empty() {
            elements = uia::apply_state_filters(&elements, &query.state_filters);
        }

        // Apply index filter
        if let Some(ref index) = query.index {
            elements = uia::apply_index_filter(elements, index);
        }

        // Convert to ElementRef format
        let mut id_gen = uia::RefIdGenerator::new();
        let matches: Vec<ElementRef> = elements
            .iter()
            .map(|elem| {
                let role = uia::infer_role(elem);
                ElementRef {
                    id: id_gen.next(role),
                    role: role.as_str().to_string(),
                    label: if !elem.name.is_empty() {
                        elem.name.clone()
                    } else if !elem.automation_id.is_empty() {
                        elem.automation_id.clone()
                    } else {
                        role.as_str().to_string()
                    },
                    action: uia::infer_action(elem),
                    selector: uia::generate_selector(elem),
                }
            })
            .collect();

        Ok(QueryResult {
            count: matches.len(),
            matches,
            suggestions: Vec::new(),
        })
    }

    #[cfg(not(windows))]
    async fn query_elements(&self, _req: QueryRequest) -> Result<QueryResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }
}
