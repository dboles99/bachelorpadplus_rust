# Inventory Pass D: Inert Policy Axes

> **Part of the v1.1.1 inventory** — V1_1_1_PLAN.md §2.
> **Method:** For each policy axis, check whether anything consults it to *change
> behaviour*, not merely to display it. An axis that is only read by inspectors
> or tests is inert: it governs nothing.

## Date

28 October 2026, started ~23:36 UTC

## Finding

**Four of the seven `Policy` axes in `bp_security::Policy` are inert:** they are
read only for display purposes (by `bp_ui::state::inspectors` or
`bp_ui::menus`) or in tests, and **never consulted to decide what the product
actually does**.

| Axis | Type | Read by | Computes behaviour? | Disposition |
| --- | --- | --- | --- | --- |
| `recovery` | [`Recovery`](bp_security::Recovery) | `bp_ui::state::security.rs`, `bp_history::Journal`, `bp_audit::AuditLog::for_policy` | ✅ Yes | Active — governs journal writing and audit destination |
| `clipboard` | [`Clipboard`](bp_security::Clipboard) | `bp_ui::state::security.rs` (`self.clips.enforce(policy.clipboard)`), `bp_ui::menus`, `bp_ui::state::inspectors` | ✅ Yes | Active — enforces clipboard history clearing |
| `metadata` | [`Metadata`](bp_security::Metadata) | `bp_audit::AuditLog::for_policy`, `bp_ui::state::inspectors` | ✅ Yes | Active — selects audit destination |
| **`embeddings`** | [`Embeddings`](bp_security::Embeddings) | `bp_ui::state::inspectors` (2 calls), `bp_audit/tests` | ❌ No | **Inert** — known case, see V1_1_1_PLAN.md §2 |
| **`network`** | [`Network`](bp_security::Network) | `bp_ui::menus` (menu enablement) | ❌ No | **Inert** — menu row greyed by value, no runtime effect |
| **`temporary_files`** | [`TemporaryFiles`](bp_security::TemporaryFiles) | `bp_security/tests` only | ❌ No | **Inert** — tested but never consulted |
| **`zeroise`** | [`Zeroise`](bp_security::Zeroise) | `bp_security/tests` only | ❌ No | **Inert** — tested but never consulted |

## Verification commands

```bash
# Check all non-test reads of policy axes
git grep -n "policy\." -- "*.rs" \
  | grep -v "test" \
  | grep -v "inspectors" \
  | grep -v "menus"

# Check specific axis reads
 git grep -n "policy\.embeddings" -- "*.rs" | grep -v test | grep -v inspectors
 git grep -n "policy\.network" -- "*.rs" | grep -v test | grep -v menus
 git grep -n "policy\.temporary_files" -- "*.rs" | grep -v test
 git grep -n "policy\.zeroise" -- "*.rs" | grep -v test
```

## Recommended action

Per V1_1_1_PLAN.md §2, table entry for `Policy::embeddings`:

> **Experimental**, behind E1 and a provider — or the readout says it governs nothing yet

The same recommendation applies to `network`, `temporary_files` and `zeroise`:
add to the Experimental menu's readout that these axes currently govern
nothing and will compute when a provider ships, or make the Security Inspector
readout explicit that they are not yet wired.

Alternatively, wire them to actual behaviour (E1–E3 for embeddings; a network
blocking mechanism for `network`; actual file lifecycle checks for
`temporary_files`; and memory-zeroing on close for `zeroise`).

## Related

- [V1_1_1_PLAN.md §2](V1_1_1_PLAN.md#2-the-inventory-and-why-it-comes-first) — the inventory method
- [ADR-0056](../docs/decisions/ADR-0056.md) — all three embedding sources as choices
- `bp_security::Policy` — the seven-axis policy type
- `bp_ui::state::inspectors::security_inspector_report` — the Security Inspector readout
