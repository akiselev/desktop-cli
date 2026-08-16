# Altium Designer pack

This built-in pack is intentionally conservative. It projects Altium's large accessibility tree into a small semantic workspace and supplies mode-specific PCB, schematic, and modal views.

The pack does not guess version-specific native IDs. Capture raw semantic snapshots from the actual Altium releases under test, add stable selectors only after observing them, and retain semantic fallbacks.

Key aliases:

- `$projects` — Projects tree when exposed by accessibility
- `$properties` — Properties panel
- `$pcb` — active PCB document/canvas
- `$schematic` — active schematic document/canvas
- `$save` — resolved Save target

`save` is also exposed as a pack action with a guarded `Ctrl+S` fallback.
