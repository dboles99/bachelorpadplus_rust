# Next session plan

Written 2026-08-17, at the end of the second build session.

State: 15 crates, 393 tests, green on Windows and Linux, 16 commits all
through the full local gate. `ROADMAP.md` has per-phase status.

---

## Start here: type into `--editor-view`

**Before writing any new code**, and this time it is specific rather than
general good practice.

```powershell
cargo run --release -- --editor-view
```

The custom editor view (ADR-0018) exists. Its *rules* are covered by 91 tests
in `bp-editor` — motion, word boundaries, chords, the visual/character column
distinction that tabs make load-bearing. Its *behaviour in a window* has been
verified to exactly one standard: it renders a frame with a real file open and
exits without panicking.

Everything below is unverified and each is a five-second check:

| Check | What would be wrong |
| --- | --- |
| Type a character | Keys not arriving at the `FocusScope` at all |
| Click somewhere mid-line | Caret lands left or right of the pointer — the measured advance is off |
| Click at the end of a long line | Advance error accumulates across columns |
| Arrow up and down through a short line | The goal column not surviving |
| Drag to select | Selection rectangles misplaced, or drag not extending |
| Scroll the wheel | Wrong direction, or snapping back to the caret |
| Ctrl+S | The surface swallowing chords it should reject — this would stop saving |
| Watch the status bar | Ln/Col is the payoff; it should track the caret |
| Tab-indented file | Tab stops in the wrong place, caret off by columns |
| Resize the window | Row count not following, blank strip at the bottom |

Write down what feels wrong, not only what breaks. The `advance` measurement
and the wheel direction are the two most likely to be subtly wrong.

The default is still `TextInput`, so nothing is lost if it is bad.

---

## Then: parity, or back it out

`--editor-view` is a temporary flag and should not become permanent furniture.
Two things stand between it and being the default:

1. **Word wrap.** The deep one. It makes a visual line differ from a document
   line, which every function in `bp_editor::view` currently assumes away, and
   it changes what Up and Down mean.
2. **Input-method composition.** Slint delivers pre-edit text through
   `KeyEventType::UpdateComposition`, which a bare `FocusScope` does not see.
   Without it, CJK entry does not work at all.

Double- and triple-click selection is a third, much smaller gap.

If those look worse than they are worth, the honest outcome is to delete the
surface and keep the rope — the storage change is the valuable half and it is
already unconditional.

---

## Also available

| Work | Phase | Notes |
| --- | --- | --- |
| Wire `bp-storage` in | 9 | The crate exists and is tested. **Read ADR-0019 first**: recording extracted titles puts a plaintext index of what the user writes about on disk, which wants phase 14, not a quiet default. |
| `bp-organize` on top of it | 9 | Related notes, duplicate detection. The store's schema was built for this. |
| Markdown preview | 5 | Needs a rendering decision first — Slint has no HTML view. |
| Paste transformations | 11 | Kind detection already identifies entries; nothing acts on it. |
| Duplicate Line, Move Line Up/Down | 2 | Now possible — the caret exists. Greyed in the Edit menu, waiting to be wired. |
| Go-to-line, zoom | 2 | Same: unblocked by the caret. |

---

## Decisions still waiting on a human

- **`.bpadx` envelope format** (phase 15). ADR-0011 forbids custom
  cryptography, which constrains this more than it looks.
- **What may leave the machine** (phases 8 and 10). Nothing says which
  providers are acceptable.
- **YAML parser** (phase 6). `serde_yaml` is deprecated.
- **Security profiles** (phase 14) are now *more* urgent, not less. Three
  things want them: the recovery journal writing unsaved text to disk in
  plaintext, clipboard history holding copied secrets in memory, and
  `bp-storage`, which is built and deliberately unwired for exactly this
  reason.

---

## Housekeeping worth doing at some point

- **Native Linux timing.** WSLg's compositor makes startup numbers
  unrepresentative, so ADR-0015's evidence remains Windows-only.
- **Scroll smoothness under CPU rasterisation** is the standing revert
  condition on ADR-0017 and needs a capture rig. The custom view makes this
  more pressing: it now redraws a set of `Text` elements per scroll.
- **`bp-ui` is past 2,200 lines.** Menus are already out; the editor-surface
  handlers are the obvious next module.
- **Idle memory drifted from ~20 MB to ~32 MB** in one session after dialogs
  had been opened. Below the 50 MB target, observed twice, never investigated.

---

## How to work in this repository

- `./scripts/Invoke-LocalCI.ps1 -Linux` is the gate. There is no hosted CI
  (ADR-0016) and Actions is disabled at the repository level.
- `./scripts/Install-GitHooks.ps1` once per clone; the hooks run the gate.
- Every commit in this repository has passed the gate. Keep it that way.
- `docs/decisions/` is the register; add an ADR when a decision has
  consequences beyond one file.
- **Do not edit files while the gate is running.** The Linux leg re-runs
  `cargo fmt --check` from scratch, and a mid-run edit fails it confusingly.
