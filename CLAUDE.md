# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Desktop CLI (`desktop-cli`) is a cross-platform desktop automation CLI optimized for LLM agents. It controls desktop applications through native accessibility APIs: UI Automation (Windows), AT-SPI2 (Linux), and Cocoa Accessibility (macOS). Written in Rust (edition 2021), licensed GPL-3.0-only.

## Build & Test Commands

```bash
cargo build                   # Debug build
cargo build --release         # Release build
cargo fmt                     # Format code
cargo fmt -- --check          # Check formatting
cargo clippy                  # Lint

# Tests
cargo test --lib              # Unit tests only
cargo test --test cross_platform_test   # Cross-platform tests

# E2E tests (platform-specific, require desktop/accessibility)
cargo test --test windows_e2e_test -- --test-threads=1
docker build -t desktop-cli-test . && docker run --rm --init desktop-cli-test  # Linux E2E
# macOS: unit tests only in CI (TCC permissions block headless E2E)
```

Windows E2E tests must run with `--test-threads=1` to avoid window enumeration race conditions. Linux E2E tests run inside Docker with Xvfb + D-Bus for AT-SPI2 access.

## Architecture

### Platform Dispatch (compile-time)

The codebase uses `#[cfg]` for zero-cost platform dispatch. There is no runtime branching:

```
main.rs (Clap CLI)
  → targeting/ (parse window queries: :1, notepad, title:X, hwnd:X, pid:X)
  → ops/ (DesktopPlatform trait, compile-time platform selection)
    → automation/{windows,linux,macos}/ (native API calls)
    → rpc/types.rs (serialization: UiaElement, PatternResult, QueryResult)
```

### Key Modules

- **`src/main.rs`** - CLI entry point, all Clap subcommand definitions
- **`src/ops/traits.rs`** - `DesktopPlatform` trait that all platforms implement
- **`src/ops/{windows,linux,macos}_ops.rs`** - Platform implementations wrapping automation layer
- **`src/automation/types.rs`** - Cross-platform types (WindowInfo, WindowRect, Action)
- **`src/automation/windows/uia/`** - UIA element tree, CSS-style selectors, enhanced query syntax, compact summaries
- **`src/automation/linux/atspi.rs`** - AT-SPI2 element tree (async, per-call tokio runtime)
- **`src/automation/macos/accessibility.rs`** - Cocoa AXUIElement tree
- **`src/targeting/`** - Window query parsing, resolution, and disambiguation
- **`src/executor/`** - Multi-step instruction execution with Gemini
- **`src/agent/`** - LLM-driven automation planning
- **`src/gemini/`** - Google Gemini API client with retry logic
- **`src/error.rs`** - DesktopCliError (CLI) and GeminiError types

### Design Invariants

- **Type normalization**: All platforms convert to common types (UiaElement, WindowInfo) before leaving the `automation/` layer. Consumers see consistent vocabulary.
- **Window handle opacity**: `hwnd` is a `String` everywhere outside platform modules. Only platform code parses it.
- **Role mapping**: Linux AT-SPI2 roles and macOS AX roles are mapped to UIA control type names for consistency.
- **Per-call async runtime (Linux)**: AT-SPI2 requires async; each operation creates a tokio runtime, blocks, and drops it (~1ms overhead vs. global runtime complexity).

### Test Infrastructure

- **`tests/cross_platform_test.rs`** - Selector parsing, serialization roundtrips, role mapping, QuickCheck property tests
- **`tests/windows_e2e_test.rs`** - Notepad/conhost window enumeration, UIA tree traversal
- **`tests/linux_e2e_test.rs`** - X11 window listing, AT-SPI2 tree traversal, pattern invocation
- **`tests/fixtures/gtk_test_app/`** - Custom C GTK3 app used as Linux E2E test target
- **`tests/common/mod.rs`** - Mock generators and test utilities

### Sub-module CLAUDE.md Files

Navigation indexes exist in `src/automation/CLAUDE.md`, `src/automation/{windows,linux,macos}/CLAUDE.md`, `src/ops/CLAUDE.md`, `docs/CLAUDE.md`, and `scripts/CLAUDE.md` for module-level guidance.

## CI

GitHub Actions (`.github/workflows/ci.yml`): matrix of Ubuntu (Docker E2E), Windows (native serial E2E), macOS (unit tests only). 30-minute timeout. Cargo caching enabled.

Documentation build (`.github/workflows/docs.yml`): generates tutorials from templates via Docker, builds mdBook site, checks for drift in generated files.
