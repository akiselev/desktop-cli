# Automation Modules

Navigation index for platform-specific automation implementations.

| What | When |
|------|------|
| `types.rs` | Reference cross-platform types (WindowInfo, WindowRect, Action) |
| `windows/` | Implement Windows automation (UIA, SendInput, screenshots) |
| `linux/` | Implement Linux automation (AT-SPI2, X11, enigo, screenshots) |
| `macos/` | Implement macOS automation (Cocoa Accessibility, CGEvent, screenshots) |
| `mod.rs` | Understand compile-time platform module selection via cfg flags |
| `README.md` | Understand architecture, platform differences, and design invariants |
