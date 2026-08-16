# AGENTS.md

## Architecture invariants

New code must target the semantic provider/session architecture, not add more platform-specific behavior to `UiaElement` or `DesktopPlatform`.

1. **Normalize semantics; preserve mechanics.** Convert native roles/states/capabilities/relations/actions into the portable model while retaining native role/subrole, properties, interfaces, action names, and runtime identity.
2. **Do not make UIA the universal vocabulary.** UIA patterns, AT-SPI interfaces, and AX attributes/actions are backend mechanics. Portable consumers use `CapabilityKind`, `SemanticAction`, `Role`, `State`, `RelationKind`, and typed `PropertyValue`.
3. **Keep providers separate.** Accessibility, windows, input, and capture are independent contracts. Do not make accessibility depend on X11/CGWindow/HWND discovery once a native target is resolved.
4. **Accessibility first.** Use native accessibility actions before keyboard/pointer fallback. Fallback requires `ActionPolicy`; pack projection is never authorization.
5. **Bound all observation work.** Large trees and text must obey node/depth/item/text/native-byte/deadline budgets. Prefer native cache/query/virtualization facilities to unbounded recursion.
6. **Keep references honest.** Public `ElementRef`s are session/generation scoped. Reject stale references rather than retargeting them heuristically.
7. **Packs are declarative.** Pack v1 contains selectors, views, projection rules, aliases, targets, and actions. Do not add embedded scripting to solve application quirks.
8. **Critical interaction state wins.** Modal dialogs, alerts, focused elements, and equivalent urgent states must remain observable even when an app pack normally prunes that subtree.
9. **Never guess native selectors.** Add automation IDs/classes/AX identifiers/AT-SPI attributes to application packs only after capturing them from a real fixture.
10. **Sensitive values are redacted at the observation boundary.** Logging/telemetry must not reintroduce them.

## Native provider rules

### Windows UIA

- Keep COM/UIAutomation objects on the dedicated UIA actor thread.
- Use cache requests and BuildCache for properties/patterns used by normal traversal.
- Preserve RuntimeId plus process identity.
- Prefer UIA conditions/native tree views for pushdown where the selector can be represented exactly.
- Realize virtualized items only on explicit demand; do not enumerate enormous virtualized collections by default.

### Linux AT-SPI2

- Keep one accessibility-bus connection/runtime per session actor rather than creating a Tokio runtime per operation.
- Native identity is bus name + object path; preserve accessible IDs when supplied by the application.
- Preserve AT-SPI states, interfaces, attributes, named actions/keybindings, and relation kinds.
- Prefer Collection/Cache/event facilities for large trees instead of one D-Bus round trip per field.
- Accessibility discovery must not require X11; window/input/capture providers may have separate X11/Wayland implementations.

### macOS AX

- Retain `AXUIElementRef`s in the AX actor registry and release them on teardown.
- Decode `CFTypeRef`/`AXValue` by runtime type; never assume `AXValue` is a string or geometry is a dictionary.
- Discover supported attributes/actions from the element rather than inferring actions only from role.
- Use `AXIdentifier` when available as a developer/stable ID, but do not confuse it with runtime identity.
- Prefer batched/paged AX attribute reads and AXObserver notifications as the provider is optimized.

## Packs

A pack consists of `pack.toml` and `views.dcss`.

- Complex applications should normally use `default-projection: prune`.
- Projection semantics: `keep`, `flatten`, `prune`, `collapse`.
- Use semantic aliases for stable agent vocabulary (`$projects`, `$properties`, `$save`).
- A named target may contain fallback selectors; ambiguity is an error/diagnostic, not an invitation to pick the first control.
- Keep mode selection declarative through `@view ... when <selector>`.
- Every significant pack change needs a normalized snapshot or deterministic fixture regression test.

## Testing

Before considering a refactor step complete:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Also run the applicable native platform E2E path. Windows E2E is serial. Linux uses the Xvfb/D-Bus/AT-SPI Docker fixture. macOS CI can compile/test semantic/provider code but interactive AX behavior requires a machine with Accessibility permission.

Tests should cover observable semantics, not merely that a function returned `Ok`. For pack work, assert active view, aliases, ambiguity diagnostics, compression/budgets, and critical overlays. For action work, assert route, before/after revisions, observation diff, and policy behavior.

## Compatibility

The v1 commands and `UiaElement`/`DesktopPlatform` path are compatibility surfaces. Avoid growing them. New features belong in semantic/provider/session code and should be adapted outward only when needed for existing scripts.

## Documentation

When public behavior changes, update `README.md`, `SKILL.md`, and the relevant semantic/pack reference. Keep `docs/implementation-audit.md` synchronized with any remaining integration gate; do not mark a native feature complete solely because the type exists.
