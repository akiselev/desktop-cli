# Platform Support

Cross-platform desktop automation implementation status and requirements.

## Supported Platforms

| Platform | Window Enumeration | Element Tree | Input Simulation | Screenshots | Status |
|----------|-------------------|--------------|------------------|-------------|--------|
| Windows | Win32 EnumWindows | UI Automation | SendInput | DWM/win-screenshot | Full support |
| Linux (X11) | X11 `_NET_CLIENT_LIST` | AT-SPI2 | enigo | xcap | Partial support (see limitations) |
| macOS | Cocoa | Accessibility | enigo | xcap | Stub (not implemented) |
| Wayland | - | - | - | - | Not supported (X11 only on Linux) |

## Platform Requirements

### Windows

**Minimum Version**: Windows 7 with UI Automation support (Vista+ has UIA)

**Dependencies**: None. UI Automation is built into Windows.

**Permissions**: None required.

**Known Limitations**: None.

### Linux (X11)

**Minimum Version**: Any modern Linux distribution with X11

**Dependencies**:
- X11 server (Xorg)
- AT-SPI2 D-Bus service (standard on GNOME, KDE, XFCE)

**Permissions**: None required (uses user session D-Bus).

**Known Limitations**:
- **Wayland not supported**: Only X11 is supported. Wayland requires different input simulation approach (libei) which has incomplete Rust support. Users on Wayland must run X11 session or use Xwayland.
- **AT-SPI2 service must be running**: Most desktop environments start this by default. If unavailable, operations return informative error with activation hint.
- **Partial implementation**: Window enumeration, element tree, and input work. Screenshot captures full monitor (not window-specific). Pattern invocation, summary, and query operations not yet implemented.

### macOS

**Status**: ⚠ **Stub implementation - not yet functional**

**Minimum Version**: macOS 10.15+ (Catalina and later)

**Dependencies**: None planned. Cocoa Accessibility framework is built into macOS.

**Current State**: Module structure exists but all operations return "Platform not supported" error. macOS support is planned but not implemented in this release.

**Planned Features** (when implemented):
- Accessibility permission required for automation operations
- Grant in System Preferences → Security & Privacy → Privacy → Accessibility
- Add terminal emulator or application running desktop-cli to allowed list
- Operations will return graceful error with remediation steps if permission not granted

**Known Limitations**:
- **Not implemented**: All automation operations currently return errors. Use Windows or Linux (X11) for functional automation.
- **Permission required** (future): First run will require user to manually grant accessibility permission. Non-interactive grant not possible due to macOS security model.
- **Retina DPI** (future): Coordinates will be logical pixels. Internal conversion will handle Retina scaling.

## Platform Selection

Platform implementation selected at compile-time via Rust cfg attributes. Binary contains only target platform code.

### Compile Targets

```bash
# Windows
cargo build --target x86_64-pc-windows-msvc

# Linux
cargo build --target x86_64-unknown-linux-gnu

# macOS
cargo build --target x86_64-apple-darwin
cargo build --target aarch64-apple-darwin  # Apple Silicon
```

### Cross-Platform Development

Each platform has isolated implementation. No platform-specific code executes on other platforms. Testing requires CI matrix with runners for each platform.

## Automation API Differences

All platforms implement `DesktopPlatform` trait with uniform API. Internal implementation uses platform-specific automation frameworks:

| Operation | Windows | Linux | macOS |
|-----------|---------|-------|-------|
| List windows | `EnumWindows` Win32 API | X11 `_NET_CLIENT_LIST` property | Cocoa `NSWorkspace` |
| Element tree | UI Automation `IUIAutomation` | AT-SPI2 D-Bus protocol | Accessibility `AXUIElement` |
| Input simulation | `SendInput` Win32 API | enigo X11 backend | enigo macOS backend |
| Screenshots | DWM capture or win-screenshot | xcap X11 capture | xcap macOS capture |
| Window handles | HWND hex string | X11 window ID hex string | PID:ref string |

## Role Mapping

Native accessibility roles mapped to Windows UIA control types for consistency:

**Windows**: Native UIA control types (Button, Edit, Menu, etc.)

**Linux AT-SPI2**:
- `push button` → `Button`
- `text` → `Edit`
- `menu` → `Menu`
- `menu item` → `MenuItem`
- `check box` → `CheckBox`

**macOS AX**:
- `AXButton` → `Button`
- `AXTextField` → `Edit`
- `AXMenu` → `Menu`
- `AXMenuItem` → `MenuItem`
- `AXCheckBox` → `CheckBox`

See `src/automation/{platform}/roles.rs` for complete mappings.

## Error Handling

Platform-specific errors wrapped in `OpsError`:

**Windows**: UIA COM errors, Win32 errors from `GetLastError`

**Linux**: D-Bus errors (AT-SPI2 unavailable), X11 protocol errors (connection failed)

**macOS**: Accessibility permission denied, AXUIElement query errors

All platform errors provide informative messages with remediation hints where applicable.

## Future Platform Support

### Wayland (Linux)

**Blockers**:
- Input simulation requires libei (Wayland input emulation protocol)
- enigo Wayland support incomplete
- Window enumeration requires compositor-specific protocols

**Workaround**: Use Xwayland (X11 compatibility layer). Most Wayland compositors support Xwayland, allowing X11-based automation.

**Timeline**: Deferred until libei and enigo support stabilizes.

### BSD

Not planned. Would require similar approach to Linux (X11 + AT-SPI2).
