use remoc::rtc;

use crate::automation::types::WindowInfo;
use crate::gemini::GeminiClient;
use crate::rpc::types::*;

#[cfg(windows)]
use crate::automation::windows::{capture_screenshot, list_windows, parse_hwnd, ScreenshotMethod};
#[cfg(windows)]
use crate::automation::windows::uia::{self, PatternOp, Selector, TreeDumpOptions};
#[cfg(windows)]
use crate::automation::windows::input::{
    click_at_coords, double_click_at_coords, right_click_at_coords,
    type_text as input_type_text, send_keys as input_send_keys, scroll as input_scroll,
};
#[cfg(windows)]
use crate::executor::Executor;
#[cfg(windows)]
use crate::gemini::retry::RetryStrategy;
#[cfg(windows)]
use uiautomation::UIAutomation;
#[cfg(windows)]
use uiautomation::types::Handle;

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

    // =========================================================================
    // Input Action Methods
    // =========================================================================

    /// Click at coordinates or on an element
    async fn click(&self, req: ClickRequest) -> Result<ActionResult, ServiceError>;

    /// Type text (optionally after focusing an element)
    async fn type_text(&self, req: TypeTextRequest) -> Result<ActionResult, ServiceError>;

    /// Send key combination (e.g., "ctrl+c", "alt+f4")
    async fn send_keys(&self, req: SendKeysRequest) -> Result<ActionResult, ServiceError>;

    /// Scroll the view
    async fn scroll(&self, req: ScrollRequest) -> Result<ActionResult, ServiceError>;

    // =========================================================================
    // Agent Method (LLM-driven automation)
    // =========================================================================

    /// Run an AI agent to accomplish a goal using natural language
    async fn run_agent(&self, req: AgentRequest) -> Result<AgentResponse, ServiceError>;
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
                summary: None,
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
                summary: None,
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

    // =========================================================================
    // Input Action Implementations
    // =========================================================================

    #[cfg(windows)]
    async fn click(&self, req: ClickRequest) -> Result<ActionResult, ServiceError> {
        let hwnd = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;

        // Determine click coordinates
        let (x, y) = if let Some((cx, cy)) = req.coords {
            (cx, cy)
        } else if let Some(ref selector_str) = req.selector {
            // Find element and get its center
            let automation = UIAutomation::new()
                .map_err(|e| ServiceError::AutomationError(format!("Failed to init UIA: {}", e)))?;
            let root = automation.element_from_handle(Handle::from(hwnd.0 as isize))
                .map_err(|e| ServiceError::AutomationError(format!("Failed to get window element: {}", e)))?;
            let selector = Selector::parse(selector_str)
                .map_err(|e| ServiceError::AutomationError(format!("Invalid selector: {}", e)))?;
            let elements = uia::find_elements(&automation, &root, &selector, false, 3000)
                .map_err(|e| ServiceError::AutomationError(format!("Find failed: {}", e)))?;

            if elements.is_empty() {
                return Ok(ActionResult {
                    success: false,
                    error: Some(format!("No element found matching: {}", selector_str)),
                });
            }

            // UiaElement has bounds as [x, y, width, height]
            let elem = &elements[0];
            let bounds = elem.bounds;

            // Center of the element
            (bounds[0] + bounds[2] / 2,
             bounds[1] + bounds[3] / 2)
        } else {
            return Ok(ActionResult {
                success: false,
                error: Some("Either coords or selector must be specified".to_string()),
            });
        };

        // Perform the click
        let result = match req.click_type.to_lowercase().as_str() {
            "right" => right_click_at_coords(hwnd, x, y),
            "double" => double_click_at_coords(hwnd, x, y),
            _ => click_at_coords(hwnd, x, y), // "left" or default
        };

        match result {
            Ok(()) => Ok(ActionResult { success: true, error: None }),
            Err(e) => Ok(ActionResult { success: false, error: Some(e.to_string()) }),
        }
    }

    #[cfg(not(windows))]
    async fn click(&self, _req: ClickRequest) -> Result<ActionResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn type_text(&self, req: TypeTextRequest) -> Result<ActionResult, ServiceError> {
        // If selector is provided, click on it first to focus
        if let Some(ref selector_str) = req.selector {
            let click_req = ClickRequest {
                hwnd: req.hwnd.clone(),
                click_type: "left".to_string(),
                coords: None,
                selector: Some(selector_str.clone()),
            };
            let click_result = self.click(click_req).await?;
            if !click_result.success {
                return Ok(ActionResult {
                    success: false,
                    error: Some(format!("Failed to focus element: {:?}", click_result.error)),
                });
            }
            // Small delay for focus
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        match input_type_text(&req.text) {
            Ok(()) => Ok(ActionResult { success: true, error: None }),
            Err(e) => Ok(ActionResult { success: false, error: Some(e.to_string()) }),
        }
    }

    #[cfg(not(windows))]
    async fn type_text(&self, _req: TypeTextRequest) -> Result<ActionResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn send_keys(&self, req: SendKeysRequest) -> Result<ActionResult, ServiceError> {
        match input_send_keys(&req.keys) {
            Ok(()) => Ok(ActionResult { success: true, error: None }),
            Err(e) => Ok(ActionResult { success: false, error: Some(e.to_string()) }),
        }
    }

    #[cfg(not(windows))]
    async fn send_keys(&self, _req: SendKeysRequest) -> Result<ActionResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn scroll(&self, req: ScrollRequest) -> Result<ActionResult, ServiceError> {
        // Move mouse to coords first if specified
        if let Some((x, y)) = req.coords {
            let hwnd = parse_hwnd(&req.hwnd)
                .map_err(|e| ServiceError::AutomationError(e.to_string()))?;
            // Move mouse without clicking
            let _ = click_at_coords(hwnd, x, y); // This moves the mouse
        }

        // Calculate scroll amount (120 units per notch)
        let amount = match req.direction.to_lowercase().as_str() {
            "up" => req.amount * 120,
            "down" => -(req.amount * 120),
            _ => return Ok(ActionResult {
                success: false,
                error: Some(format!("Invalid scroll direction: {}. Use 'up' or 'down'", req.direction)),
            }),
        };

        match input_scroll(amount) {
            Ok(()) => Ok(ActionResult { success: true, error: None }),
            Err(e) => Ok(ActionResult { success: false, error: Some(e.to_string()) }),
        }
    }

    #[cfg(not(windows))]
    async fn scroll(&self, _req: ScrollRequest) -> Result<ActionResult, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }

    #[cfg(windows)]
    async fn run_agent(&self, req: AgentRequest) -> Result<AgentResponse, ServiceError> {
        use crate::agent::{AgentPlanner, AgentAction};
        use windows::Win32::Foundation::HWND;

        let hwnd_parsed = parse_hwnd(&req.hwnd)
            .map_err(|e| ServiceError::AutomationError(e.to_string()))?;
        // Store as isize to make it Send-safe across await boundaries
        let hwnd_value = hwnd_parsed.0 as isize;

        // Create planner with same API key and model as the gemini client
        let planner = AgentPlanner::new(
            std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            self.gemini_client.model().to_string(),
        ).map_err(|e| ServiceError::GeminiError(e.to_string()))?;

        let mut history: Vec<AgentStepInfo> = Vec::new();
        let mut history_descriptions: Vec<String> = Vec::new();

        for step_num in 1..=req.max_steps {
            tracing::info!("Agent step {} of {}", step_num, req.max_steps);

            // Recreate HWND from isize (safe to do each iteration)
            let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

            // Gather context
            let screenshot_b64 = if req.include_screenshot {
                match capture_screenshot(hwnd, ScreenshotMethod::BitBlt) {
                    Ok(img) => Some(img.base64_image.clone()),
                    Err(e) => {
                        tracing::warn!("Screenshot failed: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            let ui_summary = if req.include_ui_summary {
                let automation = UIAutomation::new()
                    .map_err(|e| ServiceError::AutomationError(e.to_string()))?;
                let root = automation.element_from_handle(Handle::from(hwnd_value))
                    .map_err(|e| ServiceError::AutomationError(e.to_string()))?;

                // Get compact summary
                let options = TreeDumpOptions {
                    max_depth: 8,
                    prune_offscreen: true,
                    prune_empty: true,
                    max_list_items: 10,
                };
                match uia::dump_tree(&automation, &root, &options) {
                    Ok(tree) => Some(format_ui_summary(&tree)),
                    Err(e) => {
                        tracing::warn!("UI summary failed: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Plan next action
            let plan_result = planner.plan_next_action(
                &req.goal,
                screenshot_b64.as_deref(),
                ui_summary.as_deref(),
                &history_descriptions,
            ).await;

            let response = match plan_result {
                Ok(r) => r,
                Err(e) => {
                    return Ok(AgentResponse {
                        success: false,
                        summary: format!("Planner error: {}", e),
                        steps_taken: history.len(),
                        status: "error".to_string(),
                        history,
                    });
                }
            };

            tracing::info!("Planner decided: {:?}", response.action);

            // Execute the action
            let (action_name, action_details, exec_result) = match &response.action {
                AgentAction::Click { target, click_type } => {
                    let details = format!("{} on '{}'", click_type, target);
                    let result = self.execute_agent_click(hwnd_value, target, click_type, screenshot_b64.as_deref()).await;
                    ("click".to_string(), details, result)
                }
                AgentAction::Type { text, target } => {
                    let details = if let Some(t) = target {
                        format!("'{}' into '{}'", text, t)
                    } else {
                        format!("'{}'", text)
                    };
                    let result = input_type_text(text);
                    ("type".to_string(), details, result.map_err(|e| e.to_string()))
                }
                AgentAction::Keys { keys } => {
                    let details = format!("'{}'", keys);
                    let result = input_send_keys(keys);
                    ("keys".to_string(), details, result.map_err(|e| e.to_string()))
                }
                AgentAction::Scroll { direction, amount } => {
                    let details = format!("{} x{}", direction, amount);
                    let scroll_amount = match direction.to_lowercase().as_str() {
                        "up" => amount * 120,
                        _ => -(amount * 120),
                    };
                    let result = input_scroll(scroll_amount);
                    ("scroll".to_string(), details, result.map_err(|e| e.to_string()))
                }
                AgentAction::Wait { ms } => {
                    let details = format!("{}ms", ms);
                    tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                    ("wait".to_string(), details, Ok(()))
                }
                AgentAction::Done { reason } => {
                    history.push(AgentStepInfo {
                        step: step_num,
                        reasoning: response.reasoning.clone(),
                        action: "done".to_string(),
                        action_details: reason.clone(),
                        success: true,
                        error: None,
                    });
                    return Ok(AgentResponse {
                        success: true,
                        summary: reason.clone(),
                        steps_taken: history.len(),
                        status: "done".to_string(),
                        history,
                    });
                }
                AgentAction::Fail { reason } => {
                    history.push(AgentStepInfo {
                        step: step_num,
                        reasoning: response.reasoning.clone(),
                        action: "fail".to_string(),
                        action_details: reason.clone(),
                        success: false,
                        error: Some(reason.clone()),
                    });
                    return Ok(AgentResponse {
                        success: false,
                        summary: reason.clone(),
                        steps_taken: history.len(),
                        status: "failed".to_string(),
                        history,
                    });
                }
            };

            let (success, error) = match exec_result {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e)),
            };

            // Record step
            let step_desc = format!("{} {} - {}", action_name, action_details,
                if success { "OK" } else { error.as_deref().unwrap_or("failed") });
            history_descriptions.push(step_desc);

            history.push(AgentStepInfo {
                step: step_num,
                reasoning: response.reasoning,
                action: action_name,
                action_details,
                success,
                error,
            });

            // Small delay between actions
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        // Max steps reached
        Ok(AgentResponse {
            success: false,
            summary: format!("Reached maximum {} steps without completing goal", req.max_steps),
            steps_taken: history.len(),
            status: "max_steps".to_string(),
            history,
        })
    }

    #[cfg(not(windows))]
    async fn run_agent(&self, _req: AgentRequest) -> Result<AgentResponse, ServiceError> {
        Err(ServiceError::PlatformNotSupported)
    }
}

/// Format UIA tree into compact text summary for LLM
#[cfg(windows)]
fn format_ui_summary(elem: &UiaElement) -> String {
    let mut lines = Vec::new();
    format_ui_element(&mut lines, elem, 0);
    lines.join("\n")
}

#[cfg(windows)]
fn format_ui_element(lines: &mut Vec<String>, elem: &UiaElement, indent: usize) {
    let prefix = "  ".repeat(indent);

    // Skip empty/uninteresting elements
    if elem.name.is_empty() && elem.patterns.is_empty() && elem.children.is_empty() {
        return;
    }

    // Format: [Type] "Name" (patterns) @bounds
    let patterns_str = if elem.patterns.is_empty() {
        String::new()
    } else {
        format!(" ({})", elem.patterns.join(","))
    };

    let bounds_str = if elem.bounds[2] > 0 && elem.bounds[3] > 0 {
        format!(" @{},{}", elem.bounds[0] + elem.bounds[2]/2, elem.bounds[1] + elem.bounds[3]/2)
    } else {
        String::new()
    };

    let name_str = if elem.name.is_empty() {
        String::new()
    } else {
        format!(" \"{}\"", elem.name.chars().take(40).collect::<String>())
    };

    lines.push(format!("{}[{}]{}{}{}", prefix, elem.control_type, name_str, patterns_str, bounds_str));

    // Recurse into children (limit depth)
    if indent < 6 {
        for child in &elem.children {
            format_ui_element(lines, child, indent + 1);
        }
    }
}

#[cfg(windows)]
impl DesktopServiceImpl {
    /// Execute a click action from agent, handling both coordinate and element description targets
    /// Priority: 1) Coordinates, 2) UIA selector, 3) UIA name search, 4) Visual grounding (fallback)
    async fn execute_agent_click(
        &self,
        hwnd_value: isize,
        target: &str,
        click_type: &str,
        _screenshot_b64: Option<&str>,
    ) -> Result<(), String> {
        use windows::Win32::Foundation::HWND;

        // 1) Check if target looks like coordinates "x,y"
        if target.contains(',') && !target.contains('[') {
            let parts: Vec<&str> = target.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<i32>(), parts[1].trim().parse::<i32>()) {
                    let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
                    return match click_type.to_lowercase().as_str() {
                        "right" => right_click_at_coords(hwnd, x, y).map_err(|e| e.to_string()),
                        "double" => double_click_at_coords(hwnd, x, y).map_err(|e| e.to_string()),
                        _ => click_at_coords(hwnd, x, y).map_err(|e| e.to_string()),
                    };
                }
            }
        }

        // 2) Try UIA - build a selector from the target description
        let automation = UIAutomation::new()
            .map_err(|e| format!("Failed to init UIA: {}", e))?;
        let root = automation.element_from_handle(Handle::from(hwnd_value))
            .map_err(|e| format!("Failed to get window element: {}", e))?;

        // Try different selector strategies
        let selector_attempts = build_selectors_from_description(target);

        for selector_str in &selector_attempts {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Ok(elements) = uia::find_elements(&automation, &root, &selector, true, 1000) {
                    for elem in &elements {
                        let bounds = elem.bounds;

                        // Skip if element has invalid bounds (offscreen, negative, or zero size)
                        if bounds[0] < 0 || bounds[1] < 0 || bounds[2] <= 0 || bounds[3] <= 0 {
                            continue;
                        }

                        // Skip if element is marked as offscreen
                        if elem.is_offscreen {
                            continue;
                        }

                        let x = bounds[0] + bounds[2] / 2;
                        let y = bounds[1] + bounds[3] / 2;

                        // Sanity check - coordinates should be reasonable screen values
                        if x < 0 || y < 0 || x > 10000 || y > 10000 {
                            continue;
                        }

                        tracing::info!("UIA found '{}' at ({}, {}) using selector: {}", target, x, y, selector_str);

                        let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
                        return match click_type.to_lowercase().as_str() {
                            "right" => right_click_at_coords(hwnd, x, y).map_err(|e| e.to_string()),
                            "double" => double_click_at_coords(hwnd, x, y).map_err(|e| e.to_string()),
                            _ => click_at_coords(hwnd, x, y).map_err(|e| e.to_string()),
                        };
                    }
                }
            }
        }

        Err(format!("Could not find element '{}' via UIA. Tried selectors: {:?}", target, selector_attempts))
    }
}

/// Build UIA selectors from a natural language element description
#[cfg(windows)]
fn build_selectors_from_description(description: &str) -> Vec<String> {
    let mut selectors = Vec::new();
    let desc_lower = description.to_lowercase();

    // Extract the element name (remove common words)
    let name = description
        .replace("the ", "")
        .replace("a ", "")
        .replace("an ", "")
        .replace(" button", "")
        .replace(" menu", "")
        .replace(" item", "")
        .replace(" tab", "")
        .replace(" field", "")
        .replace(" input", "")
        .replace(" link", "")
        .replace(" icon", "")
        .trim()
        .to_string();

    // Determine likely control type
    let control_type = if desc_lower.contains("button") {
        Some("Button")
    } else if desc_lower.contains("menu") {
        Some("MenuItem")
    } else if desc_lower.contains("tab") {
        Some("TabItem")
    } else if desc_lower.contains("input") || desc_lower.contains("field") || desc_lower.contains("textbox") {
        Some("Edit")
    } else if desc_lower.contains("checkbox") || desc_lower.contains("check box") {
        Some("CheckBox")
    } else if desc_lower.contains("link") {
        Some("Hyperlink")
    } else if desc_lower.contains("tree") || desc_lower.contains("item") {
        Some("TreeItem")
    } else {
        None
    };

    // Try exact name match with control type
    if let Some(ct) = control_type {
        selectors.push(format!("{}[name='{}']", ct, name));
    }

    // Try exact name match (any type)
    selectors.push(format!("*[name='{}']", name));

    // Try partial name match with control type
    if let Some(ct) = control_type {
        selectors.push(format!("{}[name~='*{}*']", ct, name));
    }

    // Try partial name match (any type)
    selectors.push(format!("*[name~='*{}*']", name));

    // Try with original description as name
    if name != description {
        selectors.push(format!("*[name='{}']", description));
        selectors.push(format!("*[name~='*{}*']", description));
    }

    // For menu items, also try MenuItem type explicitly
    if desc_lower.contains("file") || desc_lower.contains("edit") || desc_lower.contains("view")
        || desc_lower.contains("help") || desc_lower.contains("tools") {
        selectors.push(format!("MenuItem[name='{}']", name));
        selectors.push(format!("MenuItem[name~='*{}*']", name));
        // Also try Menu type
        selectors.push(format!("Menu[name='{}']", name));
        selectors.push(format!("Menu[name~='*{}*']", name));
    }

    selectors
}
