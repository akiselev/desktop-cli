---
name: desktop-cli
description: Semantic cross-platform desktop control CLI for agents
---

# desktop-cli agent guidance

Use the semantic interface by default. Packs project noisy native accessibility trees into app-specific interfaces; legacy commands remain compatibility/debugging tools.

## Core loop

```bash
desktop windows
desktop observe <window> [--pack altium]
desktop query <window> '@button[name="Save"]' [--pack altium]
desktop action <window> '$save' activate --pack altium
desktop observe <window> --pack altium
```

For multi-step work prefer `desktop serve <window> --pack altium`. It speaks one JSON object per line on stdin/stdout and keeps element references meaningful across observations.

## Observation rules

1. Use `observe` as the default state representation; use the raw tree only for pack/selector debugging.
2. If a pack is available, use it. Pack aliases such as `$projects`, `$properties`, or `$save` are the most stable interface.
3. Element refs (`e_...`) are session-scoped. If a ref is stale, observe/query again rather than guessing.
4. Modal dialogs, alerts, and focused elements are critical overlays and can survive normal pack pruning.
5. Bound large applications with `--max-nodes`, `--max-items`, and `--max-text`.

## Semantic selectors

Prefer semantics over native classes or coordinates:

```text
@button[name="Save"]
@input:focused
@tree[name*="Projects"] > @treeitem
#SaveButton
:supports(action)
@panel:has(@input[name="Width"])
@input:labelled-by(@text[name="Username"])
@button:below(@text[name="Password"])
$properties @input
```

Selectors support semantic roles, stable IDs, property operators (`=`, `*=`, `^=`, `$=`, `~=`), tri-state states, capabilities, child/descendant combinators, `:is`, `:not`, `:has`, semantic relations, spatial relations, and pack aliases.

## Actions

Use `desktop action` with the portable action vocabulary: `activate`, `focus`, `set-value`, `toggle`, `select`, `expand`, `collapse`, `increment`, `decrement`, `show-menu`, `scroll-into-view`, `confirm`, `cancel`, `raise-window`, `set-text`, and `replace-text`.

Accessibility is attempted first. Coordinate/input fallbacks are disabled unless explicitly requested or declared as a guarded pack fallback. Packs are observation policy, not authorization policy.

If the action string is not a built-in semantic action it is resolved as a pack-defined action.

## Pack development

```bash
desktop snapshot <window> --output tests/fixtures/my-app/state.json
desktop pack show ./packs/my-app
desktop pack validate <window> ./packs/my-app
desktop pack explain <window> ./packs/my-app '$properties'
```

A pack consists of `pack.toml` and `views.dcss`. Projection operations are `keep`, `flatten`, `prune`, and `collapse`. Prefer whitelist packs (`default-projection: prune`) for complex professional applications. Add native IDs or classes only from captured fixtures; never guess them.

## Compatibility escape hatch

Legacy commands remain for existing scripts and debugging: `summary`, `click`, `type`, `keys`, `scroll`, `dump-tree`, `find-element`, `invoke`, and `do`. They are not the preferred new agent protocol.
