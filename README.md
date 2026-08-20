# Desktop CLI

[![CI](https://github.com/akiselev/desktop-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/akiselev/desktop-cli/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://akiselev.github.io/desktop-cli/)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-green)](LICENSE)

A cross-platform desktop control CLI designed for software agents. `desktop-cli` normalizes Windows UI Automation, Linux AT-SPI2, and macOS Accessibility into one semantic graph, then projects large application interfaces through declarative **application packs** so an agent sees the controls and state that matter instead of an unbounded native accessibility dump.

## Architecture

```text
Windows UIA        Linux AT-SPI2        macOS AX
     │                   │                  │
     └──────── native AccessibilityBackend ┘
                         │
                         ▼
                 AccessibilityGraph
            roles / states / capabilities
            relations / typed properties
                  stable session refs
                         │
                semantic selectors
                         │
                         ▼
                  Application Pack
          keep / flatten / prune / collapse
          aliases / views / targets / actions
                         │
                         ▼
                 bounded Observation
                         │
                    agent / CLI
```

Window discovery, accessibility, input injection, and screen capture are separate provider contracts. Native accessibility actions are preferred; keyboard or pointer fallback must be explicitly authorized.

## Platform support

| Platform | Accessibility provider | Input | Semantic provider |
|---|---|---|---|
| Windows | UI Automation | SendInput | direct UIA actor with cached properties/patterns |
| Linux | AT-SPI2 | enigo/X11 compatibility path | direct persistent AT-SPI actor with fallback |
| macOS | AXUIElement | enigo/CGEvent | direct retained-element AX actor with fallback |

The compatibility providers remain available while native providers are hardened against applications with incomplete or broken accessibility implementations.

## Quick start

```bash
cargo install desktop-cli

desktop windows
desktop observe notepad
desktop query notepad '@button[name="Save"]'
desktop action notepad '@button[name="Save"]' activate
```

For a complex application, use an application pack:

```bash
desktop observe altium --pack altium
desktop query altium '$properties @input' --pack altium --all
desktop action altium '$save' activate --pack altium
```

Altium windows are also eligible for automatic built-in pack detection.

## Semantic selectors

Selectors describe user-facing semantics rather than one platform's native object model:

```text
@button[name="Save"]
@input:focused
@tree[name*="Projects"] > @treeitem
#stableDeveloperId
:supports(action)
@panel:has(@input[name="Width"])
@input:labelled-by(@text[name="Username"])
@button:below(@text[name="Password"])
$properties @input
```

The grammar supports roles, stable IDs, pack aliases, property operators (`=`, `*=`, `^=`, `$=`, `~=`), tri-state states, capabilities, child/descendant combinators, `:is`, `:not`, `:has`, semantic relations, and spatial predicates. Native properties remain available as an escape hatch without becoming the portable API.

## Bounded observations

`desktop observe` is the canonical state representation for agents. It enforces explicit structural and output budgets:

```bash
desktop observe altium --pack altium \
  --max-nodes 300 \
  --max-items 40 \
  --max-text 24000 \
  --max-depth 20 \
  --max-native-bytes 16384 \
  --max-millis 2000
```

Large repeated collections can be collapsed to count/selection/focus/sample summaries. Modal dialogs, alerts, and focused elements form a critical overlay so pack pruning cannot hide immediate interaction state. Protected/password values are redacted before an observation is emitted.

## Stable references and sessions

One-shot commands construct a `DesktopSession` internally. Multi-step agents should use a persistent session:

```bash
desktop serve altium --pack altium
```

`serve` accepts one JSON request per line on stdin and writes one JSON response per line. Session-scoped element refs such as `e_...` remain meaningful while the native element lives; stale refs are rejected rather than silently resolving to another control.

Actions return before/after revisions and an observation delta so agent loops do not need to rediscover the entire UI after every operation.

## Semantic actions

Portable actions include:

```text
activate
focus
set-value
set-text
replace-text
toggle
select
expand
collapse
increment
decrement
show-menu
scroll-into-view
confirm
cancel
raise-window
```

Example:

```bash
desktop action app '$save' activate --pack my-pack
```

Accessibility is attempted first. `--input-fallback` and `--keyboard-fallback` are separate policy decisions; a pack's projection rules do not grant permission to perform an action.

## Application packs

A pack is declarative data:

```text
packs/my-app/
  pack.toml
  views.dcss
```

`pack.toml` contains metadata, detection rules, and the default view. `views.dcss` defines semantic projections:

```css
@view workspace {
    default-projection: prune;

    @dialog:modal {
        projection: keep;
        importance: critical;
    }

    @tree[name="Projects"] {
        projection: keep;
        alias: projects;
        max-items: 50;
    }

    @toolbar {
        projection: collapse;
        expose: name actions count;
    }
}
```

Projection operations are:

- `keep` — emit the node and recursively project children.
- `flatten` — remove the structural wrapper but continue through its children.
- `prune` — remove the node and subtree, except critical overlays.
- `collapse` — emit one semantic node plus a collection summary instead of all descendants.

Pack tooling:

```bash
desktop pack list
desktop pack detect altium
desktop pack show altium
desktop pack validate altium altium
desktop snapshot altium --output altium-pcb.json
desktop pack test altium altium-pcb.json
desktop pack explain altium '$properties' --window altium
```

Pack lookup supports explicit paths plus project, user, and built-in locations with deterministic precedence.

## Built-in Altium pack

The built-in `altium` pack is intentionally conservative. It exposes stable semantic regions such as Projects and Properties, recognizes PCB/schematic/modal modes, collapses ribbon/menu noise, and defines a Save target/action with a separately authorized `Ctrl+S` fallback. Version-specific native IDs are not guessed; they should be added only from captured fixtures.

## Snapshots and pack development

Normalized semantic snapshots make pack work reproducible and cross-platform:

```bash
desktop snapshot app --output state.json
desktop pack test ./packs/app state.json
desktop pack explain ./packs/app '@toolbar' --snapshot state.json
```

The test suite contains deterministic semantic, session, protocol, and Altium projection regression coverage. Native platform CI additionally checks Windows, Linux, and macOS implementations.

## Compatibility commands

The original v1 commands remain available for existing scripts and low-level debugging:

```text
summary
click
type
keys
scroll
dump-tree
find-element
invoke
do
```

New integrations should use `observe`, semantic `query`, `action`, `snapshot`, `pack`, and `serve`.

## Development

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Platform E2E tests require their native desktop/accessibility environment. Windows E2E tests must run serially; Linux E2E runs in the repository's Xvfb/D-Bus/AT-SPI Docker environment; macOS headless CI compiles/tests the provider but cannot grant TCC Accessibility permission automatically.

See [`docs/semantic-architecture.md`](docs/semantic-architecture.md), [`docs/implementation-audit.md`](docs/implementation-audit.md), and [`SKILL.md`](SKILL.md) for implementation and agent guidance.

## License

GPL-3.0-only
