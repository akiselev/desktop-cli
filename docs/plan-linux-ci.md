# Linux CI/CD with E2E Test Suite

## Overview

Create Docker infrastructure and GitHub Actions CI/CD pipeline to test Linux AT-SPI2 automation. Uses multi-stage Dockerfile (rust builder + ubuntu runtime) with Xvfb/D-Bus for headless testing. Includes a simple GTK test application for predictable E2E test targets.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|-----------------|
| Multi-stage Dockerfile | Smaller final image -> faster CI pulls -> rust:1.75 has build deps, ubuntu:22.04 has minimal runtime -> separation reduces attack surface |
| GTK test app over Firefox | Firefox adds ~500MB + complexity -> GTK app is <10MB and fully controllable -> predictable element IDs for reliable tests |
| Xvfb for headless X11 | AT-SPI2 requires X11 DISPLAY -> Xvfb provides virtual framebuffer -> no GPU needed in CI |
| dbus-run-session wrapper | AT-SPI2 needs D-Bus session bus -> dbus-run-session spawns isolated session -> clean state per test run |
| Property + example tests | Property tests cover edge cases efficiently -> example tests document expected behavior explicitly -> both complement each other |
| E2E tests via #[cfg(target_os)] | Compile-time gates prevent test compilation on non-Linux -> simpler than runtime checks -> cargo test --all just works cross-platform |
| 5s window detection timeout | AT-SPI2 registration typically <1s -> 5s covers slow CI runners -> timeout failure is definitive not flaky -> avoids CI hangs |
| Selector syntax: element matching in atspi.rs | Linux atspi.rs uses simple matching: #name matches element.name, .class matches class_name, bare text matches control_type or name -> window targeting uses targeting/parser.rs (:1, title:, hwnd:) which is separate concern -> element matching defined in atspi.rs:find_matching_elements |
| Docker apt packages | at-spi2-core: AT-SPI2 registry daemon -> libatk-bridge2.0-0: GTK-to-AT-SPI2 bridge -> libgtk-3-0: GTK3 runtime -> xvfb: virtual X11 -> dbus-x11: D-Bus session integration |
| ENTRYPOINT: xvfb-run -a dbus-run-session | xvfb-run -a auto-selects display -> dbus-run-session spawns session bus -> cargo test runs in this environment -> single-threaded tests avoid races |
| Cache key: Cargo.lock hash | Cargo.lock uniquely identifies dependency versions -> hash changes on dependency update -> stale cache causes build failures not silent bugs -> conservative invalidation preferred |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|--------------|
| Docker Compose multi-container | Overkill for single test app + runner -> adds orchestration complexity -> single container sufficient |
| Firefox in container | 500MB+ image size -> unpredictable UI changes across versions -> harder to maintain stable test selectors |
| Wayland instead of X11 | Current codebase is X11-only (x11rb) -> would require parallel implementation -> X11 sufficient for CI |
| GitHub Actions services | AT-SPI2 requires same process namespace as X11 -> services run in separate containers -> wouldn't work |

### Constraints & Assumptions

- Ubuntu 22.04 LTS as base (stable AT-SPI2 packages)
- Rust stable (1.75+) for build
- GitHub Actions ubuntu-latest runners
- AT-SPI2 accessibility must be enabled via gsettings
- GTK3 for test app (better AT-SPI2 support than GTK4)

### Known Risks

| Risk | Mitigation | Anchor |
|------|------------|--------|
| AT-SPI2 service not starting | Explicit at-spi2-core package + gsettings enable in Dockerfile | N/A - new file |
| Xvfb display race | Wait for Xvfb socket before tests | N/A - new file |
| D-Bus session isolation | dbus-run-session creates clean session per run | N/A - new file |
| GTK app not registering with AT-SPI2 | GTK_MODULES=gail:atk-bridge env var forces registration | N/A - new file |

## Invisible Knowledge

### Architecture

```
GitHub Actions Runner
        |
        v
+------------------+
|  Docker Build    |
|  (multi-stage)   |
+------------------+
        |
        v
+------------------+
|  Test Container  |
|  +------------+  |
|  | Xvfb       |  |
|  | D-Bus      |  |
|  | AT-SPI2    |  |
|  | GTK App    |  |
|  | Test Runner|  |
|  +------------+  |
+------------------+
```

### Data Flow

```
cargo test (unit/property)
        |
        v
xvfb-run + dbus-run-session
        |
        v
GTK test app spawns --> AT-SPI2 registers app
        |
        v
E2E tests --> AT-SPI2 --> Element tree queries
        |
        v
Test assertions on found elements
```

### Why This Structure

- Single Dockerfile handles both build and test runtime
- GTK test app lives in tests/fixtures/ - only used for testing, not production
- E2E tests separate from unit tests to allow different runtime requirements
- GitHub workflow uses matrix for future cross-platform expansion

### Invariants

- AT-SPI2 must be running before tests start
- DISPLAY must be set and Xvfb must be ready
- GTK_MODULES must include atk-bridge for accessibility
- Test app must have predictable widget IDs

## Milestones

### Milestone 1: Commit and Push Current Changes

**Files**: (existing uncommitted changes)

**Requirements**:
- Commit all uncommitted changes to next-steps branch
- Push to remote origin

**Acceptance Criteria**:
- Git status shows clean working tree
- Remote has latest commits

**Tests**: N/A - git operation

**Code Intent**:
- Git add all changed files
- Commit with descriptive message about AT-SPI2 PID matching fix
- Push to origin next-steps

### Milestone 2: Linux Dockerfile

**Files**:
- `Dockerfile`
- `.dockerignore`

**Flags**: `error-handling`

**Requirements**:
- Multi-stage build: rust:1.75 builder, ubuntu:22.04 runtime
- Install AT-SPI2, X11, D-Bus, GTK3 dependencies
- Enable accessibility via gsettings
- Entry point runs tests with Xvfb + D-Bus session

**Acceptance Criteria**:
- `docker build .` succeeds
- Container can run `cargo test`
- AT-SPI2 service accessible inside container

**Tests**:
- **Test type**: manual verification via docker build
- **Scenarios**:
  - Build completes without errors
  - Runtime has all required libraries

**Code Intent**:
- Builder stage: FROM rust:1.75, copy source, cargo build --release, cargo build --release --tests
- Runtime stage: FROM ubuntu:22.04
- apt-get install (Decision: "Docker apt packages"):
  - at-spi2-core: AT-SPI2 registry service
  - libatk1.0-0, libatk-bridge2.0-0: ATK accessibility bridge
  - libgtk-3-0: GTK3 runtime for test app
  - xvfb, dbus-x11: Virtual X11 and D-Bus integration
  - libx11-6, libxcb1: X11 client libraries
  - gcc, make, pkg-config, libgtk-3-dev: Build tools for GTK test app
- gsettings set org.gnome.desktop.interface toolkit-accessibility true
- ENTRYPOINT (Decision: "ENTRYPOINT: xvfb-run -a dbus-run-session"):
  `ENTRYPOINT ["xvfb-run", "-a", "dbus-run-session", "--", "cargo", "test", "--", "--test-threads=1"]`
- ENV GTK_MODULES=gail:atk-bridge (forces AT-SPI2 registration)

### Milestone 3: GTK Test Application

**Files**:
- `tests/fixtures/gtk_test_app/main.c`
- `tests/fixtures/gtk_test_app/Makefile`

**Flags**: `needs-rationale`

**Requirements**:
- Simple GTK3 window with labeled widgets
- Button with accessible name "Test Button"
- Text entry with accessible name "Test Entry"
- Label with text "Test Label"
- Window title "AT-SPI2 Test App"

**Acceptance Criteria**:
- Compiles with gcc and gtk3 pkg-config
- Window displays when run with DISPLAY set
- Widgets appear in AT-SPI2 tree with correct names

**Tests**:
- **Test files**: tests/linux_e2e_test.rs (in Milestone 4)
- **Test type**: e2e
- **Scenarios**:
  - Window appears in AT-SPI2 registry
  - Button element found by name
  - Entry element found by name

**Code Intent**:
- GTK3 application with GtkWindow, GtkBox container
- GtkButton with gtk_widget_set_name and atk_object_set_name for "Test Button"
- GtkEntry with accessible name "Test Entry"
- GtkLabel with text "Test Label"
- Makefile with `pkg-config --cflags --libs gtk+-3.0`

### Milestone 4: E2E Test Suite

**Files**:
- `tests/linux_e2e_test.rs`
- `tests/common/linux_helpers.rs`

**Flags**: `conformance`, `error-handling`

**Requirements**:
- E2E tests for Linux AT-SPI2 functionality
- Test window enumeration finds GTK test app
- Test dump-tree returns elements from test app
- Test find-element locates specific widgets
- Tests spawn GTK app, wait for AT-SPI2 registration, run assertions

**Acceptance Criteria**:
- All e2e tests pass when run inside Docker container
- Tests skip gracefully when not on Linux or no X11

**Tests**:
- **Test files**: tests/linux_e2e_test.rs
- **Test type**: e2e + property-based
- **Backing**: user-specified (real AT-SPI2)
- **Scenarios**:
  - Normal: spawn app, find by title, dump tree
  - Edge: app running but elements not accessible (validates atk_object_set_name correctness)
  - Edge: app not running, no accessibility service
  - Error: invalid window ID, malformed selector (empty, "#" alone, "##foo")

**Code Intent**:
- All tests gated with #[cfg(target_os = "linux")] (Decision: "E2E tests via #[cfg(target_os)]")
- Helper function spawn_gtk_test_app() -> std::process::Child
  - Sets DISPLAY from env, GTK_MODULES=gail:atk-bridge
  - Spawns tests/fixtures/gtk_test_app/gtk_test_app binary
- Helper function wait_for_window(title: &str, timeout: Duration) -> Result<String>
  - Polls list_windows() every 100ms
  - Returns window hwnd when found
  - Timeout: 5 seconds (Decision: "5s window detection timeout")
  - Returns error if timeout exceeded
- Test test_window_enumeration: spawn app -> wait_for_window("AT-SPI2 Test App") -> assert found
- Test test_dump_tree: spawn app -> wait -> dump_tree(hwnd) -> assert contains "Test Button" element
- Test test_find_element: spawn app -> wait -> find_element(hwnd, "#Test Button") -> assert found
- Helper function verify_gtk_app_elements(hwnd: &str) -> Result<()>
  - Calls dump_tree and verifies expected elements present: "Test Button", "Test Entry", "Test Label"
  - Fails fast with descriptive error if element missing (distinguishes from "app not registered")
- Property test for element matching (Decision: "Selector syntax: element matching in atspi.rs"):
  - Make element_matches_selector pub(crate) in atspi.rs to enable testing from tests/ directory
  - Property: For all selector strings (domain: valid prefixes #/., alphanumeric + underscore, length 0-100), element_matches_selector returns bool without panic
  - Add input validation to element_matches_selector: return false if selector is empty or contains only prefix char
  - Edge cases: empty string, single "#", single ".", "##foo", very long strings
  - NOTE: targeting::parser is for WINDOW queries (:1, title:, hwnd:), not element selection

### Milestone 5: GitHub Actions Workflow

**Files**:
- `.github/workflows/linux-ci.yml`

**Flags**: `needs-rationale`

**Requirements**:
- Workflow triggers on push and pull_request
- Build and test Linux target
- Use Docker container for e2e tests
- Cache cargo dependencies
- Matrix for future expansion (rust versions)

**Acceptance Criteria**:
- Workflow runs on push to any branch
- Unit tests pass
- E2E tests pass in container
- Clear failure messages on test failures

**Tests**:
- **Test type**: CI verification
- **Scenarios**:
  - Push triggers workflow
  - Tests complete within timeout
  - Failures reported clearly

**Code Intent**:
- Workflow name: "Linux CI"
- Triggers: push (all branches), pull_request (master)
- Jobs:
  1. `unit-tests`: runs-on ubuntu-latest, cargo test --lib --no-default-features
  2. `e2e-tests`: runs-on ubuntu-latest
     - docker build -t desktop-cli-test .
     - docker run desktop-cli-test
- Caching (Decision: "Cache key: Cargo.lock hash"):
  - actions/cache@v4
  - path: ~/.cargo/registry, ~/.cargo/git, target
  - key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
  - restore-keys: ${{ runner.os }}-cargo-
- Environment: RUST_BACKTRACE=1, CARGO_TERM_COLOR=always
- Timeout: 30 minutes per job

### Milestone 6: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `.github/CLAUDE.md`
- `tests/fixtures/CLAUDE.md`
- `tests/fixtures/gtk_test_app/README.md`

**Requirements**:
- CLAUDE.md indexes for new directories
- README for GTK test app explaining purpose and usage

**Acceptance Criteria**:
- CLAUDE.md files use tabular format
- README explains how to build and run GTK test app
- All new files indexed appropriately

## Milestone Dependencies

```
M1 (commit) --> M2 (Dockerfile) --> M5 (GitHub Actions)
                     |
                     v
                M3 (GTK App) --> M4 (E2E Tests) --> M5
```

Wave 1: M1 (sequential - must commit first)
Wave 2: M2, M3 (parallel - no dependencies between them)
Wave 3: M4 (depends on M2, M3)
Wave 4: M5 (depends on M2, M4)
Wave 5: M6 (documentation, after implementation)
