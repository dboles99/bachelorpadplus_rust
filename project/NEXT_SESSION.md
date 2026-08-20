# Next session plan

Rewritten 2026-08-20, at the end of the sixth build session.

State: 24 crates, 1,693 tests, **0 ignored**, green on Windows and Linux
across all ten gate stages. Every defect the eleven files of cross-crate tests
found is fixed, and the last decision behind one of them — D6, what a line is
— is ADR-0029.

---

## The shape of the problem has changed again

Three sessions ago the gap was that capabilities did not exist. Two sessions
ago it was that they existed and were unreachable. **Neither is the gap now.**

Twenty of the twenty-three library crates are reachable from the shell. Every
defect the cross-crate tests found is fixed. There is no `#[ignore]`d test
anywhere in the tree. The record has been reconciled with the product and the
gate has grown two stages that were missing. What is left is smaller, and
sharper, than any of that.

**Three things are worth more than anything else in the queue.**

### 1. The viewer for a document the rope does not hold

The largest single thing between this repository and a product, and it is now
the *only* part of the large-file story still missing.

The classification half shipped: the open path reads metadata before it reads
a byte, an ordinary document opens as it always did, a large one opens and
edits with its size in the status bar, and a huge one is refused in
`Access::ReadOnlyBySize`'s own words. "Opening a 2 GB file loads 2 GB" is no
longer true.

What is missing is showing one. `bp_buffer::LargeFile` has `display_lines`,
`index_through_line`, `line_start` and `WindowReader`, all tested and
benchmarked (ADR-0027 carries the numbers). Connecting them means
`display_lines` driving the editor surface's scroll anchor — which is
**`--editor-view` only**, because `TextInput` owns its own text and there is
no way to hand it a window of a file it does not have.

That is the shape of the work, and it is worth knowing before starting: this
is the first feature that is *only* available under the opt-in view. Decide
whether that is acceptable before writing code, because the alternative — a
huge file that opens under one flag and is refused under the other — is a
product answer, not an implementation detail.

### 2. Finish the manual pass

Four checks remain, and the last pass is the argument for doing them: it found
a defect 177 tests could not, in the one seam a test in this repository cannot
see — what a toolkit does with the string it is handed.

| Still unchecked | Why it matters |
| --- | --- |
| Scroll the wheel | Direction, and whether it snaps back to the caret |
| Drag to select | Whether the selection rectangles land where the drag went |
| Resize the window | Whether the row count follows, or a blank strip appears |
| Arrow down through a short line and back up | Whether the goal column survives |
| **File ▸ Set as Default Editor** | **The only row that changes state outside this application.** It reports before it offers and writes nothing on Windows without a Save dialog first — so it wants reading rather than clicking through |

### 3. ~~The two latent defects~~ — done

Both are fixed. Neither was wrong on the day it was written down, which is the
argument for the tier: the row that would have gone wrong had not been written
yet.

- **`recent.toml`, the recovery journal and the security history all roamed
  on Windows.** All three hung off the config directory, which is `%APPDATA%`;
  all three hold machine-specific absolute paths, which is exactly what
  `bp_platform::DirKind::roams` warns against. All three resolve through
  `DirKind::State` now, in one change owning `bp-config` **and** `bp-ui`,
  because splitting it would have scattered the product's files. The
  housekeeping item came with it: `cargo test` no longer creates a recovery
  directory in the developer's own profile.
- **`Profile::is_data` was true for `Ini` and `Xml`.** The class question is
  `Profile::is_data_class` and says outright that it is not the menu
  question; the menu question is `Format::has_data_operations`, asked of the
  format. A test walks `Format::ALL` against `menus::data` so the two are one
  truth.

What generalises, in one line each: **a path derived by subtracting another
path's last component moves when that one does**, **separate the rule from
the edge that reads the environment**, and **a predicate on the wrong type is
a trap with no name.**

## The `--editor-view` checklist — answered, and it found one defect

```powershell
cargo run --release -- --editor-view
```

**Done on 2026-08-20**, after heading this file for four sessions. A person
drove both views. What it found is worth more than the four sessions of
waiting suggested, and less than feared.

| Check | Answer |
| --- | --- |
| Type a character | Works, both views |
| Click mid-line | **Caret tracks the pointer.** A click on a line reported that line; a click at the end of `line 30` reported `Ln 30, Col 8`, exactly its length plus one |
| **Ctrl+S** | **Works** — verified by reading the file back off the disk, not by the status bar |
| Watch the status bar | `Ln 18, Col 66` and `Ln 30, Col 8`. The payoff is real |
| Open a tab-indented file | **Broken, and now fixed** — see below |
| The Security menu reaching its bottom row | Reaches it. `MenuPopup` is not clipping the four rows added since |
| Recovery journal | Fires unprompted; the status bar showed `Recovery 10:00 AM` |
| Scroll the wheel | **Still unchecked** |
| Drag to select | **Still unchecked** |
| Resize the window | **Still unchecked** |
| Arrow down through a short line and back up | **Still unchecked** |

### What it found: a tab was drawn as one glyph

`bp_editor::view` computed columns as though tabs reached the next stop --
`visual_column`, `char_column` and `wrap` all did -- while `VisualRow::text`
handed the raw `	` to Slint, which draws it as a single glyph. The surface
places the caret and the selection at `visual_column x advance`, so **on any
line containing a tab the caret sat somewhere the character was not, and the
error grew with every tab further along the line.**

Fixed by `bp_editor::view::expand_tabs` and `VisualRow::display_text`.
`text` stays raw, because the column arithmetic in that module indexes it by
character; `display_text` is what the toolkit gets. The property that pins
it: for every position in a line, the number of characters drawn before it
equals the visual column computed for it.

**Nothing in 177 tests caught this, and nothing was going to.** Every one of
them was about the model, and the model was right. The defect lived in the
one seam a test cannot see -- what a toolkit does with the string it is
handed -- which is the exact argument this file has been making for four
sessions about why somebody has to look.

### Two things about doing this again

- **Do not drive the window while somebody else is.** Two of the three
  findings from this pass evaporated on inspection: a fixture whose line
  labels were hand-numbered and drifted from the gutter by one (the gutter
  was right), and an apparent stray deletion that was the other person
  typing. Generate a fixture rather than numbering it by hand -- the one in
  the scratchpad asserts every label against its own position.
- **`AKIAIOSFODNN7EXAMPLE` is not a finding, and a scan that ignores it is
  working.** `bp-secrets` treats anything containing `example` as a
  placeholder, deliberately, with a test pinning that exact string. A fixture
  meant to exercise the scan needs a key that looks random --
  `AKIA5MZ7QK2XPLRW4VTD` does.

## What is no longer on this list

Recorded so nobody re-derives them.

- **Ten minutes with the application** — done, and it found the tab defect.
- **Wire the built-but-unreachable things** — all but three crates are
  reachable, and the three that are not are named above.
- **Parity, or retire the flag** — settled and not in the view's favour.
  Input-method composition cannot work on Slint 1.17.1; `--editor-view` stays
  opt-in.
- **Security profiles (phase 14)** — done; all three dependants read the
  policy.
- **Make the record match the product** — done, and the lesson kept: every
  stale claim went stale the same way, a fix landing without the record
  moving.
- **Put `fuzz/` inside the gate** — done, on both legs, and verified to fail
  before it was trusted.
- **The seven cross-crate defects** — all fixed. The device name that
  destroyed documents, the typed name that did the same, the tab drawn as one
  glyph, the extended-length prefix, the notebook language, the JSONL sniff,
  the encoder's location.
- **D6, what a line is** — ADR-0029. A break is `
` or `
`; the rope
  dropped `unicode_lines`, a default nobody chose.

## Decisions still waiting on a human

`project/DECISIONS_NEEDED.md` is the register; these are the ones that block
work rather than merely await it.

- **Where a signing key lives (D5).** The one blocking a row that is otherwise
  finished — Security ▸ Sign Document greys because there is no key store, no
  keyring integration and no decision on file permissions for a key on disk.
  ADR-0026 documents what `bp-integrity` can and cannot enforce per platform,
  which is most of the input needed.
- **Whether a huge document may be viewable only under `--editor-view`.** New,
  and it gates item 1 above. `TextInput` owns its own text, so a document
  served from disk in chunks cannot be shown in it. Either the feature is
  opt-in-only, or the default view has to change, or a huge file opens under
  one flag and is refused under the other — which is the answer to avoid, and
  the reason to decide before writing code.
- **What may leave the machine (D1).** Unchanged. Phases 8 and 10.
- **Whether `bp-storage` may record extracted titles (D3).** Narrower than it
  was — phase 14 answered the security half — but ADR-0019's product
  judgement still stands.
- **`bp-crypto`'s derive-once API (D7).** Reading a sealed audit history costs
  a full Argon2id derivation per line, so a few hundred lines is several
  seconds. It has not bitten yet because no history is that long.

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
- **Code taking a `Platform` parameter must not let `std::path` answer for
  it.** This has now been sprung **three times**, by three different
  functions, and every time the Windows leg was green while the Linux leg
  found it. `4390593` recorded a surviving
  mutation where `PathBuf::join` stood in for `bp_platform::paths::join` and
  only the Linux leg would ever have failed. The fifth session's device-name
  refusal did it again: the *judgement* was parameterised and the *split* was
  `Path::file_name()`, which splits by the platform the binary was compiled
  for — so on Linux `C:\Users\me\con.txt` is one file name containing no
  device, and the Windows rule passed by never being evaluated. Windows had
  been green for an hour. The third was `writable_path`, which took a platform
  and then called `std::path::absolute` -- and on the Linux leg
  `C:\docs\...` is a *relative* path, so the Windows rule was tested against
  a Linux working directory. The shape is always the same: the *judgement*
  takes the platform and some *path operation* underneath it does not.
  **`bp_platform::paths::file_name(platform, path)` exists now**, with `Path::file_name`'s exact semantics and the platform made
  explicit, so the correct thing is a drop-in rather than something you have
  to know to spell as `components(platform, p).last()`. `paths::join` was
  already the same fix for the writing direction.

  Not made into a gate check, and that was measured rather than assumed: a
  grep for host-dependent `std::path` calls in files that mention `Platform`
  returns five hits and all five are legitimate -- `str::strip_prefix`, and
  `path.parent()` on paths that really are the host's. A stage that cries
  wolf gets disabled wholesale, which `docs/governance/LOCAL_CI.md` already
  says about gates that are wrong. The enforcement is the primitive being
  there and obvious -- plus a test named for the trap in each function that
  has fallen into it, which fails on the Linux leg if anyone reintroduces it.

  **Ask of any parameterised function: which platform is each line of this
  answering about?** `is_absolute`, `components`, `file_name` and `join` all
  take one. `std::path` never does.

---

## Housekeeping

- ~~**`bp-ui`'s tests create a recovery directory in `%APPDATA%`.**~~
  **Fixed**, with the roaming item, since both owned `bp-config` and `bp-ui`
  together. `recovery_dir()` is `cfg(test)`-redirected to a unique temp path
  -- the fix the security history already took, after it wrote a live history
  into the developer's own profile once. A test pins it, and a second one
  pins that two states in one process do not share a journal, because
  parallel tests each writing checkpoints the others then read is how the
  shared-file version of this failed.

- ~~**`ui/app.slint` wants splitting.**~~ **Done** — seven files. What is left
  in it, and why each thing stayed, is in `docs/architecture/ARCHITECTURE.md`.
  The popups were the one piece deliberately not moved: a `PopupWindow` is
  positioned relative to its parent element, so moving them would change the
  coordinate parent, and nothing in the gate can see a menu opening in the
  wrong place.
- ~~**Five agent worktrees under `.claude/worktrees/`.**~~ **Pruned**, after
  each was checked file by file against what landed: the cross-crate test file
  is byte-identical to `9d716f5`, `fuzz/README.md` to `db9af48`, and the
  worktree carrying a draft `ADR-0022.md` held the *YAML* ADR, whose body is
  byte-identical to the `ADR-0023.md` it was renumbered into. Nothing unique
  was in any of them. Worth knowing for next time: **an abandoned worktree's
  file can differ from the main tree and still be safe**, because the branch
  moved on — the comparison that answers the question is against the commit
  the work landed in, not against `HEAD`. It also removes the sixfold copy of
  `fuzz/corpus/yaml/` that was breaking editor tooling.
- **All 59 commits live on `feat/phase-01-foundation-ui`.** `main` has none of
  them and 44 are unpushed, so a sixteen-phase product exists in exactly one
  place under a branch named for phase one.
- ~~**The panic discipline is real and unenforced.**~~ **Enforced now.**
  There was not one `unwrap()`, `panic!`, `todo!` or `unimplemented!` in
  production library code across all 24 crates before the lints were switched
  on, which is why switching them on cost no code changes.
  `[workspace.lints.clippy]` carries the four at `warn`, the gate's
  `-D warnings` makes them errors, and `clippy.toml` exempts test code
  because `unwrap()` in a fixture *is* the assertion.

  Three exemptions, each narrow and each with its reason on it: Slint's
  generated code moved behind a `mod generated` so its ~400 machine-written
  panics stop drowning the lint that guards the hand-written 209 tests'
  worth of shell around it; `bp-buffer`'s benchmark and `bp-naming`'s
  property-strategy helpers carry file-level allows, because clippy's
  `allow-unwrap-in-tests` recognises `#[test]` functions and `#[cfg(test)]`
  blocks and neither of those is either; and `tests/integration` does not
  inherit the lints at all, being test code end to end.

  `expect_used` (nine in library code, all with a message saying what was
  assumed), `unreachable` (one) and `indexing_slicing` (thirty-six) are
  **not** on. Each is a change to the code rather than to the policy and
  wants its own pass.
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
