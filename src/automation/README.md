# Automation Architecture

Cross-platform desktop automation with compile-time platform dispatch and normalized API surface.

## Architecture

```
                    CLI (main.rs)
                         |
                    ops/mod.rs (platform dispatch)
                         |
         +---------------+---------------+
         |               |               |
    windows_ops      linux_ops       macos_ops
         |               |               |
    automation/      automation/     automation/
    windows/         linux/          macos/
    - uia/           - atspi.rs      - accessibility.rs
    - window.rs      - window.rs     - window.rs
    - input.rs       - input.rs      - input.rs
```

Platform modules isolated by OS `cfg` flags. Each platform folder self-contained with no cross-platform dependencies at this layer. Trait abstraction lives in `ops/traits.rs`.

## Data Flow

```
User Request (hwnd/selector)
         |
         v
    Platform Dispatch (cfg-based compile-time)
         |
         v
    Window Resolution (X11/_NET_CLIENT_LIST, EnumWindows, CGWindowList)
         |
         v
    Element Tree (AT-SPI2, UIA, AXUIElement)
         |
         v
    Action Execution (enigo, SendInput, CGEvent)
         |
         v
    Result (UiaElement tree, PatternResult)
```

Each platform implements identical public API but uses platform-native libraries. Results normalized to common types before returning to ops layer.

## Why This Structure

**Platform isolation by cfg flags**: Compile-time selection, zero runtime overhead. Separate binaries per platform accepted in exchange for no branching cost.

**Common types in `types.rs`**: Consistent API surface. `WindowInfo`, `WindowRect`, `Action` shared across platforms. Platform-specific details (e.g., hwnd format) kept as opaque strings.

**`ops/` trait-based abstraction**: Testable, swappable platform implementations. CLI layer interacts only with `DesktopPlatform` trait, never platform-specific modules directly.

**Self-contained platform folders**: Contributors need only platform expertise, not cross-platform knowledge. Each folder builds independently with platform-specific dependencies.

## Invariants

**hwnd format**: Always `"0x{hex}"` string representation regardless of platform's native handle type. Windows uses actual HWND, Linux uses X11 Window ID, macOS uses PID+element reference. All code outside platform modules treats hwnd as opaque string.

**UiaElement.control_type**: Normalized to UIA names (`Button`, `Edit`, `Text`, etc.) not platform names. Linux AT-SPI2 roles mapped via `roles::map_role()`, macOS AX roles mapped similarly. Consumers see consistent vocabulary.

**Async wrapping**: All async operations (AT-SPI2 requires async) wrapped in sync API. Per-call tokio runtime created, blocks on result, runtime dropped. Library consumers never need async runtime in their code.

**Coordinate system**: All coordinates in pixels relative to window origin. DPI scaling handled internally per platform. Windows uses physical pixels, Linux uses logical pixels with X11 scaling, macOS uses Cocoa coordinates.

## Tradeoffs

**Per-call tokio runtime vs global**: Chose per-call for simplicity. Accepts ~1ms overhead per AT-SPI2 operation to avoid managing global async runtime lifetime and thread safety. Linux E2E tests confirm overhead acceptable for automation workloads.

**Compile-time platform dispatch vs runtime**: Chose compile-time for zero overhead. Accepted separate binaries per platform (increases build/release complexity) in exchange for no runtime branching or vtable indirection.

**Real test apps vs system apps**: Chose real test apps (GTK fixture, Notepad) for consistency. Accepted build complexity (Docker for Linux, process management for Windows) in exchange for reproducible element trees across environments.

**Normalization layer location**: Chose normalization at automation module boundary (before returning to ops layer). Accepted per-platform role mapping code duplication in exchange for clean separation and easier platform-specific debugging.

## Platform-Specific Details

### Windows
- **Automation API**: UI Automation (UIAutomation crate)
- **Window enumeration**: `EnumWindows` Win32 API
- **Input simulation**: `SendInput` Win32 API
- **Screenshots**: DWM API for composited windows
- **Coordinates**: Physical pixels, DPI-aware

### Linux
- **Automation API**: AT-SPI2 (async via atspi crate)
- **Window enumeration**: X11 `_NET_CLIENT_LIST` property
- **Input simulation**: enigo crate (X11 XTest extension)
- **Screenshots**: xcap crate (X11 capture)
- **Coordinates**: Logical pixels with X11 scaling
- **Runtime**: Per-call tokio runtime blocks on async AT-SPI2 calls

### macOS
- **Automation API**: Cocoa Accessibility (AXUIElement)
- **Window enumeration**: CGWindowList API
- **Input simulation**: CGEvent API
- **Screenshots**: xcap crate (Core Graphics capture)
- **Coordinates**: Cocoa coordinate system (origin at bottom-left)
- **Permissions**: Requires Accessibility permissions, checked via `permissions.rs`

## Testing Strategy

See `tests/README.md` for comprehensive testing approach per platform.
