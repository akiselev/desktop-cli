# Windows Installation

## Prerequisites

### Rust Toolchain

Install Rust via [rustup](https://rustup.rs/):

1. Download and run [rustup-init.exe](https://win.rustup.rs/)
2. Follow the installer prompts (default options work fine)
3. Restart your terminal to update PATH

Verify installation:

```powershell
rustc --version
cargo --version
```

### No Additional Dependencies

Unlike Linux, Windows requires **no additional system dependencies**. UI Automation (UIA) is built into Windows and desktop-cli uses it directly.

## Installation

### Option 1: Install from Crates.io

```powershell
cargo install desktop-cli
```

### Option 2: Build from Source

```powershell
git clone https://github.com/akiselev/desktop-cli.git
cd desktop-cli
cargo build --release

# Binary will be at target\release\desktop.exe
# Add target\release to your PATH or copy the binary
```

## Verify Installation

```powershell
desktop --version
```

## UI Automation

desktop-cli uses **UI Automation (UIA)**, the native Windows accessibility API. UIA is:

- **Built into Windows**: No installation or configuration needed
- **Always Available**: Works on Windows 7 and later
- **Framework Agnostic**: Works with Win32, WPF, UWP, WinForms, Electron, and more

## Verify It Works

```powershell
# List all windows
desktop windows

# Get summary of Notepad
desktop summary notepad

# Click a button
desktop click notepad "@button 'Save'"
```

## Common Issues

### "Access Denied" or "Operation Failed"

Some Windows system applications (like Task Manager) run with elevated privileges. To automate them:

1. Run your terminal **as Administrator**
2. Then run desktop-cli commands

### "Cannot find window"

Make sure:
1. The application is actually open and visible
2. The window is not minimized
3. You're using the correct window query (try `desktop windows` to list all windows)

### "Element not found"

If `desktop summary <window>` shows elements but you can't interact with them:

1. Some applications don't expose full accessibility information
2. Try using coordinate-based actions: `desktop click notepad --coords 100,200`
3. Check if the application is built with an accessibility-aware framework

## Supported Windows Versions

- ✅ Windows 11 (all editions)
- ✅ Windows 10 (all editions)
- ✅ Windows 8/8.1
- ✅ Windows 7 (with Platform Update)

## Next Steps

- [Tutorials](../tutorials/basics.md): Learn how to use desktop-cli
- [Commands Reference](../reference/commands.md): Explore all available commands
