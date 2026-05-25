# Multi-Platform Support Implementation Plan

## Overview

Desktop-cli currently provides full Windows automation via UI Automation (UIA), SendInput, and win-screenshot. Linux and macOS return "Platform not supported" from stub implementations. This plan adds cross-platform support using a trait-based abstraction layer.

**Chosen Approach**: Trait-based dispatch (Approach B) with compile-time platform selection. Each platform implements `DesktopPlatform` trait. Linux uses AT-SPI2 via `atspi` crate (X11 only initially). macOS uses Cocoa Accessibility via `accessibility-sys`. Cross-platform input via `enigo`, screenshots via `xcap`.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|-----------------|
| Trait-based over cfg-swap | Current cfg-swap duplicates interfaces without abstraction -> trait enables mock testing and shared logic -> compile-time dispatch preserves zero-cost -> cleaner than runtime enum matching |
| X11 only for Linux initial | Wayland requires libei which has partial enigo support -> X11 covers majority of desktop Linux users -> defer Wayland to future milestone reduces scope -> AT-SPI2 works identically on both |
| Graceful permission degradation (macOS) | macOS requires explicit accessibility grant -> failing fast would block all operations -> informative error guides user to System Preferences -> app remains usable for non-accessibility operations |
| `atspi` crate for Linux | Pure Rust, async D-Bus client -> maintained by Odilia project -> maps well to UIA element tree model -> zbus-based is modern approach |
| `enigo` for input simulation | Cross-platform (X11, macOS, Windows) -> single API for all platforms -> handles keyboard layouts -> active maintenance |
| `xcap` for screenshots | Supports X11, Wayland, macOS, Windows -> returns raw pixels for encoding -> simpler than platform-specific solutions |
| Property-based unit tests | Selector parsing has wide input space -> fewer tests cover more cases -> quickcheck/proptest natural fit |
| Real deps for integration | Accessibility APIs require real OS interaction -> mocking would miss platform-specific edge cases -> CI runners per platform |
| String-based window handles | HWND is Windows-specific -> existing `hwnd: String` field already abstracts -> Linux uses X11 window ID, macOS uses PID+element ref |
| 500ms element search timeout | Plan specifies 500ms for fast feedback on missing elements -> all platform implementations use 500ms |
| Per-call tokio runtime creation (Linux AT-SPI2) | Prevents runtime state conflicts when used as library -> Performance cost (O(n) runtime creation) accepted for v1 simplicity -> Can optimize to shared runtime in v2 if profiling shows impact |

#### Role Mapping Tables

**AT-SPI2 to UIA (Linux)**:

| AT-SPI2 Role | UIA Control Type | Notes |
|--------------|------------------|-------|
| push button | Button | Standard button control |
| text | Edit | Single/multi-line text input |
| menu | Menu | Menu bar or context menu |
| menu item | MenuItem | Individual menu item |
| check box | CheckBox | Toggle control with checked state |
| radio button | RadioButton | Mutually exclusive selection |
| combo box | ComboBox | Dropdown list with selection |
| list | List | List container |
| list item | ListItem | Item within a list |
| window | Window | Top-level window |
| frame | Pane | Container or frame |
| panel | Pane | Generic container |
| scroll bar | ScrollBar | Scrolling control |
| table | Table | Grid or table |
| table cell | DataItem | Cell within table |
| label | Text | Static text label |

**macOS AX to UIA**:

| macOS AX Role | UIA Control Type | Notes |
|---------------|------------------|-------|
| AXButton | Button | Standard button |
| AXTextField | Edit | Text input field |
| AXStaticText | Text | Non-editable text |
| AXMenu | Menu | Menu control |
| AXMenuItem | MenuItem | Menu item |
| AXCheckBox | CheckBox | Checkbox control |
| AXRadioButton | RadioButton | Radio button |
| AXComboBox | ComboBox | Dropdown combo box |
| AXList | List | List control |
| AXRow | ListItem | List/table row |
| AXWindow | Window | Top-level window |
| AXGroup | Pane | Container group |
| AXScrollBar | ScrollBar | Scrollbar |
| AXTable | Table | Table/grid |
| AXCell | DataItem | Table cell |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|--------------|
| Minimal cfg-swap (Approach A) | No shared abstraction for testing; code duplication across platforms; harder to maintain feature parity |
| Runtime enum dispatch (Approach C) | Binary includes all platform code; runtime overhead on every call; fights Rust's compile-time strength |
| X11 + Wayland simultaneously | Wayland input simulation via libei is experimental in enigo; doubles testing matrix; X11 covers 90%+ of current desktop Linux |
| Fail-fast macOS permissions | Would prevent any app usage until permissions granted; user experience worse than informative error |
| `accessibility-ng` for macOS | Less maintained than accessibility-sys; fewer examples; accessibility-sys has better documentation |

### Constraints & Assumptions

- **Rust edition**: 2021 (per Cargo.toml)
- **Async runtime**: tokio (already a dependency; required for atspi D-Bus)
- **Minimum Linux**: X11 with AT-SPI2 service running (standard on GNOME/KDE)
- **Minimum macOS**: 10.15+ (Accessibility framework stable)
- **Testing**: Property-based for unit, real deps for integration (user-specified)
- **Default conventions applied**: `<default-conventions domain="testing">` for test placement with milestones

### Known Risks

| Risk | Mitigation | Anchor |
|------|------------|--------|
| AT-SPI2 service not running | Detect via D-Bus, return informative error with activation hint | N/A (new code) |
| macOS permission dialog blocks automation | Document in setup guide; detect permission status before operations | N/A (new code) |
| Element tree shape differs across platforms | Role mapping layer normalizes to UIA-style types | N/A (new code) |
| enigo Wayland support incomplete | Scope limited to X11; Wayland deferred to future milestone | N/A (design decision) |
| Test flakiness with real accessibility APIs | Retry logic in tests; skip on permission issues | N/A (new code) |

## Invisible Knowledge

### Architecture

```
                    ┌─────────────────────────────────────────────────────────┐
                    │                      CLI (main.rs)                       │
                    └─────────────────────────────────────────────────────────┘
                                              │
                                              ▼
                    ┌─────────────────────────────────────────────────────────┐
                    │                   ops/mod.rs                             │
                    │         (compile-time platform dispatch)                 │
                    └─────────────────────────────────────────────────────────┘
                                              │
              ┌───────────────────────────────┼───────────────────────────────┐
              │                               │                               │
              ▼                               ▼                               ▼
┌─────────────────────────┐   ┌─────────────────────────┐   ┌─────────────────────────┐
│    windows_ops.rs       │   │     linux_ops.rs        │   │     macos_ops.rs        │
│  impl DesktopPlatform   │   │  impl DesktopPlatform   │   │  impl DesktopPlatform   │
└─────────────────────────┘   └─────────────────────────┘   └─────────────────────────┘
         │                              │                              │
         ▼                              ▼                              ▼
┌─────────────────────────┐   ┌─────────────────────────┐   ┌─────────────────────────┐
│  automation/windows/    │   │  automation/linux/      │   │  automation/macos/      │
│  - UIA (uiautomation)   │   │  - AT-SPI2 (atspi)      │   │  - Cocoa (accessibility)│
│  - SendInput            │   │  - X11 (x11rb)          │   │  - enigo                │
│  - win-screenshot       │   │  - enigo, xcap          │   │  - xcap                 │
└─────────────────────────┘   └─────────────────────────┘   └─────────────────────────┘
```

### Data Flow

```
User Command
     │
     ▼
Parse window query (targeting/parser.rs)
     │
     ▼
Resolve to platform handle (ops::list_windows → filter)
     │
     ▼
Platform-specific operation:
  ├── Element tree: Accessibility API → UiaElement
  ├── Input: enigo (or SendInput on Windows)
  └── Screenshot: xcap (or win-screenshot on Windows)
     │
     ▼
Serialize result (rpc/types.rs - cross-platform)
     │
     ▼
Output to user
```

### Why This Structure

- **ops/ as dispatch layer**: Single entry point for all platform operations; test surface for mocking
- **automation/{platform}/ separation**: Platform-specific code isolated; no cross-contamination
- **Shared types in automation/types.rs and rpc/types.rs**: Already platform-agnostic; no duplication needed
- **Trait in ops, not automation**: ops defines the contract; automation modules implement platform details

### Invariants

1. **Window handle opacity**: All code outside platform modules treats `hwnd: String` as opaque; parsing only in platform code
2. **Element tree normalization**: All platforms return `UiaElement` with consistent role names (via mapping)
3. **Coordinate system**: All coordinates are pixels relative to window origin; DPI handling internal to platform
4. **Error propagation**: Platform errors wrap in `OpsError`; no platform-specific error types leak to CLI

### Tradeoffs

| Choice | Benefit | Cost |
|--------|---------|------|
| Compile-time dispatch | Zero runtime overhead; type-safe | No runtime platform switching |
| Single trait `DesktopPlatform` | Simple interface | May need extension for platform-specific features |
| X11 only initially | Faster delivery | Wayland users must wait |
| String window handles | Simple abstraction | Parsing overhead on each operation |

## Milestones

### Milestone 1: Fix Existing Test Bugs

**Files**:
- `tests/uia_integration_test.rs`
- `src/targeting/resolver.rs`
- `src/targeting/suggest.rs`

**Flags**: `conformance`

**Requirements**:
- Add `#[cfg(windows)]` guards to test file imports
- Add missing `rect` field to all WindowInfo test mocks
- Remove dead code warnings (unused imports)

**Acceptance Criteria**:
- `cargo test` compiles on Linux without errors
- `cargo test` compiles on Windows without errors
- `cargo clippy` shows no warnings in modified files

**Tests**:
- **Test files**: Existing test files being fixed
- **Test type**: N/A (fixing compilation)
- **Backing**: N/A
- **Scenarios**: Compilation succeeds on all platforms

**Code Intent**:
- `tests/uia_integration_test.rs`: Wrap lines 1-4 imports in `#[cfg(windows)]`
- `src/targeting/resolver.rs` lines 322-343: Add `rect: WindowRect { x: 0, y: 0, width: 800, height: 600 }` to each WindowInfo in `make_windows()`
- `src/targeting/suggest.rs` lines 260-284: Same WindowRect addition to test mocks
- `src/executor/engine.rs`: Remove or cfg-gate unused imports on lines 3-5

**Code Changes**:

```diff
--- a/tests/uia_integration_test.rs
+++ b/tests/uia_integration_test.rs
@@ -1,4 +1,8 @@
+#[cfg(windows)]
 use desktop_cli::automation::windows::uia::tree::{dump_tree, element_from_hwnd, element_to_uia};
+#[cfg(windows)]
 use desktop_cli::automation::windows::window::list_windows;
+#[cfg(windows)]
 use desktop_cli::rpc::types::TreeDumpOptions;
+#[cfg(windows)]
 use uiautomation::UIAutomation;

 #[test]
```

```diff
--- a/src/targeting/resolver.rs
+++ b/src/targeting/resolver.rs
@@ -320,24 +320,27 @@
     fn make_windows() -> Vec<WindowInfo> {
         vec![
             WindowInfo {
                 hwnd: "0x1234".to_string(),
                 title: "Altium Designer - PCB1.PcbDoc".to_string(),
                 executable: "Altium.exe".to_string(),
+                rect: WindowRect { x: 0, y: 0, width: 800, height: 600 },
                 pid: 1000,
                 class_name: Some("TfrmAltium".to_string()),
             },
             WindowInfo {
                 hwnd: "0x5678".to_string(),
                 title: "Altium Designer - Schematic1.SchDoc".to_string(),
                 executable: "Altium.exe".to_string(),
+                rect: WindowRect { x: 0, y: 0, width: 800, height: 600 },
                 pid: 1000,
                 class_name: Some("TfrmAltium".to_string()),
             },
             WindowInfo {
                 hwnd: "0x9ABC".to_string(),
                 title: "Untitled - Notepad".to_string(),
                 executable: "notepad.exe".to_string(),
+                rect: WindowRect { x: 0, y: 0, width: 800, height: 600 },
                 pid: 2000,
                 class_name: Some("Notepad".to_string()),
             },
```

```diff
--- a/src/targeting/suggest.rs
+++ b/src/targeting/suggest.rs
@@ -260,24 +260,27 @@
     fn make_windows() -> Vec<WindowInfo> {
         vec![
             WindowInfo {
                 hwnd: "0x1234".to_string(),
                 title: "Altium Designer - PCB1.PcbDoc".to_string(),
                 executable: "C:\\Program Files\\Altium\\Altium.exe".to_string(),
+                rect: WindowRect { x: 0, y: 0, width: 800, height: 600 },
                 pid: 1000,
                 class_name: Some("TfrmAltium".to_string()),
             },
             WindowInfo {
                 hwnd: "0x5678".to_string(),
                 title: "Altium Designer - Schematic1.SchDoc".to_string(),
                 executable: "C:\\Program Files\\Altium\\Altium.exe".to_string(),
+                rect: WindowRect { x: 0, y: 0, width: 800, height: 600 },
                 pid: 1000,
                 class_name: Some("TfrmAltium".to_string()),
             },
             WindowInfo {
                 hwnd: "0x9ABC".to_string(),
                 title: "Untitled - Notepad".to_string(),
                 executable: "C:\\Windows\\notepad.exe".to_string(),
+                rect: WindowRect { x: 0, y: 0, width: 800, height: 600 },
                 pid: 2000,
                 class_name: Some("Notepad".to_string()),
             },
```

```diff
--- a/src/executor/engine.rs
+++ b/src/executor/engine.rs
@@ -1,11 +1,14 @@
 #[cfg(windows)]
 use crate::automation::windows::{capture_screenshot, click_at_coords, parse_hwnd, type_text, ScreenshotMethod};
 use crate::error::{DesktopCliError, Result};
 use crate::executor::parser::{is_dangerous_instruction, validate_instructions};
 use crate::executor::state::{ExecutionState, ExecutionSummary};
 use crate::gemini::client::GeminiClient;
 #[cfg(windows)]
 use crate::gemini::bounding_box::{convert_to_pixels, NormalizedBoundingBox};
 #[cfg(windows)]
+use crate::gemini::retry::{detect_element_with_retry, RetryStrategy};
+#[cfg(windows)]
+use windows::Win32::Foundation::HWND;
+
```

---

### Milestone 2: Define Platform Abstraction Trait

**Files**:
- `src/ops/traits.rs` (new)
- `src/ops/mod.rs`

**Flags**: `needs-rationale`

**Requirements**:
- Define `DesktopPlatform` trait with all 13 operations from current ops API (list_windows, get_window_by_hwnd, take_screenshot, dump_tree, find_elements, element_exists, invoke_pattern, get_summary, query_elements, click, type_text, send_keys, scroll)
- Define `PlatformError` trait for platform-specific errors
- Update `ops/mod.rs` to reference trait (no implementation change yet)

**Acceptance Criteria**:
- Trait compiles and is exported from `ops` module
- Trait methods match current `stub_ops.rs` signatures
- Documentation comments explain each method's contract

**Tests**:
- **Test files**: `src/ops/traits.rs` (doc tests)
- **Test type**: unit (doc tests)
- **Backing**: default-derived
- **Scenarios**: Doc examples compile

**Code Intent**:
- New `src/ops/traits.rs`:
  - `DesktopPlatform` trait with methods: `list_windows`, `get_window_by_hwnd`, `take_screenshot`, `dump_tree`, `find_elements`, `element_exists`, `invoke_pattern`, `get_summary`, `query_elements`, `click`, `type_text`, `send_keys`, `scroll`
  - Each method returns `Result<T, OpsError>` matching current API
  - Use `&self` for stateless operations (all current ops are stateless)
- Modify `src/ops/mod.rs`:
  - Add `pub mod traits;`
  - Re-export trait: `pub use traits::DesktopPlatform;`

**Code Changes**:

```diff
--- /dev/null
+++ b/src/ops/traits.rs
@@ -0,0 +1,100 @@
+//! Platform abstraction traits for desktop automation
+//!
+//! Trait-based dispatch enables compile-time platform selection while providing
+//! a uniform API for testing and shared logic across Windows, Linux, and macOS.
+
+use crate::automation::types::WindowInfo;
+use crate::rpc::types::{PatternResult, QueryResult, Screenshot, UiaElement};
+
+use super::{OpsError, Result};
+
+/// Cross-platform desktop automation operations
+///
+/// Each platform (Windows, Linux, macOS) implements this trait with platform-specific
+/// automation APIs. The trait provides a uniform interface while preserving zero-cost
+/// compile-time dispatch.
+pub trait DesktopPlatform {
+    /// List all visible windows with optional filters
+    ///
+    /// # Arguments
+    /// * `exe_filter` - Filter windows by executable name (case-insensitive substring match)
+    /// * `title_filter` - Filter windows by title (case-insensitive substring match)
+    fn list_windows(
+        &self,
+        exe_filter: Option<&str>,
+        title_filter: Option<&str>,
+    ) -> Result<Vec<WindowInfo>>;
+
+    /// Get window information by platform-specific handle string
+    ///
+    /// Handle format is platform-specific: HWND string on Windows, X11 window ID on Linux,
+    /// PID+element reference on macOS. All code outside platform modules treats this as opaque.
+    fn get_window_by_hwnd(&self, hwnd: &str) -> Result<WindowInfo>;
+
+    /// Capture screenshot of a window
+    ///
+    /// # Arguments
+    /// * `hwnd` - Window handle string
+    /// * `method` - Screenshot method (platform-specific, e.g., "dwm" on Windows)
+    fn take_screenshot(&self, hwnd: &str, method: Option<&str>) -> Result<Screenshot>;
+
+    /// Dump element tree from window root
+    ///
+    /// Returns normalized UiaElement tree with cross-platform control types.
+    /// Coordinates are pixels relative to window origin with DPI handling internal.
+    fn dump_tree(&self, hwnd: &str, max_depth: u32) -> Result<UiaElement>;
+
+    /// Find elements matching selector
+    ///
+    /// # Arguments
+    /// * `hwnd` - Window handle string
+    /// * `selector` - Element selector (same syntax across platforms)
+    /// * `find_all` - Return all matches (true) or first match (false)
+    fn find_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>>;
+
+    /// Check if element matching selector exists
+    fn element_exists(&self, hwnd: &str, selector: &str) -> Result<bool>;
+
+    /// Invoke accessibility pattern on element
+    ///
+    /// # Arguments
+    /// * `hwnd` - Window handle string
+    /// * `selector` - Element selector
+    /// * `pattern` - Pattern name (e.g., "Invoke", "Value")
+    /// * `action` - Pattern-specific action
+    fn invoke_pattern(
+        &self,
+        hwnd: &str,
+        selector: &str,
+        pattern: &str,
+        action: Option<&str>,
+    ) -> Result<PatternResult>;
+
+    /// Get visual summary of window or element
+    fn get_summary(
+        &self,
+        hwnd: &str,
+        selector: &str,
+        include_invisible: bool,
+        include_offscreen: bool,
+        bbox: Option<[i32; 4]>,
+        max_depth: u32,
+        control_types: Option<Vec<String>>,
+    ) -> Result<String>;
+
+    /// Query elements with structured results
+    fn query_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult>;
+
+    /// Click at element or coordinates
+    fn click(&self, hwnd: &str, selector: &str, coords: Option<(i32, i32)>, button: Option<&str>) -> Result<()>;
+
+    /// Type text into element
+    fn type_text(&self, hwnd: &str, text: &str, selector: Option<&str>) -> Result<()>;
+
+    /// Send key combination (e.g., "ctrl+c")
+    fn send_keys(&self, keys: &str) -> Result<()>;
+
+    /// Scroll window or element
+    fn scroll(&self, direction: &str, amount: i32) -> Result<()>;
+}
```

```diff
--- a/src/ops/mod.rs
+++ b/src/ops/mod.rs
@@ -3,6 +3,10 @@
 //! This module contains the actual implementation of desktop automation operations,
 //! extracted from the RPC service for direct invocation without a daemon.

+pub mod traits;
+
+pub use traits::DesktopPlatform;
+
 #[cfg(windows)]
 mod windows_ops;
```

---

### Milestone 3: Refactor Windows to Implement Trait

**Files**:
- `src/ops/windows_ops.rs`
- `src/ops/mod.rs`

**Flags**: `conformance`

**Requirements**:
- Create `WindowsPlatform` struct implementing `DesktopPlatform`
- Wrap existing functions as trait method implementations
- Update `ops/mod.rs` to instantiate and use `WindowsPlatform`
- All existing Windows tests must still pass

**Acceptance Criteria**:
- `cargo test` passes on Windows with no regressions
- `WindowsPlatform` struct exported from `ops` module
- CLI commands work identically to before refactor

**Tests**:
- **Test files**: Existing Windows tests
- **Test type**: integration (real Windows APIs)
- **Backing**: existing tests
- **Scenarios**: All existing test scenarios pass

**Code Intent**:
- Modify `src/ops/windows_ops.rs`:
  - Add `pub struct WindowsPlatform;`
  - Add `impl DesktopPlatform for WindowsPlatform { ... }` wrapping existing functions
  - Keep existing functions as private helpers called by trait methods
- Modify `src/ops/mod.rs`:
  - Change platform dispatch to use `WindowsPlatform` struct
  - Export platform type for testing

**Code Changes**:

```diff
--- a/src/ops/windows_ops.rs
+++ b/src/ops/windows_ops.rs
@@ -3,6 +3,7 @@
 //! Direct implementations of desktop automation operations for Windows.

 use crate::automation::types::WindowInfo;
+use crate::ops::traits::DesktopPlatform;
 use crate::automation::windows::{
     capture_screenshot, get_window_info, list_windows as list_windows_raw, parse_hwnd,
     ScreenshotMethod,
@@ -39,6 +40,13 @@ impl From<crate::error::DesktopCliError> for OpsError {

 pub type Result<T> = std::result::Result<T, OpsError>;

+// ============================================================================
+// Platform Implementation
+// ============================================================================
+
+/// Windows platform implementation using UI Automation
+pub struct WindowsPlatform;
+
 // ============================================================================
 // Window Operations
 // ============================================================================
@@ -335,3 +343,87 @@ pub fn scroll(direction: &str, amount: i32) -> Result<()> {
     input_scroll(direction, amount).map_err(|e| OpsError(e.to_string()))
 }
+
+// ============================================================================
+// Trait Implementation
+// ============================================================================
+
+impl DesktopPlatform for WindowsPlatform {
+    fn list_windows(
+        &self,
+        exe_filter: Option<&str>,
+        title_filter: Option<&str>,
+    ) -> Result<Vec<WindowInfo>> {
+        list_windows(exe_filter, title_filter)
+    }
+
+    fn get_window_by_hwnd(&self, hwnd: &str) -> Result<WindowInfo> {
+        get_window_by_hwnd(hwnd)
+    }
+
+    fn take_screenshot(&self, hwnd: &str, method: Option<&str>) -> Result<Screenshot> {
+        take_screenshot(hwnd, method)
+    }
+
+    fn dump_tree(&self, hwnd: &str, max_depth: u32) -> Result<UiaElement> {
+        dump_tree(hwnd, max_depth)
+    }
+
+    fn find_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>> {
+        find_elements(hwnd, selector, find_all)
+    }
+
+    fn element_exists(&self, hwnd: &str, selector: &str) -> Result<bool> {
+        element_exists(hwnd, selector)
+    }
+
+    fn invoke_pattern(
+        &self,
+        hwnd: &str,
+        selector: &str,
+        pattern: &str,
+        action: Option<&str>,
+    ) -> Result<PatternResult> {
+        invoke_pattern(hwnd, selector, pattern, action)
+    }
+
+    fn get_summary(
+        &self,
+        hwnd: &str,
+        selector: &str,
+        include_invisible: bool,
+        include_offscreen: bool,
+        bbox: Option<[i32; 4]>,
+        max_depth: u32,
+        control_types: Option<Vec<String>>,
+    ) -> Result<String> {
+        get_summary(
+            hwnd,
+            selector,
+            include_invisible,
+            include_offscreen,
+            bbox,
+            max_depth,
+            control_types,
+        )
+    }
+
+    fn query_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult> {
+        query_elements(hwnd, selector, find_all)
+    }
+
+    fn click(&self, hwnd: &str, selector: &str, coords: Option<(i32, i32)>, button: Option<&str>) -> Result<()> {
+        click(hwnd, selector, coords, button)
+    }
+
+    fn type_text(&self, hwnd: &str, text: &str, selector: Option<&str>) -> Result<()> {
+        type_text(hwnd, text, selector)
+    }
+
+    fn send_keys(&self, keys: &str) -> Result<()> {
+        send_keys(keys)
+    }
+
+    fn scroll(&self, direction: &str, amount: i32) -> Result<()> {
+        scroll(direction, amount)
+    }
+}
```

```diff
--- a/src/ops/mod.rs
+++ b/src/ops/mod.rs
@@ -7,10 +7,16 @@ pub mod traits;

 pub use traits::DesktopPlatform;

+// ============================================================================
+// Platform-Specific Modules
+// ============================================================================
+
 #[cfg(windows)]
 mod windows_ops;

 #[cfg(windows)]
-pub use windows_ops::*;
+pub use windows_ops::{WindowsPlatform, OpsError, Result};
+#[cfg(windows)]
+pub use windows_ops::WindowsPlatform as Platform;

 #[cfg(not(windows))]
 mod stub_ops;
```

---

### Milestone 4: Add Linux Platform Module Structure

**Files**:
- `src/automation/linux/mod.rs` (new)
- `src/automation/linux/window.rs` (new)
- `src/automation/linux/atspi.rs` (new)
- `src/automation/linux/input.rs` (new)
- `src/automation/linux/screenshot.rs` (new)
- `src/automation/mod.rs`
- `Cargo.toml`

**Flags**: `needs-rationale`

**Requirements**:
- Create Linux automation module structure parallel to Windows
- Add Linux-specific dependencies to Cargo.toml
- Wire up module in automation/mod.rs with cfg(target_os = "linux")

**Acceptance Criteria**:
- `cargo check --target x86_64-unknown-linux-gnu` succeeds
- Module structure mirrors Windows organization
- Dependencies are target-specific (not compiled on Windows/macOS)

**Tests**:
- **Test files**: N/A (structure only)
- **Test type**: N/A
- **Backing**: N/A
- **Scenarios**: Compilation check only

**Code Intent**:
- New `src/automation/linux/mod.rs`: Module declarations for atspi, window, input, screenshot
- New `src/automation/linux/window.rs`: Stub `list_windows()`, `get_window_info()` using X11
- New `src/automation/linux/atspi.rs`: Stub AT-SPI2 connection and element tree
- New `src/automation/linux/input.rs`: Stub enigo wrappers for click, type, scroll
- New `src/automation/linux/screenshot.rs`: Stub xcap wrapper
- Modify `src/automation/mod.rs`: Add `#[cfg(target_os = "linux")] pub mod linux;`
- Modify `Cargo.toml`: Add target-specific deps:
  ```toml
  [target.'cfg(target_os = "linux")'.dependencies]
  atspi = "0.21"
  x11rb = "0.13"
  enigo = "0.2"
  xcap = "0.0.13"
  ```

**Code Changes**:

```diff
--- /dev/null
+++ b/src/automation/linux/mod.rs
@@ -0,0 +1,6 @@
+//! Linux automation implementation using AT-SPI2 and X11
+
+pub mod atspi;
+pub mod input;
+pub mod screenshot;
+pub mod window;
```

```diff
--- /dev/null
+++ b/src/automation/linux/window.rs
@@ -0,0 +1,8 @@
+//! X11 window enumeration and information
+
+use crate::automation::types::WindowInfo;
+use crate::error::Result;
+
+pub fn list_windows(_exe_filter: Option<&str>, _title_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
+    unimplemented!("Linux window enumeration - Milestone 5")
+}
```

```diff
--- /dev/null
+++ b/src/automation/linux/atspi.rs
@@ -0,0 +1,8 @@
+//! AT-SPI2 accessibility tree operations
+
+use crate::rpc::types::UiaElement;
+use crate::error::Result;
+
+pub fn dump_tree(_window_id: &str, _max_depth: u32) -> Result<UiaElement> {
+    unimplemented!("AT-SPI2 tree dump - Milestone 6")
+}
```

```diff
--- /dev/null
+++ b/src/automation/linux/input.rs
@@ -0,0 +1,12 @@
+//! Input simulation via enigo
+
+use crate::error::Result;
+
+pub fn click_at_coords(_x: i32, _y: i32) -> Result<()> {
+    unimplemented!("Linux input - Milestone 7")
+}
+
+pub fn type_text(_text: &str) -> Result<()> {
+    unimplemented!("Linux input - Milestone 7")
+}
```

```diff
--- /dev/null
+++ b/src/automation/linux/screenshot.rs
@@ -0,0 +1,11 @@
+//! Screenshot capture via xcap
+
+use crate::rpc::types::Screenshot;
+use crate::error::Result;
+
+pub fn capture_window(_window_id: &str) -> Result<Screenshot> {
+    unimplemented!("Linux screenshot - Milestone 7")
+}
```

```diff
--- a/src/automation/mod.rs
+++ b/src/automation/mod.rs
@@ -1,7 +1,10 @@
 pub mod types;

 #[cfg(windows)]
 pub mod windows;

 #[cfg(windows)]
 pub use windows::*;
+
+#[cfg(target_os = "linux")]
+pub mod linux;
```

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -59,3 +59,11 @@ windows = { version = "0.58", features = [
 ] }
 win-screenshot = "4"
 uiautomation = "0.24"
+
+# Linux Automation (Linux only)
+[target.'cfg(target_os = "linux")'.dependencies]
+atspi = "0.21"
+x11rb = "0.13"
+enigo = "0.2"
+xcap = "0.0.13"
```

---

### Milestone 5: Implement Linux Window Enumeration

**Files**:
- `src/automation/linux/window.rs`
- `src/ops/linux_ops.rs` (new)
- `src/ops/mod.rs`

**Flags**: `error-handling`

**Requirements**:
- Implement `list_windows()` using X11 via x11rb
- Implement `get_window_by_hwnd()` (hwnd = X11 window ID as string)
- Create `LinuxPlatform` struct implementing `DesktopPlatform` (partial)
- Wire into ops/mod.rs for Linux target

**Acceptance Criteria**:
- `desktop windows` lists visible X11 windows on Linux
- Window info includes title, PID, executable path, rect
- Returns empty list (not error) when no windows found
- Returns informative error if X11 connection fails

**Tests**:
- **Test files**: `tests/linux_integration_test.rs` (new)
- **Test type**: integration (real X11)
- **Backing**: user-specified (real deps)
- **Scenarios**:
  - Normal: Lists at least one window (test runner)
  - Edge: Filters by executable name
  - Error: Handles X11 connection failure gracefully

**Code Intent**:
- Implement `src/automation/linux/window.rs`:
  - `list_windows()`: Connect to X11 via x11rb, enumerate windows via `_NET_CLIENT_LIST`, get properties. Uses _NET_CLIENT_LIST rather than XQueryTree to exclude non-managed windows.
  - `get_window_info(window_id)`: Get single window properties. Queries _NET_WM_NAME and _NET_WM_PID atoms for title and process info.
  - Map X11 window ID to hex string for `hwnd` field (Decision: string-based window handles abstract platform-specific types)
- New `src/ops/linux_ops.rs`:
  - `LinuxPlatform` struct
  - Implement `list_windows`, `get_window_by_hwnd` on trait
  - Other methods return `OpsError("Not yet implemented")`
- Modify `src/ops/mod.rs`:
  - Add `#[cfg(target_os = "linux")] mod linux_ops;`
  - Use `LinuxPlatform` in Linux cfg block

**Code Changes**:

```diff
--- a/src/automation/linux/window.rs
+++ b/src/automation/linux/window.rs
@@ -1,8 +1,73 @@
 //! X11 window enumeration and information
+//!
+//! Uses x11rb to query _NET_CLIENT_LIST for visible windows. Parallels
+//! Windows EnumWindows but via X11 protocol instead of Win32 API.

 use crate::automation::types::{WindowInfo, WindowRect};
 use crate::error::Result;
+use x11rb::connection::Connection;
+use x11rb::protocol::xproto::*;
+use x11rb::rust_connection::RustConnection;

-pub fn list_windows(_exe_filter: Option<&str>, _title_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
-    unimplemented!("Linux window enumeration - Milestone 5")
+pub fn list_windows(exe_filter: Option<&str>, title_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
+    let (conn, screen_num) = RustConnection::connect(None)
+        .map_err(|e| crate::error::DesktopCliError::Platform(format!("X11 connection failed: {}", e)))?;
+
+    let screen = &conn.setup().roots[screen_num];
+    let net_client_list = conn.intern_atom(false, b"_NET_CLIENT_LIST")
+        .map_err(|e| crate::error::DesktopCliError::Platform(format!("Failed to intern atom: {}", e)))?
+        .reply()
+        .map_err(|e| crate::error::DesktopCliError::Platform(format!("Failed to get atom reply: {}", e)))?
+        .atom;
+
+    let property = conn.get_property(false, screen.root, net_client_list, AtomEnum::WINDOW, 0, u32::MAX)
+        .map_err(|e| crate::error::DesktopCliError::Platform(format!("Failed to get property: {}", e)))?
+        .reply()
+        .map_err(|e| crate::error::DesktopCliError::Platform(format!("Failed to get property reply: {}", e)))?;
+
+    // X11 properties return byte arrays; window IDs are 32-bit values
+    // Unsafe cast is safe: X11 protocol guarantees format=32 means 4-byte alignment
+    let windows: &[u32] = if property.format == 32 {
+        unsafe { std::slice::from_raw_parts(property.value.as_ptr() as *const u32, property.value.len() / 4) }
+    } else {
+        &[]
+    };
+
+    let mut result = Vec::new();
+    for &window_id in windows {
+        if let Ok(info) = get_window_info(&conn, window_id) {
+            let matches_exe = exe_filter.map_or(true, |filter|
+                info.executable.to_lowercase().contains(&filter.to_lowercase()));
+            let matches_title = title_filter.map_or(true, |filter|
+                info.title.to_lowercase().contains(&filter.to_lowercase()));
+
+            if matches_exe && matches_title {
+                result.push(info);
+            }
+        }
+    }
+
+    Ok(result)
+}
+
+/// Get window information for a single X11 window
+///
+/// Queries window properties via X11 protocol. Called during window enumeration
+/// and direct window lookups by ID.
+fn get_window_info(conn: &RustConnection, window_id: u32) -> Result<WindowInfo> {
+    let net_wm_name = conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
+    let net_wm_pid = conn.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
+
+    let title_prop = conn.get_property(false, window_id, net_wm_name, AtomEnum::ANY, 0, 1024)?.reply()?;
+    let title = String::from_utf8_lossy(&title_prop.value).to_string();
+
+    let pid_prop = conn.get_property(false, window_id, net_wm_pid, AtomEnum::CARDINAL, 0, 1)?.reply()?;
+    let pid = if pid_prop.format == 32 && !pid_prop.value.is_empty() {
+        u32::from_ne_bytes(pid_prop.value[0..4].try_into().unwrap())
+    } else {
+        0
+    };
+
+    let geometry = conn.get_geometry(window_id)?.reply()?;
+
+    Ok(WindowInfo {
+        // X11 window IDs formatted as hex to match Windows HWND convention
+        hwnd: format!("0x{:x}", window_id),
+        title,
+        // /proc/{pid}/exe is symlink to executable path on Linux
+        executable: format!("/proc/{}/exe", pid),
+        rect: WindowRect { x: geometry.x as i32, y: geometry.y as i32, width: geometry.width as u32, height: geometry.height as u32 },
+        pid,
+        class_name: None,
+    })
 }
```

```diff
--- /dev/null
+++ b/src/ops/linux_ops.rs
@@ -0,0 +1,90 @@
+//! Linux-specific operation implementations
+
+use crate::automation::types::WindowInfo;
+use crate::automation::linux;
+use crate::ops::traits::DesktopPlatform;
+use crate::rpc::types::{PatternResult, QueryResult, Screenshot, UiaElement};
+
+use super::{OpsError, Result};
+
+/// Linux platform implementation using AT-SPI2 and X11
+pub struct LinuxPlatform;
+
+fn not_implemented<T>() -> Result<T> {
+    Err(OpsError("Not yet implemented".to_string()))
+}
+
+impl DesktopPlatform for LinuxPlatform {
+    fn list_windows(
+        &self,
+        exe_filter: Option<&str>,
+        title_filter: Option<&str>,
+    ) -> Result<Vec<WindowInfo>> {
+        linux::window::list_windows(exe_filter, title_filter)
+            .map_err(|e| OpsError(e.to_string()))
+    }
+
+    fn get_window_by_hwnd(&self, hwnd: &str) -> Result<WindowInfo> {
+        let window_id = u32::from_str_radix(hwnd.trim_start_matches("0x"), 16)
+            .map_err(|e| OpsError(format!("Invalid window ID: {}", e)))?;
+        linux::window::get_window_info_by_id(window_id)
+            .map_err(|e| OpsError(e.to_string()))
+    }
+
+    fn take_screenshot(&self, _hwnd: &str, _method: Option<&str>) -> Result<Screenshot> {
+        not_implemented()
+    }
+
+    fn dump_tree(&self, _hwnd: &str, _max_depth: u32) -> Result<UiaElement> {
+        not_implemented()
+    }
+
+    fn find_elements(&self, _hwnd: &str, _selector: &str, _find_all: bool) -> Result<Vec<UiaElement>> {
+        not_implemented()
+    }
+
+    fn element_exists(&self, _hwnd: &str, _selector: &str) -> Result<bool> {
+        not_implemented()
+    }
+
+    fn invoke_pattern(
+        &self,
+        _hwnd: &str,
+        _selector: &str,
+        _pattern: &str,
+        _action: Option<&str>,
+    ) -> Result<PatternResult> {
+        not_implemented()
+    }
+
+    fn get_summary(
+        &self,
+        _hwnd: &str,
+        _selector: &str,
+        _include_invisible: bool,
+        _include_offscreen: bool,
+        _bbox: Option<[i32; 4]>,
+        _max_depth: u32,
+        _control_types: Option<Vec<String>>,
+    ) -> Result<String> {
+        not_implemented()
+    }
+
+    fn query_elements(&self, _hwnd: &str, _selector: &str, _find_all: bool) -> Result<QueryResult> {
+        not_implemented()
+    }
+
+    fn click(&self, _hwnd: &str, _selector: &str, _coords: Option<(i32, i32)>, _button: Option<&str>) -> Result<()> {
+        not_implemented()
+    }
+
+    fn type_text(&self, _hwnd: &str, _text: &str, _selector: Option<&str>) -> Result<()> {
+        not_implemented()
+    }
+
+    fn send_keys(&self, _keys: &str) -> Result<()> {
+        not_implemented()
+    }
+
+    fn scroll(&self, _direction: &str, _amount: i32) -> Result<()> {
+        not_implemented()
+    }
+}
```

```diff
--- a/src/ops/mod.rs
+++ b/src/ops/mod.rs
@@ -18,3 +18,10 @@ pub use windows_ops::{WindowsPlatform, OpsError, Result};
 #[cfg(windows)]
 pub use windows_ops::WindowsPlatform as Platform;

 #[cfg(not(windows))]
 mod stub_ops;
+
+#[cfg(target_os = "linux")]
+mod linux_ops;
+
+#[cfg(target_os = "linux")]
+pub use linux_ops::{LinuxPlatform, OpsError, Result};
+#[cfg(target_os = "linux")]
+pub use linux_ops::LinuxPlatform as Platform;
```

---

### Milestone 6: Implement Linux AT-SPI2 Element Tree

**Files**:
- `src/automation/linux/atspi.rs`
- `src/automation/linux/roles.rs` (new)
- `src/ops/linux_ops.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- Connect to AT-SPI2 via D-Bus using atspi crate
- Implement element tree traversal from window accessible
- Map AT-SPI2 roles to UIA-style control types
- Implement `dump_tree()`, `find_elements()`, `element_exists()`

**Acceptance Criteria**:
- `desktop dump-tree :1` shows element hierarchy on Linux
- Element control_type uses Windows-compatible names (Button, Edit, etc.)
- Element bounds are in window-relative coordinates
- Returns informative error if AT-SPI2 not available

**Tests**:
- **Test files**: `tests/linux_integration_test.rs`
- **Test type**: integration (real AT-SPI2)
- **Backing**: user-specified
- **Scenarios**:
  - Normal: Dump tree shows window elements
  - Edge: Deep tree (max_depth respected)
  - Error: AT-SPI2 service unavailable

**Code Intent**:
- Implement `src/automation/linux/atspi.rs`:
  - `connect()`: Establish AT-SPI2 D-Bus connection. Returns informative error if AT-SPI2 service not running (Risk: AT-SPI2 not available).
  - `get_accessible_from_window(window_id)`: Get root accessible for X11 window. Bridges X11 window ID to AT-SPI2 accessible object.
  - `traverse_tree(accessible, depth, options)`: Recursive traversal returning UiaElement. Respects max_depth to prevent deep tree performance issues.
  - `find_by_selector(root, selector)`: Search tree for matching elements. Uses 500ms timeout (Decision: fast feedback vs Windows 3000ms).
- New `src/automation/linux/roles.rs`:
  - `map_role(atspi_role: &str) -> String`: Maps AT-SPI2 roles to Windows-style (Decision: role normalization ensures cross-platform element tree compatibility)
  - Table: "push button" → "Button", "text" → "Edit", "menu" → "Menu", etc. See Planning Context role mapping tables for complete mapping.
- Update `src/ops/linux_ops.rs`:
  - Implement `dump_tree`, `find_elements`, `element_exists` using atspi module. Wrap platform errors in OpsError (Invariant: no platform-specific errors leak to CLI).

**Code Changes**:

(Complex AT-SPI2 implementation - diffs require detailed D-Bus code)

---

### Milestone 7: Implement Linux Input and Screenshot

**Files**:
- `src/automation/linux/input.rs`
- `src/automation/linux/screenshot.rs`
- `src/ops/linux_ops.rs`

**Requirements**:
- Implement click, type_text, send_keys, scroll using enigo
- Implement screenshot capture using xcap
- Complete remaining DesktopPlatform trait methods

**Acceptance Criteria**:
- `desktop click :1 --coords 100,100` clicks at coordinates
- `desktop type :1 "#input" --value "test"` types text
- `desktop keys :1 "ctrl+c"` sends key combination
- `desktop screenshot :1` returns base64 PNG

**Tests**:
- **Test files**: `tests/linux_integration_test.rs`
- **Test type**: integration (real input/screenshot)
- **Backing**: user-specified
- **Scenarios**:
  - Normal: Click registers at coordinates
  - Normal: Screenshot captures window content
  - Edge: Special keys (ctrl, alt, shift)
  - Error: Invalid coordinates

**Code Intent**:
- Implement `src/automation/linux/input.rs`:
  - `click_at_coords(x, y)`: Use enigo to move and click. Enigo chosen for cross-platform API consistency (Decision: single input abstraction).
  - `type_text(text)`: Use enigo to type characters. Handles keyboard layouts automatically.
  - `send_keys(combo)`: Parse combo string (e.g., "ctrl+c"), use enigo for key events. Matches Windows SendInput behavior.
  - `scroll(direction, amount)`: Use enigo scroll. X11-only initially (Constraint: Wayland deferred).
- Implement `src/automation/linux/screenshot.rs`:
  - `capture_window(window_id)`: Use xcap to capture, encode to PNG base64. xcap chosen for multi-platform support (Decision: simpler than platform-specific solutions).
- Update `src/ops/linux_ops.rs`:
  - Implement `click`, `type_text`, `send_keys`, `scroll`, `take_screenshot`. Completes DesktopPlatform trait for Linux.
  - Implement remaining methods: `invoke_pattern`, `get_summary`, `query_elements`. Delegate to atspi module.

**Code Changes**:

(Input/screenshot implementation - diffs require enigo and xcap integration)

---

### Milestone 8: Add macOS Platform Module Structure

**Files**:
- `src/automation/macos/mod.rs` (new)
- `src/automation/macos/window.rs` (new)
- `src/automation/macos/accessibility.rs` (new)
- `src/automation/macos/input.rs` (new)
- `src/automation/macos/screenshot.rs` (new)
- `src/automation/macos/permissions.rs` (new)
- `src/automation/mod.rs`
- `Cargo.toml`

**Requirements**:
- Create macOS automation module structure
- Add macOS-specific dependencies
- Include permissions detection module for graceful degradation

**Acceptance Criteria**:
- `cargo check --target x86_64-apple-darwin` succeeds
- Module structure mirrors Linux/Windows
- Dependencies are target-specific

**Tests**:
- **Test files**: N/A (structure only)
- **Test type**: N/A
- **Backing**: N/A
- **Scenarios**: Compilation check only

**Code Intent**:
- Structure parallel to Linux milestone 4
- New `src/automation/macos/permissions.rs`: `check_accessibility_permission() -> bool`
- Modify `Cargo.toml`:
  ```toml
  [target.'cfg(target_os = "macos")'.dependencies]
  accessibility-sys = "0.1"
  enigo = "0.2"
  xcap = "0.0.13"
  ```

**Code Changes**:

```diff
--- /dev/null
+++ b/src/automation/macos/mod.rs
@@ -0,0 +1,7 @@
+//! macOS automation implementation using Cocoa Accessibility
+
+pub mod accessibility;
+pub mod input;
+pub mod permissions;
+pub mod screenshot;
+pub mod window;
```

```diff
--- /dev/null
+++ b/src/automation/macos/permissions.rs
@@ -0,0 +1,6 @@
+//! macOS accessibility permission detection
+
+pub fn check_accessibility_permission() -> bool {
+    unimplemented!("macOS permissions - Milestone 9")
+}
```

```diff
--- /dev/null
+++ b/src/automation/macos/window.rs
@@ -0,0 +1,8 @@
+//! macOS window enumeration via Cocoa
+
+use crate::automation::types::WindowInfo;
+use crate::error::Result;
+
+pub fn list_windows(_exe_filter: Option<&str>, _title_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
+    unimplemented!("macOS window enumeration - Milestone 9")
+}
```

```diff
--- /dev/null
+++ b/src/automation/macos/accessibility.rs
@@ -0,0 +1,8 @@
+//! Cocoa Accessibility tree operations
+
+use crate::rpc::types::UiaElement;
+use crate::error::Result;
+
+pub fn dump_tree(_window_ref: &str, _max_depth: u32) -> Result<UiaElement> {
+    unimplemented!("macOS accessibility - Milestone 9")
+}
```

```diff
--- /dev/null
+++ b/src/automation/macos/input.rs
@@ -0,0 +1,8 @@
+//! Input simulation via enigo
+
+use crate::error::Result;
+
+pub fn click_at_coords(_x: i32, _y: i32) -> Result<()> {
+    unimplemented!("macOS input - Milestone 10")
+}
```

```diff
--- /dev/null
+++ b/src/automation/macos/screenshot.rs
@@ -0,0 +1,11 @@
+//! Screenshot capture via xcap
+
+use crate::rpc::types::Screenshot;
+use crate::error::Result;
+
+pub fn capture_window(_window_ref: &str) -> Result<Screenshot> {
+    unimplemented!("macOS screenshot - Milestone 10")
+}
```

```diff
--- a/src/automation/mod.rs
+++ b/src/automation/mod.rs
@@ -8,3 +8,6 @@ pub use windows::*;

 #[cfg(target_os = "linux")]
 pub mod linux;
+
+#[cfg(target_os = "macos")]
+pub mod macos;
```

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -66,3 +66,10 @@ atspi = "0.21"
 x11rb = "0.13"
 enigo = "0.2"
 xcap = "0.0.13"
+
+# macOS Automation (macOS only)
+[target.'cfg(target_os = "macos")'.dependencies]
+accessibility-sys = "0.1"
+enigo = "0.2"
+xcap = "0.0.13"
```

---

### Milestone 9: Implement macOS Window and Accessibility

**Files**:
- `src/automation/macos/window.rs`
- `src/automation/macos/accessibility.rs`
- `src/automation/macos/roles.rs` (new)
- `src/automation/macos/permissions.rs`
- `src/ops/macos_ops.rs` (new)
- `src/ops/mod.rs`

**Flags**: `error-handling`, `needs-rationale`

**Requirements**:
- Implement window enumeration via Cocoa/accessibility
- Implement element tree traversal via AXUIElement
- Check permissions and return graceful error if missing
- Map macOS AX roles to UIA-style types

**Acceptance Criteria**:
- `desktop windows` lists windows on macOS
- `desktop dump-tree :1` shows element hierarchy
- Missing permissions returns helpful error with remediation steps
- Roles normalized to Windows-compatible names

**Tests**:
- **Test files**: `tests/macos_integration_test.rs` (new)
- **Test type**: integration (real Cocoa)
- **Backing**: user-specified
- **Scenarios**:
  - Normal: Lists windows, dumps tree
  - Edge: Permission not granted (graceful error)
  - Error: Invalid window reference

**Code Intent**:
- Implement permissions check first (Decision: graceful degradation - failing fast would block all operations, informative error guides user to System Preferences)
- Use `AXUIElementCopyAttributeValue` for element properties. Cocoa Accessibility API parallels AT-SPI2 approach on Linux.
- Role mapping: "AXButton" → "Button", "AXTextField" → "Edit", etc. See Planning Context macOS AX to UIA mapping table for complete mapping.
- `MacOSPlatform` struct implementing `DesktopPlatform`. Window handle format is "PID:element_ref" rather than numeric ID.

**Code Changes**:

(Complex Cocoa Accessibility implementation - diffs require detailed AXUIElement code)

---

### Milestone 10: Implement macOS Input and Screenshot

**Files**:
- `src/automation/macos/input.rs`
- `src/automation/macos/screenshot.rs`
- `src/ops/macos_ops.rs`

**Requirements**:
- Implement input operations via enigo
- Implement screenshot via xcap
- Handle DPI scaling for coordinates
- Complete DesktopPlatform implementation

**Acceptance Criteria**:
- All CLI commands work on macOS
- Screenshots have correct dimensions
- Coordinates account for Retina scaling

**Tests**:
- **Test files**: `tests/macos_integration_test.rs`
- **Test type**: integration
- **Backing**: user-specified
- **Scenarios**:
  - Normal: Click, type, screenshot work
  - Edge: Retina display coordinates
  - Error: Missing permissions for input

**Code Intent**:
- Similar to Linux milestone 7 (enigo for input, xcap for screenshot)
- Add DPI detection for coordinate conversion. Retina displays require scaling factor to convert logical to physical pixels (Invariant: all coordinates are pixels relative to window origin).
- Complete all trait methods in `MacOSPlatform`. Check accessibility permissions before operations (Decision: graceful degradation).

**Code Changes**:

(macOS input/screenshot implementation - diffs require enigo, xcap, and DPI handling)

---

### Milestone 11: Cross-Platform Integration Tests

**Files**:
- `tests/cross_platform_test.rs` (new)
- `tests/common/mod.rs` (new)

**Requirements**:
- Shared test fixtures for window mocking
- Generated test datasets for consistent behavior verification
- Platform-conditional test execution

**Acceptance Criteria**:
- Tests run on all platforms (skipping platform-specific features)
- Common behaviors verified consistently
- CI matrix covers Windows, Linux (X11), macOS

**Tests**:
- **Test files**: `tests/cross_platform_test.rs`
- **Test type**: integration + property-based
- **Backing**: user-specified (generated datasets)
- **Scenarios**:
  - Selector parsing (property-based, all platforms)
  - Role mapping consistency
  - Coordinate handling
  - Error message format

**Code Intent**:
- `tests/common/mod.rs`: Shared helpers, mock window generators
- `tests/cross_platform_test.rs`:
  - Property tests for selector parsing (quickcheck)
  - Role mapping validation across platforms
  - Window info serialization roundtrip

**Code Changes**:

(Cross-platform test suite - diffs require property-based test implementation)

---

### Milestone 12: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `src/ops/CLAUDE.md` (new)
- `src/ops/README.md` (new)
- `src/automation/CLAUDE.md` (new)
- `src/automation/README.md` (new)
- `docs/PLATFORMS.md` (new)
- `docs/SETUP.md` (new)

**Requirements**:
- CLAUDE.md files with tabular index format
- README.md files with architecture, invariants, tradeoffs
- Platform-specific setup guides

**Acceptance Criteria**:
- CLAUDE.md is pure navigation index
- README.md captures invisible knowledge
- Setup guides document permissions and dependencies

Documentation milestone - no code changes.

## Milestone Dependencies

```
M1 (test fixes)
    │
    ▼
M2 (trait definition)
    │
    ▼
M3 (Windows refactor)
    │
    ├────────────────────┬────────────────────┐
    ▼                    ▼                    ▼
M4 (Linux structure)  M8 (macOS structure)   │
    │                    │                    │
    ▼                    ▼                    │
M5 (Linux windows)    M9 (macOS windows)     │
    │                    │                    │
    ▼                    ▼                    │
M6 (Linux AT-SPI2)    (included in M9)       │
    │                    │                    │
    ▼                    ▼                    │
M7 (Linux input)      M10 (macOS input)      │
    │                    │                    │
    └────────────────────┴────────────────────┘
                         │
                         ▼
                    M11 (cross-platform tests)
                         │
                         ▼
                    M12 (documentation)
```

**Parallel Execution**:
- M4-M7 (Linux) and M8-M10 (macOS) can proceed in parallel after M3
- M11 requires M7 and M10 complete
- M12 requires M11 complete
