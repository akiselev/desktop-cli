# Operations Layer Architecture

Platform abstraction layer for desktop automation operations.

## Architecture

CLI → ops/mod.rs (compile-time platform dispatch) → Platform-specific implementation (WindowsPlatform | LinuxPlatform | MacOSPlatform)

### Compile-Time Dispatch

`mod.rs` selects platform implementation via cfg attributes. Single binary contains only target platform code. No runtime overhead.

```rust
#[cfg(windows)]
pub use windows_ops::WindowsPlatform as Platform;

#[cfg(target_os = "linux")]
pub use linux_ops::LinuxPlatform as Platform;

#[cfg(target_os = "macos")]
pub use macos_ops::MacOSPlatform as Platform;
```

Each platform module exports functions wrapping trait methods for direct invocation compatibility.

## Invariants

1. **Window handle opacity**: `hwnd: String` is opaque outside platform modules. Format is platform-specific (HWND hex on Windows, X11 window ID on Linux, PID:ref on macOS). Only platform code parses.

2. **Element tree normalization**: All platforms return `UiaElement` with consistent role names. Platform-specific roles mapped to Windows UIA control types (Button, Edit, Menu, etc.).

3. **Coordinate system**: All coordinates are pixels relative to window origin. DPI handling internal to platform modules.

4. **Error propagation**: Platform errors wrapped in `OpsError`. No platform-specific error types leak to CLI.

## Tradeoffs

| Choice | Benefit | Cost |
|--------|---------|------|
| Compile-time dispatch | Zero runtime overhead; type-safe; binary only contains target platform code | No runtime platform switching; test coverage requires CI matrix |
| Single trait `DesktopPlatform` | Simple interface; uniform API for testing | May need extension for platform-specific features not in common API |
| String window handles | Simple cross-platform abstraction; uniform API | Parsing overhead on each operation (mitigated by single parse) |

## Platform Implementations

### Windows (windows_ops.rs)
- UI Automation (UIA) via `uiautomation` crate
- SendInput for keyboard/mouse via `windows` crate
- Window enumeration via Win32 `EnumWindows`
- Screenshot via DWM or win-screenshot

### Linux (linux_ops.rs)
- AT-SPI2 via `atspi` crate for element tree
- X11 via `x11rb` for window enumeration
- enigo for input simulation
- xcap for screenshots
- X11-only initially; Wayland deferred

### macOS (macos_ops.rs)
- Cocoa Accessibility via `accessibility-sys`
- AXUIElement for element tree
- enigo for input simulation
- xcap for screenshots
- Requires accessibility permissions (graceful degradation on missing permissions)

## Data Flow

User Command → Parse window query (targeting/parser.rs) → Resolve to platform handle (ops::list_windows → filter) → Platform-specific operation → Serialize result (rpc/types.rs) → Output

## Why This Structure

- **ops/ as dispatch layer**: Single entry point for all platform operations. Test surface for mocking platform implementations.
- **Trait-based abstraction**: Enables shared logic and mock testing while preserving compile-time dispatch.
- **Platform modules separated**: Isolated platform-specific dependencies. No cross-contamination between Windows, Linux, macOS code.
