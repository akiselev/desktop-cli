# Semantic architecture implementation audit

This document is a checklist for the semantic accessibility/application-pack refactor. It is intentionally implementation-facing: each item names behavior that must remain covered as the native providers replace the v1 compatibility paths.

## Core semantic model

- [x] Backend-neutral element graph with native identity preservation.
- [x] Stable developer IDs are distinct from runtime identities.
- [x] Typed property values include scalar, geometry, text-range, reference, collection, map, and opaque values.
- [x] Portable roles preserve native role/subrole metadata.
- [x] States are tri-state rather than lossy booleans.
- [x] Semantic relation kinds are represented separately from parent/child hierarchy.
- [x] Capabilities replace UIA-pattern strings as the portable interface.
- [x] Geometry records its coordinate space.
- [x] Public element refs are session/generation scoped and reject stale use.

## Provider boundaries

- [x] Accessibility, windows, input, and capture are independent traits.
- [x] Windows UIA provider owns COM objects on a dedicated actor thread.
- [x] Linux AT-SPI provider owns one Tokio runtime and accessibility-bus connection per session actor.
- [x] macOS AX provider owns retained AXUIElement references on a dedicated actor thread.
- [x] Existing v1 adapters remain a fallback for applications/providers that cannot initialize directly.
- [x] Provider query plans expose which selector predicates are eligible for native/early pushdown.
- [ ] Native event subscriptions are the preferred invalidation source; polling/reconciliation remains a recovery path.
- [ ] Provider-native collection/query APIs and virtualization realization must be used for very large/virtualized trees when available.

## Query language

- [x] Semantic role selectors.
- [x] Stable-ID and pack-alias selectors.
- [x] Exact/contains/prefix/suffix/wildcard property operators.
- [x] State and capability predicates.
- [x] Child and descendant combinators.
- [x] `:is`, `:not`, and `:has`.
- [x] Semantic relation predicates.
- [x] Spatial predicates.
- [x] Query-plan explanation.

## Observation protocol

- [x] Node, text, collection-item, depth, native-property-byte, and wall-clock budgets.
- [x] Collection collapse summaries.
- [x] Critical overlay for modal/focused/alert UI.
- [x] Sensitive/password value redaction.
- [x] Revisioned observations and observation diffs.
- [x] Operation telemetry counters.

## Application packs

- [x] `pack.toml` manifest and declarative `views.dcss` projection language.
- [x] Deterministic view activation, priority, and inheritance.
- [x] `keep`, `flatten`, `prune`, and `collapse` projection operations.
- [x] Aliases, targets, fallback selectors, semantic actions, and guarded key fallback.
- [x] Pack projection is independent from ActionPolicy authorization.
- [x] Built-in conservative Altium pack with workspace/modal/PCB/schematic views.
- [x] Explicit/project/user/built-in pack registry precedence.
- [x] Snapshot-backed pack validation entry point.
- [ ] Snapshot expectation files should assert aliases, view, ambiguity, compression, and token/node budgets as regression tests.

## Session and agent protocol

- [x] Long-lived `DesktopSession` owns graph/provider/pack/policy state.
- [x] Model-independent semantic agent command schema.
- [x] JSON-lines persistent session protocol over stdio.
- [x] Semantic actions are accessibility-first; keyboard/pointer fallbacks are separately authorized.
- [x] Action results carry before/after revisions and observation deltas.
- [ ] Model-specific legacy planner must consume semantic refs/aliases rather than exact visible names.
- [ ] Optional local socket/named-pipe transport should wrap the same versioned request/response protocol.

## Compatibility and validation

- [x] Existing summary/click/type/keys/scroll/dump/find/invoke/do commands remain available for a compatibility period.
- [x] Cross-platform semantic graph/query/pack tests.
- [x] Deterministic mock provider including semantic actions and events.
- [x] Session conformance tests.
- [ ] Native UIA/AT-SPI/AX provider compilation and platform smoke tests must be green before merge.
- [ ] Documentation and agent instructions must present semantic observe/query/action as canonical and legacy commands as compatibility paths.

The unchecked items are integration gates, not alternate designs. They should be resolved in this pull request before it is marked ready for review.
