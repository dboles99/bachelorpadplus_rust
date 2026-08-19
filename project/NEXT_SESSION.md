# Next session plan

Rewritten 2026-08-20, at the end of the fifth build session.

State: 24 crates, 1,633 tests, green on Windows and Linux. Three of those are
`#[ignore]`d on purpose, because each one names a real defect and wants a fix
or a decision rather than a passing assertion. There were four: the one that
destroyed documents is fixed, and its test runs now.

---

## The shape of the problem has changed

For three sessions the largest gap was that capabilities did not exist. It is
now the opposite. **Four crates are built, tested and unreachable** — the
large-file engine, notebooks, research and audit — and the queue's Tier 0 is
nothing but wiring. Building another library is the easy, comfortable move
and it is the wrong one.

Two things are worth more than any new crate:

### 1. Ten minutes with the application

Still first, still not done, and now with more riding on it. Security gained
four rows in the fourth session; the fifth rebuilt Verify Signature on
`bp-integrity`'s sidecar and added File ▸ Set as Default Editor, which writes
into the desktop database on Linux and saves a `.reg` on Windows. **None of it
has been clicked**, and the last of those is the one row here that changes
something outside this application. The `--editor-view` checklist below is
unchanged and still unanswered.

```powershell
cargo run --release
cargo run --release -- --editor-view
```

Click Security ▸ Scan for Secrets on a file with an AWS key in it. Click Hash
Document and check the digest against `sha256sum`. Note whether the menu, now
four rows longer, still reaches its own bottom row — `MenuPopup` caps its
height and scrolls, and a long menu putting its lowest rows out of reach has
happened here before and was found by a person, not a test.

Then File ▸ Set as Default Editor, which deserves reading rather than
clicking through: it reports what opens those types today *before* it offers
to change anything, and on Windows it writes nothing at all without a Save
dialog first. It is the only row in this product that touches state outside
it.

### 2. Fix the defects the cross-crate tests found

`project/WORK_QUEUE.md` Tier 0.5. **The one that destroyed documents is
fixed.** `bp_naming::sanitize` now takes the stem before the first dot, the
way Win32 does, so `con.txt`, `aux.log`, `NUL.dat` and `lpt1.bak` become
`con File.txt` and its friends rather than names that write to the device.
Its list gained `CONIN$` and `CONOUT$`, and a property test compares the two
crates' verdicts stem by stem, in both directions, so the two lists cannot
drift apart quietly.

Two sit behind it, pinned by `#[ignore]`d tests: `bp-notebook` running code
it could not name through the kernel's interpreter, and `bp-files` never
applying `\\?\`. The line-definition disagreement is D6 and wants an
answer, not a patch.

**Write more cross-crate tests as you go.** Eleven files of them have now
found seven defects that the unit tests had not. That ratio is still the most
useful thing this project has learned.

---

## The `--editor-view` checklist, unchanged and still unanswered

```powershell
cargo run --release -- --editor-view
```

This has been the first item for three sessions running and it has not been
done.

The custom editor view (ADR-0018) has 147 tests behind its *rules* — motion,
word boundaries, chords, tab stops, the involution property that caught two
real defects. Its *behaviour in a window* has been verified to exactly one
standard: it renders a frame with a real file open and exits without
panicking.

Everything below is unverified, and each is seconds:

| Check | What would be wrong |
| --- | --- |
| Type a character | Keys not reaching the `FocusScope` at all |
| Click mid-line | Caret lands left or right of the pointer — the measured advance is off |
| Click at the end of a long line | Advance error accumulating across columns |
| Arrow down through a short line and back up | The goal column not surviving |
| Drag to select | Selection rectangles misplaced, or drag not extending |
| Scroll the wheel | Wrong direction, or snapping back to the caret |
| **Ctrl+S** | The surface swallowing chords it should reject — this would stop saving |
| Ctrl+D, Alt+Up, Alt+Down | Duplicate and move line, shipped this session, never run |
| Watch the status bar | Ln/Col is the payoff; it should track the caret |
| Open a tab-indented file | Tab stops wrong, caret off by columns |
| Resize the window | Row count not following, blank strip at the bottom |

**The advance measurement and the wheel direction are the two most likely to
be subtly wrong.** Both are single-line fixes once someone has looked.

Write down what feels wrong, not only what breaks. The default is still
`TextInput`, so nothing is lost if the answer is bad — but five features
shipped in the last two sessions are enabled only here, and they all inherit
whatever that caret does.

---

## What is no longer on this list

Three items that headed previous versions of this plan are done, and are
recorded here so nobody re-derives them:

- **Wire the five built-but-unreachable things** — done, plus seven more.
  Every library half in the old table has its menu row.
- **Parity, or retire the flag** — settled, and not in the view's favour.
  Word wrap works; input-method composition cannot be done on Slint 1.17.1
  and the evidence is in `project/WORK_QUEUE.md`. `--editor-view` stays
  opt-in and the default stays `TextInput`.
- **Security profiles (phase 14)** — done. All three dependants read the
  policy, Privacy Mode clamps it for the session, and phase 15 shipping means
  the profiles wanting an encrypted journal are honoured rather than refused.

The replacement for all three is Tier 0 of `project/WORK_QUEUE.md`: four
crates with no way in.

## Decisions still waiting on a human

- **Where a signing key lives** (phase 16). This is the one blocking a row
  that is otherwise finished — Sign Document greys because there is no key
  store, no keyring integration and no decision on file permissions for a key
  on disk. ADR-0026 documents what `bp-integrity` can and cannot enforce per
  platform, which is most of the input needed.
- **One definition of a line.** `bp-search` counts `
`; `bp-buffer`'s rope
  uses ropey's `unicode_lines`. `bp-ui` uses both today, in different views.
- **What may leave the machine** (phases 8 and 10). Unchanged.
- **Whether `bp-storage` may record extracted titles** (phase 9). Narrower
  than it was — the security half is answered by phase 14 — but ADR-0019's
  product judgement still stands.

## Working with agents in this repository

Three sessions of evidence, worth reading before dispatching any:

- **`prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` earns its keep.** An
  agent that read it found and fixed an unbounded dispatch range that would
  have swallowed its own new ids.
- **Give one crate per agent.** Four ran concurrently without collision when
  each owned one crate. The moment two want `bp-ui` or `app.slint`, serialise.
- **Ask for a property, not more examples.** Every defect found by review this
  session was found by a property — "moving a line down and back returns the
  exact original bytes" caught two that example-based unit tests written
  alongside the feature missed entirely.
- **Tell them to report problems outside their scope rather than fix them.**
  Both times an agent fixed something adjacent, it was wrong to: a JSON
  reformatter that duplicated `bp-data` and disagreed with it on key order,
  and the removal of the launch stage's timeout inside a "pure refactor"
  commit.
- **`scripts/Invoke-LocalCI.ps1` changes are their own commit, never a
  passenger.** It is the thing that validates everything else.
- **Review the substance, not the report.** Agents self-reported all green in
  every case where a real defect was present.

---

## Housekeeping

- ~~**`ui/app.slint` wants splitting.**~~ **Done** — seven files. What is left
  in it, and why each thing stayed, is in `docs/architecture/ARCHITECTURE.md`.
  The popups were the one piece deliberately not moved: a `PopupWindow` is
  positioned relative to its parent element, so moving them would change the
  coordinate parent, and nothing in the gate can see a menu opening in the
  wrong place.
- **Native Linux timing.** WSLg's compositor makes startup numbers
  unrepresentative, so ADR-0015's evidence remains Windows-only.
- **Scroll smoothness under CPU rasterisation** is the standing revert
  condition on ADR-0017 and needs a capture rig. The custom view makes it more
  pressing: it redraws a set of `Text` elements per scroll.
- **Idle memory drifted from ~20 MB to ~32 MB** in one session after dialogs
  had been opened. Below the 50 MB target, observed twice, never investigated.
- **A clipboard menu row decodes its id against the history at click time**,
  not when the menu was built, and a 1.2 s poll can shift it in between.
  Pre-existing and applies to plain paste too; fixing it means freezing a
  snapshot while a menu is open.
- **`fuzz/corpus/yaml/` breaks editor tooling that scans the workspace.**
  Google's Cloud Code extension crash-loops its YAML language server on
  `billion-laughs.yaml` — a 570-byte file, 21 aborts in one window, and three
  notifications that never mention YAML. No checked-in setting fixes it —
  narrowing `cloudcode.yaml.yamlFileMatcher` was tried and does not, because
  the server dies during initialisation. Disable the extension for this
  workspace; nothing here is GCP. Any other tool that walks the tree for
  `*.yaml` is a candidate; the `.claude/worktrees/` copies multiply it.
  Reproduction and numbers in `fuzz/README.md`.

---

## How to work in this repository

- `./scripts/Invoke-LocalCI.ps1 -Linux` is the gate. There is no hosted CI
  (ADR-0016) and Actions is disabled at the repository level.
- `./scripts/Install-GitHooks.ps1` once per clone; the hooks run the gate.
- Every commit in this repository has passed the gate. Keep it that way.
- `docs/decisions/` is the register; add an ADR when a decision has
  consequences beyond one file.
- **Do not edit files while the gate is running.** The Linux leg re-runs
  `cargo fmt --check` from scratch, and a mid-run edit fails it for a reason
  that has nothing to do with the change.
