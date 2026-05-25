# Scripts

Build and generation scripts for desktop-cli.

| What | When |
|------|------|
| `generate-tutorials.sh` | Generate tutorial markdown from .tmpl templates |

## Security Note

`generate-tutorials.sh` executes `{{COMMAND:...}}` markers from template files.
Only run with trusted templates in a sandboxed environment (Docker).
