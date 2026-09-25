# Next session plan

Rewritten 2026-09-25, at the end of the sixth build session.

State: 24 crates, 1,729 tests, green on Linux -- **the Windows leg was not
run this session**, see below. One is `#[ignore]`d on purpose because it
names a decision rather than a defect (D6); the two defects that shared its
company are fixed.

---

## What the sixth session did

Tier 0.5 of the queue, the housekeeping item that went with it, and a
request that arrived mid-session -- **make the cursor keys, Home, End,
Page Up, Page Down and Insert work** -- which turned out to hide two real
defects nobody had seen, because nothing had ever tested the default view
in a window.

| Commit | What |
| --- | --- |
| `fix(bp-notebook)` | A cell naming a language we cannot map stays `Raw`, with a warning, even when the notebook names a kernel. ADR-0025 updated |
| `fix(bp-files)` | A long path is spelled `\\?\` for the rename. **`std::fs` lengthens long paths itself; `tempfile::persist` calls `MoveFileExW` raw**, so only the atomic rename failed. `SaveError::PathTooLong` when it cannot be lengthened |
| `feat(bp-config)` | ADR-0029: recent list and security history to `DirKind::State`, recovery journal to `DirKind::Data`. Old locations are *adopted* by one rename; a failed rename keeps using the old location. `cargo test` no longer writes into the developer's config directory |
| `docs(bp-formats)` | `Profile::is_data` says what it answers |
| `feat(bp-ui)` | Page Up/Down, a view that follows the caret, Insert/overwrite in both views, the CUA clipboard keys, and **window tests** |

**The Windows leg has not run on any of this.** The session's container had
no PowerShell and no Windows; the Linux leg was run stage by stage by hand
(fmt, check --locked, clippy, test, launch, log hygiene, fuzz) and is green.
The long-path fix is Windows behaviour, exercised on Linux only through its
platform-parameterised half, and the window tests have never met Consolas.
**Run `./scripts/Invoke-LocalCI.ps1` on Windows first**, before anything
else.

## The window tests, and what they are for

`crates/bp-ui/src/window_tests.rs` builds the real `AppWindow` on
`MinimalSoftwareWindow` and dispatches real `WindowEvent::KeyPressed`
events. No display, no new dependency -- both are public Slint API. Slint's
platform is per thread, and so is each test. **This is the tool the
`--editor-view` checklist below has been missing**: most of its rows can now
be asked of a test rather than of a person. What it found first:

- **Page Up and Page Down did nothing in the default view.** `TextInput`
  pages by its `page-height`, which defaults to zero, and it refuses a page
  of zero. Set to the `Flickable`'s height now.
- **The arrows, End and Ctrl+End moved the caret out of sight.** A bare
  `Flickable` does not follow its content's caret, so past the bottom row
  the keys looked dead. `cursor-position-changed` scrolls it now, only as
  far as needed, and a no-wrap document has a viewport as wide as its
  longest line.

Both were seen to fail with the fix removed. `AppWindow` gained two `out`
properties, `editor-scroll-x` and `-y`, so a test can ask where the view is.

**Driving the whole binary on Xvfb works for `--editor-view` and not for
the default view.** With `libxkbcommon-x11-0` installed and `python-xlib`
sending XTEST events, the custom surface took clicks, every navigation key,
overwrite and Ctrl+S end to end, and the saved file was checked. The default
view's `TextInput` received nothing -- not even the window's own Ctrl+S --
which looks like the X input-method path `TextInput` enables, with no IM
server on a bare Xvfb. A harness limit, not a product defect; the headless
tests above are the better tool for that view anyway.

**That run found one defect it did not fix: `--editor-view` draws its caret
to the right of the text on Linux.** `EditorSurface` multiplies one measured
advance by the column, which holds only for a monospace font. It asks for
`"Consolas"`; Slint resolves a family name literally -- no fontconfig
aliasing, no generic `monospace` -- so Linux gets a proportional sans and the
caret drifts further right with every character. This is the "advance
measurement" row of the checklist below, answered for Linux. The fix is a
product choice (bundle a monospace font with `import`, or choose the family
per platform in Rust) and is in the queue as Tier 0.75.

## Insert, and what it had been doing

Under `--editor-view`, Insert had no meaning, so its key code -- U+F727, in
the private-use block Slint encodes named keys into -- fell through as a
character and **was typed into the document, invisibly**. So was every other
key the shell does not name (F1-F12, Menu). `bp_editor::keys::is_toolkit_key`
refuses the block now, as `TextInput` always did.

Overwrite is one rule in one place: `bp_editor::overwritten` -- one character
per character typed, never past the end of the line, nothing for a line
break or a key that is not text. `Editor::type_text` uses it; the `TextInput`
view asks through `overwrite_end`, selects what it will replace, and lets the
widget's own "typing replaces the selection" do the rest. That reads the
caret through `cursor-position_byte-offset`, which Slint marks internal -- the
only way to read it. An upgrade that removes it fails to compile rather than
misbehaving. The mode is the window's, shown as `OVR` in the status bar.

## The shape of the problem has changed

For three sessions the largest gap was that capabilities did not exist. It is
now the opposite. **Three crates are built, tested and unreachable** — the
large-file engine, notebooks and research — and the queue's Tier 0 is
nothing but wiring. Building another library is the easy, comfortable move
and it is the wrong one.

An audit at the end of the fifth session put a number on that: `bp-research`,
`bp-notebook` and `bp-storage` have **zero reverse dependencies anywhere in
the application**, and with `bp-buffer`'s large-file engine that is over ten
thousand lines and 237 unit tests defending code no user can reach.
`bp-notebook` is named by the fuzz harness; nothing in the shipped binary
names any of the three. `bp-audit` came off that list in the same session --
Security ▸ Security History reads it and five capabilities write into it.

The same audit found two things this file had no way to see, and they are
items 2 and 3 below. Both are cheap, and both make everything after them
worth trusting.

Four things are worth more than any new crate:

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

### 2. ~~Fix the defects the cross-crate tests found~~ — done, all but a decision

**Six of the seven defects are fixed; the seventh, D6, is a decision.** The sixth
session took the last two. The fifth took the one that destroyed documents: `bp_naming::sanitize` now takes the stem before the first dot, the
way Win32 does, so `con.txt`, `aux.log`, `NUL.dat` and `lpt1.bak` become
`con File.txt` and its friends rather than names that write to the device.
Its list gained `CONIN$` and `CONOUT$`, and a property test compares the two
crates' verdicts stem by stem, in both directions, so the two lists cannot
drift apart quietly. `bp-files` closes the other route into the same failure
-- the name a user *types*, which sanitising never sees -- by refusing to
write to a device at all.

The two that sat behind it -- `bp-notebook` running code it could not name
through the kernel's interpreter, and `bp-files` never applying `\\?\` --
are fixed and their tests run. The line-definition disagreement is D6 and
wants an answer, not a patch.

**Write more cross-crate tests as you go.** Eleven files of them have now
found seven defects that the unit tests had not. That ratio is still the most
useful thing this project has learned.

### 3. ~~Make the record match the product again~~ — done, and worth knowing why

**Done at the end of the fifth session, in the same pass that found it.**
This repository's method is that the written record says what is real — that
is what ROADMAP's three-word status vocabulary is for. An audit found four
entries in `DECISIONS.md`'s open-items list describing defects that were
demonstrably fixed, a README roster naming two crates that have menu rows
while omitting two that have none, and phase 18 still marked *Built,
unwired* a session after Set as Default Editor shipped. All of it is
corrected. What is worth carrying forward is the list itself, because every
one of these went stale the same way — the fix landed and the record did
not:

| Claim | Where it is false |
| --- | --- |
| JSONL sniff calls a pretty-printed array "JSON Lines" | `bp_formats::looks_like_json_lines` requires `closes_what_it_opens(first)` |
| The encoder is `pub(crate)` in the shell | `crates/bp-files/src/encode.rs` |
| YAML has no way in | `bp-ui/src/state.rs:593` calls `bp_data::yaml_validate` |
| `bp-audit`'s sealed destination has no implementor | `bp-audit/src/passphrase.rs:185` |
| README: five crates built and unreachable | Names `bp-integrity` and `bp-platform`, which have rows; omits `bp-audit` and `bp-storage`, which do not |
| ROADMAP phase 18: **Built, unwired** | File ▸ Set as Default Editor shipped in the fifth session |

A reader who checks one claim and finds it stale has no way to know which of
the rest to trust, and the method depends entirely on trusting them. **The
roster is now counted rather than remembered** — reverse dependencies read
out of every `Cargo.toml` and checked against what the shell's source
actually names — which is the only version of that claim that stays true
without somebody maintaining it.

### 4. ~~Put `fuzz/` inside the gate~~ — done, and it costs five minutes

`Invoke-LocalCI.ps1` swept `spikes/` for standalone workspaces. There is no
`spikes/` directory in this tree, and there *was* a `fuzz/` one holding five
hostile-input harnesses that nothing formatted, linted or ran — while phase 19
was marked *Started* on the strength of them.

Two stages now run them, on both legs, and they were green from the first run:
42 tests, one `#[ignore]`d for cost with the reason on it. **The full gate is
about five minutes longer** — two and a half per leg — which is the honest
price of the assurance and is visible in the stage table rather than hidden.
`-Quick` skips it, so `pre-commit` is unchanged and `pre-push` carries it.

Two things worth keeping from doing it:

- **The stage was verified to fail**, by breaking the formatting of
  `fuzz/tests/yaml.rs` and watching it go red while the workspace `fmt` stage
  stayed green. That contrast *is* the blindness, reproduced. A gate stage
  nobody has seen fail is a stage nobody knows the shape of.
- **Where one harness becomes too expensive to gate on**, the answer is
  already in the tree: `fuzz/tests/envelope.rs` `#[ignore]`s its KDF-cost
  test with the reason on it. Do that, not a switch that turns the stage off.

---

## The `--editor-view` checklist, partly answered by a machine

```powershell
cargo run --release -- --editor-view
```

A person has still not done this. The sixth session drove the binary on a
virtual X display with synthetic input, on Linux, which answers some rows
and cannot answer the rest -- a synthetic click lands exactly where it was
sent, and nobody was looking at the result as a user would.

| Check | What would be wrong | Sixth session, Xvfb, Linux |
| --- | --- | --- |
| Type a character | Keys not reaching the `FocusScope` at all | **Works** |
| Click mid-line | Caret lands left or right of the pointer — the measured advance is off | **Wrong on Linux**: no Consolas, proportional fallback. See above |
| Click at the end of a long line | Advance error accumulating across columns | **Wrong on Linux**, same cause, and it does accumulate |
| Arrow down through a short line and back up | The goal column not surviving | Page Down kept the column; the round trip itself not run |
| Drag to select | Selection rectangles misplaced, or drag not extending | Not run |
| Scroll the wheel | Wrong direction, or snapping back to the caret | Not run |
| **Ctrl+S** | The surface swallowing chords it should reject — this would stop saving | **Works** -- the file on disk had every edit |
| Ctrl+D, Alt+Up, Alt+Down | Duplicate and move line, shipped this session, never run | Not run |
| Watch the status bar | Ln/Col is the payoff; it should track the caret | **Works**, and shows `OVR` |
| Open a tab-indented file | Tab stops wrong, caret off by columns | Not run |
| Resize the window | Row count not following, blank strip at the bottom | Not run |
| Home, End, Page Up/Down, Ctrl+Home/End, Insert | Keys unnamed, or landing on the wrong line | **Works** -- each checked by where a typed marker landed in the saved file |

**The advance measurement was one of the two most likely to be subtly wrong,
and on Linux it is -- not subtly.** On Windows it still wants a look, and
so does the wheel direction.

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
- **One definition of a line.** `bp-search` counts
  `\n`; `bp-buffer`'s rope
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
- **Code taking a `Platform` parameter must not use `std::path` to split
  anything.** This has now been sprung twice. `4390593` recorded a surviving
  mutation where `PathBuf::join` stood in for `bp_platform::paths::join` and
  only the Linux leg would ever have failed. The fifth session's device-name
  refusal did it again: the *judgement* was parameterised and the *split* was
  `Path::file_name()`, which splits by the platform the binary was compiled
  for — so on Linux `C:\Users\me\con.txt` is one file name containing no
  device, and the Windows rule passed by never being evaluated. Windows had
  been green for an hour. **`bp_platform::paths::file_name(platform, path)`
  exists now**, with `Path::file_name`'s exact semantics and the platform made
  explicit, so the correct thing is a drop-in rather than something you have
  to know to spell as `components(platform, p).last()`. `paths::join` was
  already the same fix for the writing direction.

  Not made into a gate check, and that was measured rather than assumed: a
  grep for host-dependent `std::path` calls in files that mention `Platform`
  returns five hits and all five are legitimate -- `str::strip_prefix`, and
  `path.parent()` on paths that really are the host's. A stage that cries
  wolf gets disabled wholesale, which `docs/governance/LOCAL_CI.md` already
  says about gates that are wrong. The enforcement is the primitive being
  there and obvious.

---

## Housekeeping

- ~~**`bp-ui`'s tests create `%APPDATA%\bachelorpad\recovery`.**~~ **Done**
  with ADR-0029, together with the `recent.toml` item it was paired with.
  Worse than recorded: the tests were also replacing the developer's own
  recent-files list with fixture paths. What was recorded: Empty --
  every checkpoint they trigger is refused and refusal deletes -- so it is a
  `mkdir` and nothing more. It is still a test reaching outside a temp
  directory, and it is the same shape as the defect the security history hit
  and now guards against: `AppState::new()` builds its `Journal` from the
  real `recovery_dir()`. The fix is the one the history took, a field rather
  than a global, and it wants doing with the item below about `recent.toml`
  roaming, since both own `bp-config` and `bp-ui` together.


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
