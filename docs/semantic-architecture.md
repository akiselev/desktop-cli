# Semantic accessibility and application-pack architecture

`desktop-cli` treats platform accessibility APIs as backends for a shared semantic graph. Windows UI Automation, Linux AT-SPI2, and macOS AX keep their native identity and metadata while exposing portable roles, tri-state states, semantic relations, typed properties, capabilities, actions, and geometry.

## Layers

```text
UIA / AT-SPI2 / AX
        |
        v
AccessibilityBackend        WindowBackend / InputBackend / CaptureBackend
        |
        v
DesktopSession -> AccessibilityGraph -> semantic selectors
        |                                  |
        +--------------------+-------------+
                             v
                      application pack
                             |
                   projected Observation
                             |
                     agent / JSONL client
```

Accessibility discovery is intentionally independent of X11/window enumeration, input injection does not define accessibility semantics, and capture/vision can evolve independently.

## Semantic model

The graph preserves both normalized and native information: `ElementIdentity`, portable `Role` plus `NativeRole`, tri-state `StateSet`, typed `PropertyValue`, `CapabilityKind`, semantic `Relation` edges, `Geometry`, actions, and raw native metadata. Public `e_...` refs are scoped to a `DesktopSession` and graph generation and are rejected when stale.

## Selectors and query planning

The semantic selector language is shared by queries and packs. It supports roles, stable IDs, typed properties, states, capabilities, structural combinators, `:is`, `:not`, `:has`, relation predicates, spatial predicates, and `$pack` aliases. Backends expose a `NativeQueryPlan` so supported predicates can be pushed into UIA conditions, AT-SPI Collection rules, or AX early-prune traversal while residual predicates stay in the graph engine.

## Packs

A pack is declarative data: `pack.toml` plus `views.dcss`. Views activate from semantic UI state and can inherit another view. Rules are deterministic and ordered. The projection operations are `keep`, `flatten`, `prune`, and `collapse`; aliases create the stable vocabulary agents should use; targets provide selector fallbacks; actions bind targets to portable actions plus optional guarded key fallbacks.

Pack projection is observation policy, not authorization. Modal dialogs, alerts, and focused elements form a critical overlay so normal pruning cannot hide immediate interaction state.

## Sessions

A `DesktopSession` owns graph state, backend handles, the active pack, and event receiver. `desktop serve` exposes the session as JSON-lines. This is the basis for long-lived UIA/AT-SPI/AX event subscriptions and incremental observations; one-shot CLI commands construct the same session in-process.

## Migration

The semantic/provider/session surface is canonical. Existing platform implementations are bridged through `LegacyAccessibilityBackend`, allowing all three platforms to use the graph/pack/agent interfaces during migration. Platform mapping modules preserve richer UIA, AT-SPI, and AX semantics so direct v2 providers can replace the compatibility bridge without changing packs or agent-facing APIs.
