# Test Strategy

Platform-specific testing strategies balancing coverage with CI feasibility.

## Test Architecture

```
tests/
  cross_platform_test.rs   # Unit tests, property tests, role mapping
  windows_e2e_test.rs      # Windows E2E with Notepad/conhost
  linux_e2e_test.rs        # Linux E2E with GTK test fixture
  common/mod.rs            # Mock generators, shared utilities
  fixtures/
    gtk_test_app/          # C GTK3 app with accessible elements
```

## Platform-Specific Strategies

### Linux: Docker E2E with Real GTK App

**Environment**: Docker container with Xvfb + D-Bus + GTK3
**Test app**: Custom GTK fixture (`tests/fixtures/gtk_test_app/`) with known elements
**Rationale**: AT-SPI2 requires D-Bus session, X11 display, and GTK accessibility bridge. Docker provides consistent environment across CI and local development.

**Setup requirements**:
- Xvfb for headless X11 server
- D-Bus session bus for AT-SPI2 communication
- GTK3 with `gail:atk-bridge` modules loaded
- `GTK_A11Y=atspi` environment variable

**Test coverage**:
- X11 window listing (`test_x11_window_list`)
- AT-SPI2 tree traversal (`test_dump_tree`)
- Element finding by name (`test_find_element`)
- Pattern invocation (`test_invoke_pattern`)
- Timeout handling (`test_timeout`)
- Graceful unsupported pattern handling (`test_pattern_unsupported`)

**Known limitations**: Timeouts longer than Windows due to AT-SPI2 async overhead. Tests verify functionality, not performance.

### Windows: Native E2E with Notepad

**Environment**: Native Windows, serial execution (`--test-threads=1`)
**Test app**: Notepad.exe (or conhost.exe fallback)
**Rationale**: UIA works reliably with native Windows apps. Notepad universally available, conhost fallback for minimal environments.

**Setup requirements**:
- Native Windows (no virtualization needed)
- `--test-threads=1` to prevent window enumeration race conditions
- Process cleanup verification via `tasklist`

**Test coverage**:
- Window listing (`test_window_listing`)
- UIA tree traversal with Edit control verification (`test_uia_tree`)
- Element finding by control type (`test_find_element`)
- Process cleanup after drop (`test_cleanup`)

**Why Notepad**: Universal availability, stable UIA tree structure, predictable Edit control. Conhost fallback ensures CI compatibility.

**Why serial execution**: Window enumeration during parallel tests causes race conditions. Multiple test processes spawning/killing windows simultaneously interferes with window list consistency.

### macOS: Unit Tests Only

**Environment**: Native macOS
**Test coverage**: Unit tests only (role mapping, permissions check, window listing if permissions granted)
**Rationale**: E2E tests skipped in CI due to TCC permissions requirement. macOS requires user approval for Accessibility access, incompatible with headless CI.

**Setup requirements**:
- Accessibility permissions granted manually for test runner
- Tests check permissions and skip if not granted

**Test coverage**:
- Role mapping (`test_role_mapping_macos`)
- Permissions check (`test_permissions_check_macos`)
- Window listing (runs if permissions granted, prints result)

**Known limitations**: No E2E automation in CI. Manual testing required for full coverage.

## Cross-Platform Tests

**File**: `cross_platform_test.rs`
**Coverage**: Platform-agnostic logic
- Window selector parsing (index, executable, title, hwnd, pid)
- `WindowInfo` serialization roundtrip
- Role mapping (platform-conditional)
- Mock window generation

**Property tests**: QuickCheck-based testing for selector parsing edge cases.

## Test Fixtures

### GTK Test App (`tests/fixtures/gtk_test_app/`)

C GTK3 application with known accessible element tree:
- Window with title "AT-SPI2 Test App"
- Button with name "Test Button" (supports invoke action)
- Entry with name "Test Entry"
- Label with name "Test Label" (does not support invoke)

Built during Docker image creation. Binary path: `/app/tests/fixtures/gtk_test_app/gtk_test_app`

**Why custom app**: System apps have unpredictable element trees varying by distribution. Custom app guarantees consistent structure for assertions.

### Notepad (Windows)

System application, no custom fixture needed. Predictable element tree:
- Window with class "Notepad"
- Edit control (always present)

Fallback to `conhost.exe` if Notepad unavailable (minimal Windows environments).

## Test Execution

### Local Development

**Linux**: `docker build -t desktop-cli-test . && docker run desktop-cli-test`
**Windows**: `cargo test --test windows_e2e_test -- --test-threads=1`
**macOS**: `cargo test --test cross_platform_test` (E2E requires manual permissions)

### CI

**Linux**: Docker-based, full E2E coverage
**Windows**: Native runner, full E2E coverage with `--test-threads=1`
**macOS**: Unit tests only, E2E skipped due to TCC

## Common Test Utilities

`tests/common/mod.rs` provides:
- `mock_window_info()`: Generate mock `WindowInfo` for unit tests
- `generate_mock_windows()`: Generate list of mock windows (Firefox, Terminal, VSCode, Notepad, Chrome)

Used by cross-platform tests to verify serialization, selector matching, etc. without requiring live windows.

## Tradeoffs

**Docker overhead vs consistency**: Chose Docker for Linux to guarantee D-Bus + X11 + GTK environment. Accepted slower local test iteration in exchange for CI/local parity.

**Serial execution vs speed**: Chose `--test-threads=1` for Windows to prevent enumeration races. Accepted slower test execution in exchange for reliability.

**Custom GTK app vs system apps**: Chose custom app for predictable element tree. Accepted build complexity in exchange for assertion stability across distributions.

**macOS E2E skipped in CI**: Chose to skip rather than bypass TCC. Accepted reduced CI coverage in exchange for not compromising security model. Manual testing documented as requirement.
