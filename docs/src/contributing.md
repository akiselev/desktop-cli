# Contributing

Contributions are welcome! desktop-cli is a cross-platform project with opportunities for improvement on all platforms.

## Development Setup

### Prerequisites

- **Rust toolchain**: Install via [rustup](https://rustup.rs/)
- **Platform-specific dependencies**:
  - **Linux**: `libxdo-dev` for input simulation
  - **Windows**: No extra dependencies
  - **macOS**: Xcode Command Line Tools (optional)

### Clone and Build

```bash
git clone https://github.com/akiselev/desktop-cli.git
cd desktop-cli
cargo build
```

### Platform-Specific Build

desktop-cli uses compile-time platform selection via `#[cfg]` attributes. Each platform has its own module:

```
src/automation/
├── windows/    # Windows UIA implementation
├── linux/      # Linux AT-SPI2 implementation
└── macos/      # macOS Accessibility implementation
```

When you run `cargo build`, only the code for your current platform is compiled.

## Testing

### Unit Tests

Run unit tests (tests marked with `#[test]` in library code):

```bash
cargo test --lib
```

### Integration Tests

Integration tests (`tests/`) require platform-specific setup:

#### Linux E2E Tests

Linux tests run in Docker to provide a consistent X11 + AT-SPI2 environment:

```bash
# Build the test container
docker build -t desktop-cli-test -f Dockerfile .

# Run E2E tests
docker run --rm \
  --init \
  desktop-cli-test \
  /bin/bash -c "xvfb-run -a --server-args='-screen 0 1280x1024x24' ./tests/linux_e2e_test"
```

The Docker setup ensures:
- X11 display via Xvfb
- AT-SPI2 bus running
- GTK test application available
- Consistent environment across machines

#### Windows E2E Tests

Windows tests use native applications (Notepad):

```powershell
cargo test --test windows_e2e_test
```

Make sure Notepad is installed (it's built into Windows).

#### macOS E2E Tests

macOS tests require:
1. Accessibility permissions granted to your terminal
2. Native test applications available

```bash
cargo test --test macos_e2e_test
```

## Code Style

desktop-cli follows standard Rust conventions:

```bash
# Format code
cargo fmt

# Check for common issues
cargo clippy

# Check formatting without modifying
cargo fmt -- --check
```

**Important style notes:**

- Use `#[cfg(target_os = "...")]` for platform-specific code
- Keep platform modules self-contained (no cross-platform imports at the automation layer)
- Normalize platform-specific types (control types, coordinates, etc.) before returning to the `ops/` layer
- Add doc comments for public APIs
- Use descriptive error messages with context

## Architecture

### High-Level Structure

```
src/
├── main.rs              # CLI entry point
├── ops/                 # Platform abstraction traits
│   ├── traits.rs        # DesktopPlatform trait
│   ├── windows_ops.rs   # Windows implementation
│   ├── linux_ops.rs     # Linux implementation
│   └── macos_ops.rs     # macOS implementation
└── automation/          # Platform-specific automation
    ├── types.rs         # Cross-platform types
    ├── windows/         # Windows UIA
    ├── linux/           # Linux AT-SPI2
    └── macos/           # macOS Accessibility
```

### Key Principles

1. **Platform Isolation**: Each `automation/{platform}/` directory is self-contained
2. **Trait Abstraction**: `ops/` layer uses traits to abstract platform differences
3. **Compile-Time Dispatch**: `#[cfg]` attributes select platform at compile time (zero runtime overhead)
4. **Type Normalization**: Platform-specific types converted to common types before leaving `automation/` layer

### Example: Adding a New Feature

To add a feature across all platforms:

1. **Add to trait** (`ops/traits.rs`):
   ```rust
   trait DesktopPlatform {
       fn new_feature(&self) -> Result<Data>;
   }
   ```

2. **Implement per platform**:
   - `ops/windows_ops.rs`: Call `automation/windows/new_feature.rs`
   - `ops/linux_ops.rs`: Call `automation/linux/new_feature.rs`
   - `ops/macos_ops.rs`: Call `automation/macos/new_feature.rs`

3. **Add CLI command** (`main.rs`):
   ```rust
   #[derive(Subcommand)]
   enum Command {
       NewFeature { /* args */ },
   }
   ```

## Pull Request Guidelines

### Before Submitting

1. **Test on your platform**: Run unit tests and integration tests
2. **Format code**: Run `cargo fmt`
3. **Check for issues**: Run `cargo clippy`
4. **Document**: Add doc comments for new public APIs

### PR Description

Include:

1. **What**: Brief description of the change
2. **Why**: Motivation or issue reference
3. **Testing**: What you tested and on which platform(s)
   - Example: "Tested on Linux (Ubuntu 22.04) with GTK apps"
   - Example: "Tested on Windows 11 with Notepad and VS Code"

### Platform Testing

We understand not everyone has access to all platforms. In your PR:

- **Explicitly state which platforms you tested on**
- If you can't test on a platform, note: "Unable to test on macOS/Windows/Linux"
- Maintainers will test on other platforms before merging

Example PR description:

```markdown
## Summary
Add support for keyboard shortcuts with modifiers

## Testing
- ✅ Tested on Linux (Ubuntu 22.04): Works with GTK apps
- ✅ Tested on Windows 11: Works with Notepad, VS Code
- ❌ Unable to test on macOS (no access to Mac)

## Changes
- Added `send_keys_with_modifiers()` to trait
- Implemented on Windows using SendInput
- Implemented on Linux using enigo
- Added macOS stub (needs testing)
```

## Areas for Contribution

### High Priority

- **macOS support**: Complete feature parity with Windows/Linux
- **Documentation**: More examples and tutorials
- **Test coverage**: More integration tests
- **Error handling**: Better error messages with actionable advice

### Good First Issues

- Add more selector syntax tests
- Improve CLI help text
- Add examples for common workflows
- Platform-specific bug fixes

### Platform-Specific

- **Windows**: Improve element filtering heuristics for complex applications
- **Linux**: Better Wayland support (currently X11-only)
- **macOS**: Implement screenshot support, improve permission handling

## Community

- **Issues**: Report bugs or request features on [GitHub Issues](https://github.com/akiselev/desktop-cli/issues)
- **Discussions**: Ask questions or share ideas in [GitHub Discussions](https://github.com/akiselev/desktop-cli/discussions)

## License

desktop-cli is licensed under **GPL-3.0-only**. By contributing, you agree that your contributions will be licensed under the same terms.
