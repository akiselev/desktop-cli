# Automation Module Architecture

Platform-specific desktop automation implementations.

## Architecture

```
automation/
├── types.rs              # Cross-platform types (WindowInfo, WindowRect)
├── windows/              # Windows automation
│   ├── uia/              # UI Automation element tree
│   ├── input.rs          # SendInput keyboard/mouse
│   ├── screenshot.rs     # DWM/win-screenshot capture
│   └── window.rs         # EnumWindows enumeration
├── linux/                # Linux automation
│   ├── atspi.rs          # AT-SPI2 element tree
│   ├── window.rs         # X11 window enumeration
│   ├── input.rs          # enigo input simulation
│   ├── screenshot.rs     # xcap capture
│   └── roles.rs          # AT-SPI2 → UIA role mapping
└── macos/                # macOS automation
    ├── accessibility.rs  # AXUIElement element tree
    ├── window.rs         # Cocoa window enumeration
    ├── input.rs          # enigo input simulation
    ├── screenshot.rs     # xcap capture
    ├── permissions.rs    # Accessibility permission check
    └── roles.rs          # AXRole → UIA role mapping
```

## Platform Differences

### Windows
- Native UI Automation (UIA) provides element tree with consistent control types
- SendInput for input simulation (OS-level)
- HWND-based window handles (hex string format)
- DPI awareness built into Windows APIs

### Linux (X11)
- AT-SPI2 over D-Bus provides element tree (requires running service)
- X11 protocol for window enumeration via `_NET_CLIENT_LIST`
- enigo for input simulation (X11 level)
- xcap for screenshots
- Window handles are X11 window IDs (hex string format)
- Wayland support deferred (X11-only initially)

### macOS
- Cocoa Accessibility framework provides element tree
- AXUIElement for element queries
- Requires explicit user permission grant (System Preferences → Security & Privacy → Accessibility)
- enigo for input simulation
- xcap for screenshots
- Window handles are PID:element_ref format
- Retina DPI scaling handled in coordinate conversion

## Role Normalization

All platforms map native roles to Windows UIA control types:

### AT-SPI2 → UIA (Linux)
- `push button` → `Button`
- `text` → `Edit`
- `menu` → `Menu`
- `menu item` → `MenuItem`
- `check box` → `CheckBox`
- See `linux/roles.rs` for complete mapping

### AXRole → UIA (macOS)
- `AXButton` → `Button`
- `AXTextField` → `Edit`
- `AXMenu` → `Menu`
- `AXMenuItem` → `MenuItem`
- `AXCheckBox` → `CheckBox`
- See `macos/roles.rs` for complete mapping

## Invariants

1. **Element tree structure**: All platforms return `UiaElement` with normalized `control_type` field. Tree traversal APIs are consistent.

2. **Coordinate system**: Coordinates are pixels relative to window origin. DPI scaling handled internally on macOS (Retina) and Windows (high-DPI).

3. **Async operations**: AT-SPI2 on Linux uses async D-Bus. Runtime is tokio (already a dependency).

4. **Error handling**: Platform-specific errors (AT-SPI2 service unavailable, macOS permission denied, X11 connection failed) wrapped in `DesktopCliError::Platform`.

## Why This Structure

- **Platform isolation**: Each platform has dedicated directory. No shared code that would couple platforms.
- **Parallel to ops/**: Each automation module mirrors ops structure. `linux_ops.rs` calls `automation::linux::*`.
- **Shared types in types.rs**: `WindowInfo` and `WindowRect` are already cross-platform. No platform-specific variants needed.
- **Role mapping as separate module**: Role normalization logic isolated in `roles.rs` per platform. Single responsibility.

## Platform-Specific Notes

### Linux AT-SPI2 Setup
AT-SPI2 D-Bus service must be running. Standard on GNOME and KDE. Detection via D-Bus connection attempt returns informative error with activation hint if unavailable.

### macOS Permissions
Operations requiring accessibility return graceful error with remediation steps if permission not granted. Non-accessibility operations (window listing without element tree) remain functional.

### Windows UIA
Available by default on Windows. No service dependencies or permission prompts.
