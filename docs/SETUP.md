# Platform Setup Guide

Platform-specific setup instructions for desktop automation.

## Windows

### Requirements

- Windows 7 or later (Vista+ has UI Automation support)
- No additional dependencies required

### Setup

1. Build or download Windows binary:
   ```bash
   cargo build --release --target x86_64-pc-windows-msvc
   ```

2. Run directly. No configuration needed:
   ```bash
   desktop-cli windows
   ```

### Permissions

No permissions required. UI Automation is available to all processes by default.

### Troubleshooting

**Issue**: Operations fail on specific applications

**Cause**: Some applications disable UI Automation or use custom controls

**Solution**: Use screenshot-based automation or application-specific APIs if available

---

## Linux (X11)

### Requirements

- X11 server (Xorg)
- AT-SPI2 D-Bus service
- Standard on GNOME, KDE, XFCE desktop environments

### Checking Dependencies

Verify AT-SPI2 is running:
```bash
# Check if AT-SPI2 bus is available
busctl --user status org.a11y.Bus

# List accessible applications
busctl --user tree org.a11y.atspi.Registry
```

Verify X11 connection:
```bash
echo $DISPLAY  # Should show :0 or similar
xdpyinfo       # Should show X server information
```

### Setup

1. Install dependencies (if not already present):

   **Ubuntu/Debian**:
   ```bash
   sudo apt-get install at-spi2-core libx11-dev
   ```

   **Fedora/RHEL**:
   ```bash
   sudo dnf install at-spi2-core libX11-devel
   ```

   **Arch**:
   ```bash
   sudo pacman -S at-spi2-core libx11
   ```

2. Build Linux binary:
   ```bash
   cargo build --release --target x86_64-unknown-linux-gnu
   ```

3. Run:
   ```bash
   desktop-cli windows
   ```

### Permissions

No special permissions required. Uses user session D-Bus.

### Wayland Users

**Desktop-cli requires X11.** If running Wayland:

**Option 1: Switch to X11 session** (recommended)
- Log out
- Select X11 session at login screen (usually "GNOME on Xorg" or similar)
- Log back in

**Option 2: Use Xwayland**
- Most Wayland compositors run Xwayland automatically
- Set environment variable:
  ```bash
  export WAYLAND_DISPLAY=
  ```
- Applications will run under Xwayland (X11 compatibility layer)
- Limitations: May not work for all Wayland-native applications

### Troubleshooting

**Issue**: `AT-SPI2 service unavailable`

**Cause**: AT-SPI2 D-Bus service not running

**Solution**:
```bash
# Check if service is running
systemctl --user status at-spi-dbus-bus

# Start if not running
systemctl --user start at-spi-dbus-bus

# Enable automatic start
systemctl --user enable at-spi-dbus-bus
```

**Issue**: `X11 connection failed`

**Cause**: Not running X11 or `DISPLAY` not set

**Solution**:
```bash
# Verify DISPLAY variable
echo $DISPLAY

# If empty, set it (usually :0)
export DISPLAY=:0

# Check if X server is running
ps aux | grep Xorg
```

**Issue**: Element tree operations fail on specific applications

**Cause**: Application does not expose AT-SPI2 interface

**Solution**: Some applications (especially non-GTK/Qt) may not support AT-SPI2. Use screenshot-based automation or coordinate-based input as fallback.

---

## macOS

⚠ **macOS support is planned but not yet implemented. Current release provides stub implementations only.**

All automation operations on macOS will return "Platform not supported on this system" errors. Use Windows or Linux (X11) for functional automation.

### Requirements (Planned)

- macOS 10.15+ (Catalina or later)
- No additional dependencies required

### Setup (When Implemented)

1. Build macOS binary:
   ```bash
   # Intel Macs
   cargo build --release --target x86_64-apple-darwin

   # Apple Silicon Macs
   cargo build --release --target aarch64-apple-darwin
   ```

2. Grant Accessibility permission (required):

   a. Run desktop-cli. First automation operation will fail with permission error.

   b. Open System Preferences → Security & Privacy → Privacy → Accessibility

   c. Click lock icon to make changes (requires admin password)

   d. Add your terminal emulator (Terminal.app, iTerm2, etc.) or the application running desktop-cli to the allowed list:
      - Click `+` button
      - Navigate to `/Applications/Utilities/Terminal.app` (or your terminal)
      - Select and add

   e. Ensure checkbox next to the application is checked

   f. Restart terminal and retry operation

### Alternative: Running from Scripts

If running desktop-cli from a script or application (not terminal):

1. Add the script executor to Accessibility allowed list instead of terminal
2. For example, if running from Python script: add Python to allowed list
3. For compiled applications: add your application binary to allowed list

### Permissions

**Required**: Accessibility permission for element tree queries and input simulation

**Not required**: Window enumeration works without permission (limited to window list only)

### Graceful Degradation

Operations gracefully degrade based on permission status:

| Operation | Without Permission | With Permission |
|-----------|-------------------|-----------------|
| List windows | Works (title, PID, rect) | Works |
| Get window info | Works | Works |
| Dump element tree | Error with remediation hint | Works |
| Find elements | Error with remediation hint | Works |
| Input simulation | Error with remediation hint | Works |
| Screenshots | Works | Works |

### Troubleshooting

**Issue**: `Accessibility permission required`

**Cause**: Application not granted accessibility permission

**Solution**: Follow setup steps above to grant permission. Must restart terminal after granting.

**Issue**: Permission granted but operations still fail

**Cause**: macOS may cache permission state

**Solution**:
```bash
# Restart terminal completely (quit and reopen)
# Or restart macOS

# Verify permission via System Information
/usr/libexec/PlistBuddy -c "Print :TCC:kTCCServiceAccessibility" \
  ~/Library/Application\ Support/com.apple.TCC/TCC.db
```

**Issue**: Element tree queries return empty results

**Cause**: Application may not expose Accessibility interface

**Solution**: Not all applications support Cocoa Accessibility. Native macOS apps generally support it; some cross-platform apps may not. Use screenshot-based automation as fallback.

**Issue**: Coordinates incorrect on Retina displays

**Cause**: DPI scaling issue

**Solution**: This should be handled automatically. If experiencing issues, file a bug report with display information (`system_profiler SPDisplaysDataType`).

---

## Cross-Platform Testing

For testing across platforms without access to all systems:

1. **Use CI matrix**: GitHub Actions supports Windows, Linux, macOS runners
   ```yaml
   strategy:
     matrix:
       os: [ubuntu-latest, windows-latest, macos-latest]
   ```

2. **Virtual machines**:
   - Windows: Use free Windows VMs from Microsoft
   - Linux: Use any Linux distro in VM or Docker
   - macOS: Requires Mac hardware (licensing restriction)

3. **Remote access**: Use cloud development environments with platform-specific runners

---

## Development Environment Setup

### Building for All Platforms

Install cross-compilation targets:
```bash
# Linux target
rustup target add x86_64-unknown-linux-gnu

# Windows target
rustup target add x86_64-pc-windows-msvc

# macOS targets
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

### Platform-Specific Dependencies

Dependencies are target-specific in `Cargo.toml`. Only relevant platform dependencies compile for each target.

**Windows**:
- `windows` crate (Win32 APIs)
- `uiautomation` crate
- `win-screenshot` crate

**Linux**:
- `atspi` crate (AT-SPI2 D-Bus)
- `x11rb` crate (X11 protocol)
- `enigo` crate (input simulation)
- `xcap` crate (screenshots)

**macOS**:
- `accessibility-sys` crate (Cocoa Accessibility)
- `enigo` crate (input simulation)
- `xcap` crate (screenshots)

### Testing on Each Platform

**Unit tests**: Run on host platform only (platform-specific code isolated)
```bash
cargo test
```

**Integration tests**: Require real automation APIs (platform-specific)
```bash
# Linux: Requires X11 + AT-SPI2
cargo test --test linux_integration_test

# macOS: Requires accessibility permission
cargo test --test macos_integration_test

# Windows: No special requirements
cargo test --test uia_integration_test
```

**CI matrix**: Automate cross-platform testing
- Set up runners for each platform
- Grant necessary permissions (macOS accessibility in CI runner setup)
- Run platform-specific tests on matching runner

---

## Known Platform-Specific Issues

### Windows
- Some legacy applications may not expose UIA interface properly
- DWM screenshot method requires DWM enabled (disabled in Windows Server without Desktop Experience)

### Linux
- Wayland compositors not supported (X11 only)
- Some applications (Electron, Firefox with certain configs) may have limited AT-SPI2 support
- Xwayland compatibility varies by compositor

### macOS
- Permission prompt cannot be automated (macOS security restriction)
- Some cross-platform applications may have limited Accessibility support
- Retina coordinate handling tested but edge cases may exist

---

## Support Matrix Summary

| Feature | Windows | Linux (X11) | Linux (Wayland) | macOS |
|---------|---------|-------------|-----------------|-------|
| Window enumeration | ✓ | ✓ | ✗ | ✗ (stub) |
| Element tree | ✓ | ✓ | ✗ | ✗ (stub) |
| Input simulation | ✓ | ✓ | ✗ | ✗ (stub) |
| Screenshots | ✓ | ⚠ (monitor only) | ✗ | ✗ (stub) |
| Pattern invocation | ✓ | ✓ | ✗ | ✗ (stub) |
| Summary/query | ✓ | ✓ | ✗ | ✗ (stub) |
| No setup required | ✓ | ✗ (needs AT-SPI2) | ✗ | ✗ (stub) |
| Runs in CI | ✓ | ✓ | ✗ | ✗ (stub) |
