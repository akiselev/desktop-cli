# Documentation

Cross-platform desktop automation documentation, built with mdBook.

## mdBook Site

| What | When |
|------|------|
| `book.toml` | Configure mdBook build settings |
| `src/SUMMARY.md` | Edit navigation tree and page order |
| `src/introduction.md` | Edit project overview and features |
| `src/installation/` | Edit platform-specific install guides |
| `src/tutorials/` | Edit or add tutorials |
| `src/reference/` | Edit command, selector, and pattern reference |
| `src/contributing.md` | Edit contributing guide |

## Templates

| What | When |
|------|------|
| `templates/*.md.tmpl` | Edit auto-generated tutorial source templates |

Templates use `{{COMMAND:...}}` and `{{OUTPUT}}` markers. The generate script
(`scripts/generate-tutorials.sh`) executes commands in Docker and substitutes output.

## Other Docs

| What | When |
|------|------|
| `PLATFORMS.md` | Understand platform support status and requirements |
| `SETUP.md` | Set up platform-specific dependencies and permissions |
